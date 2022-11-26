use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    time::Duration,
};

use color_eyre::{eyre::eyre, Report};
use futures::prelude::stream::StreamExt;
use libp2p::{
    self,
    core::{muxing::StreamMuxerBox, transport, upgrade},
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, IdentTopic,
        MessageAuthenticity, MessageId, ValidationMode,
    },
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, Multiaddr, PeerId, Swarm, Transport,
};
use tokio::io::{self, AsyncBufReadExt};

use self::signals::spawn_signal_handler;

pub mod signals;

// FIXME: Refactor `Controller` to avoid circular depenencies or invalid/initialized state.

/// NetworkBehaviour for multicast DNS using the Tokio runtime. Peers on the
/// local network are automatically discovered and added to the topology.
pub type MdnsBehaviour = mdns::Behaviour<mdns::tokio::Tokio>;

/// Network behaviour that combines Gossipsub and mDNS.
///
/// Floodsub is used for publish / subscribe and mDNS for local peer discovery.
///
/// The derive generates a delegating `NetworkBehaviour` implementation.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ControllerEvent")]
pub struct ControllerBehaviour {
    gossipsub: Gossipsub,
    mdns: MdnsBehaviour,
}

impl ControllerBehaviour {
    fn new(node: &Node, mdns: MdnsBehaviour) -> Result<ControllerBehaviour, Report> {
        // The content of each message is hashed, yielding the message ID.
        let message_id_fn = |message: &GossipsubMessage| {
            let mut hasher = DefaultHasher::new();
            message.data.hash(&mut hasher);
            MessageId::from(hasher.finish().to_string())
        };

        // Enable message signing. Use owner of key for author and random sequence number.
        let privacy = MessageAuthenticity::Signed(node.keypair().clone());
        let config = GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_millis(1053)) // Increase to aid with debugging by decreasing noise
            .validation_mode(ValidationMode::Strict) // Set message validation (default: Strict)
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
            .build()
            .map_err(|err| eyre!(err))?;

        // Build a gossipsub network behaviour from the privacy and config options.
        let gossipsub = Gossipsub::new(privacy, config).map_err(|err| eyre!(err))?;
        Ok(ControllerBehaviour { gossipsub, mdns })
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ControllerEvent {
    Gossipsub(GossipsubEvent),
    Mdns(mdns::Event),
}

impl From<mdns::Event> for ControllerEvent {
    fn from(value: mdns::Event) -> Self {
        Self::Mdns(value)
    }
}

impl From<GossipsubEvent> for ControllerEvent {
    fn from(value: GossipsubEvent) -> Self {
        Self::Gossipsub(value)
    }
}

/// Simple wrapper around a `Swarm` instance, with a pre-defined
/// `NetworkBehaviour` implementation. At this time, a [Controller] is used
///  almost exactly the same as a `Swarm` instance.
pub struct Controller {
    swarm: Swarm<ControllerBehaviour>,
    keypair: identity::Keypair,
}

impl Controller {
    /// Construct a new [Controller] instance.
    pub fn new(
        transport: transport::Boxed<(PeerId, StreamMuxerBox)>,
        behaviour: ControllerBehaviour,
        node: Node,
    ) -> Controller {
        let swarm = Swarm::with_tokio_executor(transport, behaviour, node.peer_id());
        tracing::info!("initializing controller node");
        Controller {
            swarm,
            keypair: node.keypair().clone(),
        }
    }

    /// Returns a refernce to the controller's Swarm instance.
    pub fn swarm(&self) -> &Swarm<ControllerBehaviour> {
        &self.swarm
    }

    pub fn public_key(&self) -> identity::PublicKey {
        self.keypair.public()
    }

    /// Returns a reference to the provided [`NetworkBehaviour`].
    pub fn behaviour(&self) -> &ControllerBehaviour {
        self.swarm.behaviour()
    }

    /// Returns a mutable reference to the provided [`NetworkBehaviour`].
    pub fn behaviour_mut(&mut self) -> &mut ControllerBehaviour {
        self.swarm.behaviour_mut()
    }

    /// Dial a known or unknown peer.
    pub fn dial_peer(&mut self, peer_addr: &str) -> Result<(), Report> {
        let multiaddr = peer_addr.parse::<Multiaddr>()?;
        self.swarm.dial(multiaddr)?;
        tracing::info!("Dialed peer {peer_addr}");
        Ok(())
    }

    /// Start listening on the given address.
    ///
    /// Returns an error if the address is not supported.
    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<(), Report> {
        Ok(self.swarm.listen_on(addr).map(|_| {})?)
    }

    /// Start the main event loop, handling peers and swarm events.
    pub async fn run(&mut self, topic: IdentTopic) -> Result<(), Report> {
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        spawn_signal_handler().await;

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(line)) = line {
                        match self.swarm.behaviour_mut().gossipsub.publish(topic.clone(), line.as_bytes()) {
                            Ok(message_id) => tracing::info!("Published message with ID {message_id}"),
                            Err(err) => tracing::error!("Failed to publish message; error = {err:?}"),
                        }
                    } else {
                        return Err(eyre!("Stdin handle closed unexpectedly"))
                    }
                }
                event = self.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::info!("Listening on {address:?}");
                    }
                    SwarmEvent::Behaviour(ControllerEvent::Gossipsub(GossipsubEvent::Message { propagation_source, message_id, message })) => {
                        let peer_id = propagation_source;
                        let message_data = String::from_utf8_lossy(&message.data);
                        tracing::info!("Message received:\n[{peer_id}] said: \"{message_data}\" -  (message ID: {message_id})");
                    }
                    SwarmEvent::Behaviour(ControllerEvent::Mdns(event)) => match event {
                        mdns::Event::Discovered(list) => {
                            for (peer, multiaddr) in list {
                                self
                                    .behaviour_mut()
                                    .gossipsub
                                    .add_explicit_peer(&peer);
                                tracing::info!("mDNS discovered new peer: {peer} at {multiaddr}");
                            }
                        }
                        mdns::Event::Expired(list) => {
                            for (peer, multiaddr) in list {
                                if !self.behaviour().mdns.has_node(&peer) {
                                    self.behaviour_mut().gossipsub.remove_explicit_peer(&peer);
                                    tracing::info!("mDNS peer discovery expired for: {peer} at {multiaddr}");
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl Deref for Controller {
    type Target = Swarm<ControllerBehaviour>;

    fn deref(&self) -> &Self::Target {
        &self.swarm
    }
}

impl DerefMut for Controller {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.swarm
    }
}

/// A [Node] is a member of the peer-to-peer network.
pub struct Node(identity::Keypair);

impl Node {
    /// Create a new instance of a [Node], generating its cryptographic keypair.
    pub fn init() -> Self {
        Self::default()
    }

    /// Returns the Peer ID of the [`Controller`] node, derived from its keypair.
    pub fn keypair(&self) -> &identity::Keypair {
        &self.0
    }

    /// Return the peer's ID.
    pub fn peer_id(&self) -> PeerId {
        self.0.public().to_peer_id()
    }
}

impl Default for Node {
    fn default() -> Self {
        Self(identity::Keypair::generate_ed25519())
    }
}

/// Hard-coded string representing the topic to be used for pubsub.
pub const PUBSUB_TOPIC: &str = "coil-05FjJDr9Y8z";

/// Starts a [Swarm] to manage peers and events. The swarm listens by default,
/// but will dial out to a peer if a multi-address is passed as a CLI argument.
///
/// [Swarm]: https://docs.rs/libp2p/latest/libp2p/struct.Swarm.html
pub async fn bootstrap() -> Result<(), Report> {
    let node = Node::init();

    // TODO: Learn more about the transport setup process, then refactor if needed.
    let transport_config = tcp::Config::default().nodelay(true);
    let transport = tcp::tokio::Transport::new(transport_config)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseAuthenticated::xx(node.keypair())?)
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let pubsub_topic = IdentTopic::new(PUBSUB_TOPIC);

    let mdns_behaviour = mdns::tokio::Behaviour::new(mdns::Config::default())?;
    let behaviour = ControllerBehaviour::new(&node, mdns_behaviour)?;

    let mut controller = Controller::new(transport, behaviour, node);
    match controller
        .swarm
        .behaviour_mut()
        .gossipsub
        .subscribe(&pubsub_topic)
    {
        Ok(_true) => tracing::info!("Subscribed to new topic: {pubsub_topic}"),
        Err(err) => tracing::error!("Subscription to topic {pubsub_topic} failed: {err:?}"),
    }

    // Reach out to another node if specified
    if let Some(ref to_dial) = std::env::args().nth(1) {
        controller.dial_peer(to_dial)?;
    }

    let listen_addr = "/ip4/0.0.0.0/tcp/15550".parse::<Multiaddr>()?;
    controller.listen_on(listen_addr)?;
    controller.run(pubsub_topic.clone()).await
}
