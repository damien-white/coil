use std::ops::{Deref, DerefMut};

use color_eyre::{eyre::eyre, Report};
use futures::prelude::stream::StreamExt;
use libp2p::{
    self,
    core::{muxing::StreamMuxerBox, transport, upgrade},
    floodsub::{self, Floodsub, FloodsubEvent, Topic},
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, Multiaddr, PeerId, Swarm, Transport,
};
use tokio::io::{self, AsyncBufReadExt};

// FIXME: Refactor `Controller` impl due to potential circular depenencies and invalid/initialized state.

/// NetworkBehaviour for multicast DNS using the Tokio runtime. Peers on the
/// local network are automatically discovered and added to the topology.
pub type MdnsBehaviour = mdns::Behaviour<mdns::tokio::Tokio>;

/// Network behaviour that combines floodsub and mDNS.
///
/// Floodsub is used for publish / subscribe and mDNS for local peer discovery.
///
/// The derive generates a delegating `NetworkBehaviour` implementation.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "CoilOutEvent")]
pub struct ControllerBehaviour {
    protocol: Floodsub,
    mdns: MdnsBehaviour,
}
impl ControllerBehaviour {
    fn new(node: &Node, mdns: MdnsBehaviour) -> ControllerBehaviour {
        ControllerBehaviour {
            protocol: Floodsub::new(node.peer_id()),
            mdns,
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum CoilOutEvent {
    Floodsub(FloodsubEvent),
    Mdns(mdns::Event),
}

impl From<mdns::Event> for CoilOutEvent {
    fn from(value: mdns::Event) -> Self {
        Self::Mdns(value)
    }
}

impl From<FloodsubEvent> for CoilOutEvent {
    fn from(value: FloodsubEvent) -> Self {
        Self::Floodsub(value)
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

    // /// Returns the Peer ID of the [`Controller`] node, derived from its keypair.
    // fn peer_id(&self) -> PeerId {
    //     self.keypair.public().to_peer_id()
    // }

    // /// Subscribes to a `Topic` and adds it to the controller's internal state.
    // ///
    // /// Returns `Ok` if the subscription works, or an `Err` if the controller is
    // /// already subscribed to the topic.
    // pub fn subscribe(&mut self, topic: &str) -> Result<(), Report> {
    //     if self
    //         .swarm
    //         .behaviour_mut()
    //         .protocol
    //         .subscribe(Topic::new(topic))
    //     {
    //         Ok(())
    //     } else {
    //         Err(eyre!("controller is already subscribed to {topic}"))
    //     }
    // }

    // /// Publishes a message to the network, if we're subscribed to the topic only.
    // pub fn publish(&mut self, topic: &str, message: &[u8]) {
    //     self.swarm
    //         .behaviour_mut()
    //         .protocol
    //         .publish(Topic::new(topic), message)
    // }

    /// Start the main event loop, handling peers and swarm events.
    pub async fn run(&mut self, pubsub_topic: Topic) -> Result<(), Report> {
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        tokio::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for shutdown signal.");
            tracing::debug!("Received shutdown signal. Exiting gracefully...");
            std::process::exit(0);
        });

        loop {
            tokio::select! {
                line = stdin.next_line() => {
                    if let Ok(Some(line)) = line {
                        self.swarm.behaviour_mut().protocol.publish(pubsub_topic.clone(), line.as_bytes());
                    } else {
                        return Err(eyre!("Stdin handle closed unexpectedly"))
                    }
                }
                event = self.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("Listening on {address:?}");
                        }
                        SwarmEvent::Behaviour(CoilOutEvent::Floodsub(FloodsubEvent::Message(message))) => {
                            let message_data = String::from_utf8_lossy(&message.data);
                            let source_peer = message.source;
                            tracing::info!("Message received: '{message_data}' from {source_peer}");
                        }
                        SwarmEvent::Behaviour(CoilOutEvent::Mdns(event)) => match event {
                            mdns::Event::Discovered(list) => {
                                for (peer, addr) in list {
                                    self
                                        .behaviour_mut()
                                        .protocol
                                        .add_node_to_partial_view(peer);
                                    tracing::info!("Added peer to bucket: {peer} at {addr}");
                                }
                            }
                            mdns::Event::Expired(list) => {
                                for (peer, addr) in list {
                                    if !self.behaviour().mdns.has_node(&peer) {
                                        self.behaviour_mut().protocol.remove_node_from_partial_view(&peer);
                                        tracing::info!("Removed peer from bucket: {peer} at {addr}");
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

    let pubsub_topic = floodsub::Topic::new(PUBSUB_TOPIC);

    let mdns_behaviour = mdns::tokio::Behaviour::new(mdns::Config::default())?;
    let mut behaviour = ControllerBehaviour::new(&node, mdns_behaviour);
    behaviour.protocol.subscribe(pubsub_topic.clone());

    let mut controller = Controller::new(transport, behaviour, node);

    // Reach out to another node if specified
    if let Some(ref to_dial) = std::env::args().nth(1) {
        controller.dial_peer(to_dial)?;
    }

    let listen_addr = "/ip4/0.0.0.0/tcp/15550".parse::<Multiaddr>()?;
    controller.listen_on(listen_addr)?;
    controller.run(pubsub_topic.clone()).await
}
