use color_eyre::{eyre::eyre, Report};
use futures::prelude::stream::StreamExt;
use libp2p::{
    self,
    core::upgrade,
    floodsub::{self, Floodsub, FloodsubEvent},
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, Multiaddr, PeerId, Swarm, Transport,
};
use tokio::io::{self, AsyncBufReadExt};

/// Network behaviour that combines floodsub and mDNS.
///
/// Floodsub is used for publish / subscribe and mDNS for local peer discovery.
///
/// The derive generates a delegating `NetworkBehaviour` implementation.
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "CoilOutEvent")]
pub struct CoilNetworkBehaviour {
    protocol: Floodsub,
    mdns: mdns::tokio::Behaviour,
}
impl CoilNetworkBehaviour {
    fn new(peer_id: PeerId, mdns: mdns::Behaviour<mdns::tokio::Tokio>) -> CoilNetworkBehaviour {
        CoilNetworkBehaviour {
            protocol: Floodsub::new(peer_id),
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

pub type CoilSwarm = Swarm<CoilNetworkBehaviour>;

/// Starts a [Swarm] to manage peers and events. The swarm listens by default,
/// but will dial out to a peer if a multi-address is passed as a CLI argument.
///
/// [Swarm]: https://docs.rs/libp2p/latest/libp2p/struct.Swarm.html
pub async fn bootstrap() -> Result<(), Report> {
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    tracing::info!(%peer_id, "initializing network node");

    // Create a tokio-based TCP transport using noise for authenticated
    // encryption, and mplex for multiplexing of substreams on a TCP stream
    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseAuthenticated::xx(&id_keys)?)
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Create a Floodsub topic
    let floodsub_topic = floodsub::Topic::new("coil");

    let mdns_behaviour = mdns::tokio::Behaviour::new(mdns::Config::default())?;
    let mut behaviour = CoilNetworkBehaviour::new(peer_id, mdns_behaviour);
    behaviour.protocol.subscribe(floodsub_topic.clone());

    // create the swarm using the transport, behaviour and local peer id
    let mut swarm = Swarm::with_tokio_executor(transport, behaviour, peer_id);

    // Reach out to another node if specified
    if let Some(ref to_dial) = std::env::args().nth(1) {
        dial_peer_node(to_dial, &mut swarm)?;
    }

    // Read full lines from stdin
    // let mut stdin_rx = spawn_stdin_channel()?;
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces using a randomly-assigned port
    let listen_addr = "/ip4/0.0.0.0/tcp/0".parse::<Multiaddr>()?;
    let _ = swarm.listen_on(listen_addr)?;
    // tracing::debug!("swarm {swarm_id:?} is listening on {listen_addr}");

    // start the main loop, allowing the swarm to do its work
    loop {
        tokio::select! {
            line = stdin.next_line() => {
                if let Ok(Some(line)) = line {
                    swarm.behaviour_mut().protocol.publish(floodsub_topic.clone(), line.as_bytes());
                } else {
                    return Err(eyre!("Stdin handle closed unexpectedly"))
                }
            }
            event = swarm.select_next_some() => {
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
                                swarm
                                    .behaviour_mut()
                                    .protocol
                                    .add_node_to_partial_view(peer);
                                tracing::info!("Added peer to bucket: {peer} at {addr}");
                            }
                        }
                        mdns::Event::Expired(list) => {
                            for (peer, addr) in list {
                                if !swarm.behaviour().mdns.has_node(&peer) {
                                    swarm.behaviour_mut().protocol.remove_node_from_partial_view(&peer);
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

fn dial_peer_node(to_dial: &str, swarm: &mut Swarm<CoilNetworkBehaviour>) -> Result<(), Report> {
    let addr = to_dial.parse::<Multiaddr>()?;
    swarm.dial(addr)?;
    tracing::info!("Dialed peer {to_dial}");
    Ok(())
}
