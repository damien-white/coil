use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use bytes::{Bytes, BytesMut};
use color_eyre::{eyre::eyre, Report};
use futures::SinkExt;
use std::sync::Arc;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{debug, error, info};

pub fn default_socket_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 19880)
}

#[derive(Debug)]
pub enum CommandSwitch {
    /// open a connection to a listening peer
    Dial(String),
    /// listen for connections from other peers
    Listen(String),
}

impl CommandSwitch {
    pub async fn parse_from_args<A>(args: A) -> Result<(), Report>
    where
        A: IntoIterator<Item = String>,
    {
        let args = args.into_iter().skip(1).collect::<Vec<String>>();
        if args.len() != 2 {
            return Err(eyre!("missing or invalid number of arguments"));
        }

        let mode = &args[0];
        let addr = &args[1];
        match mode.to_lowercase().as_str() {
            "dial" => {
                let mut dialier = Dialer::new(addr).await?;
                let mut input = String::new();
                let len = std::io::stdin().read_line(&mut input)?;
                let message = &input[..len];
                info!("attempting to send message: {message}");
                let response = dialier.send_message(message.as_bytes()).await?;
                let response = String::from_utf8_lossy(&response);
                info!("listener responded with {response}");
            }
            "listen" => {
                Listener::new(addr).await?.start().await?;
            }
            unsupported => return Err(eyre!("invalid or unsupported mode: {unsupported}")),
        };

        Ok(())
    }
}

pub fn valid_socket_addr(source: &str) -> SocketAddr {
    source.parse().unwrap_or_else(|_| default_socket_addr())
}

pub struct Dialer {
    connection: TcpStream,
}

impl Dialer {
    pub async fn new(address: &str) -> Result<Self, Report> {
        let peer_addr = valid_socket_addr(address);
        let connection = TcpStream::connect(peer_addr).await?;
        Ok(Self { connection })
    }

    pub async fn send_message(&mut self, message: &[u8]) -> Result<Bytes, Report> {
        self.connection.write_all(message).await?;

        let resp = BytesMut::new();
        let n = self.connection.read_to_end(&mut resp.to_vec()).await?;
        info!(
            "response received from server: {}",
            String::from_utf8_lossy(&resp[..n])
        );
        if n == 0 || resp.is_empty() {
            error!("sent message to listener, but got back an empty message");
        }

        println!("message was sent!");
        Ok(resp.freeze())
    }
}

/// Sender half of the messaging channel.
pub type Transmitter<T> = mpsc::Sender<T>;
// Receiver half of the messaging channel.
pub type Receiver<T> = mpsc::Receiver<T>;

/// Data that is shared between all connected peers.
pub struct SharedState {
    /// Peers stored in a map, with their socket address as the key.
    peers: HashMap<SocketAddr, Transmitter<String>>,
}

impl SharedState {
    /// Create an empty instance of [SharedState].
    fn new() -> SharedState {
        SharedState {
            peers: HashMap::new(),
        }
    }

    /// Send a `LineCodec` encoded message to all connected peers, exlcluding
    /// the sender.
    pub async fn broadcast<S: ToString>(&mut self, sender: SocketAddr, message: S) {
        for (addr, tx) in self.peers.iter_mut() {
            if *addr != sender {
                _ = tx.send(message.to_string())
            }
        }
    }
}

pub struct Listener {
    listener: TcpListener,
    state: Arc<Mutex<SharedState>>,
}

impl Listener {
    /// Create a new [Listener] instance, with an empty [SharedState].
    pub async fn new(bind_addr: &str) -> Result<Listener, Report> {
        let bind_addr = valid_socket_addr(bind_addr);
        let listener = TcpListener::bind(bind_addr).await?;
        Ok(Self {
            listener,
            state: Arc::new(Mutex::new(SharedState::new())),
        })
    }

    /// Start the server, allowing clients to connect.
    pub async fn start(&self) -> Result<(), Report> {
        info!("listener started on {}", self.listener.local_addr()?);

        loop {
            let (stream, addr) = self.listener.accept().await?;

            let state = Arc::clone(&self.state);

            // spawn a handler for each client
            tokio::spawn(async move {
                debug!("accepted connection from {addr}");
                if let Err(err) = handle_connection(state, stream, addr).await {
                    error!("unrecoverable error: {err:?}");
                }
            });
        }
    }
}

/// State for each connected client, or peer.
pub struct Peer {
    /// Socket wrapped with a codec.
    lines: Framed<TcpStream, LinesCodec>,
    receiver: Receiver<String>,
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(
        state: Arc<Mutex<SharedState>>,
        lines: Framed<TcpStream, LinesCodec>,
    ) -> io::Result<Peer> {
        // Get the client socket address
        let addr = lines.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (transmitter, receiver) = mpsc::channel(256);

        // Add an entry for this `Peer` in the shared state map.
        state.lock().await.peers.insert(addr, transmitter);

        Ok(Peer { lines, receiver })
    }
}

const BUF_MAX_LEN: usize = 1024;

/// Handle an individual connection.
async fn handle_connection(
    state: Arc<Mutex<SharedState>>,
    stream: TcpStream,
    address: SocketAddr,
) -> Result<(), Report> {
    let message = vec![0; 1024];

    let codec = LinesCodec::new_with_max_length(BUF_MAX_LEN);

    let mut lines = Framed::new(stream, codec);

    lines.send("peer id:").await?;

    let identity = match lines.next().await {
        Some(Ok(line)) => line,
        // failed to get a line, so return early
        _ => {
            error!("failed to get identity from {address}");
            return Ok(());
        }
    };

    let mut peer = Peer::new(state.clone(), lines).await?;

    // A client has connected, let's let everyone know.
    {
        let mut state = state.lock().await;
        let msg = format!("Peer {} joined the swarm", identity);
        tracing::info!("{}", msg);
        state.broadcast(address, &msg).await;
    }

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer. Send it to the current user.
            Some(msg) = peer.receiver.recv() => {
                peer.lines.send(&msg).await?;
            }
            result = peer.lines.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    let msg = format!("{}: {}", identity, msg);

                    state.broadcast(address, &msg).await;
                }
                // An error occurred.
                Some(Err(err)) => {
                    tracing::error!(
                        "an error occurred while processing messages for {identity}; error = {err:?}",
                    );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // If this section is reached it means that the client was disconnected!
    // Let's let everyone still connected know about it.
    {
        let mut state = state.lock().await;
        state.peers.remove(&address);

        let msg = format!("{} disconnected from the swarm", identity);
        tracing::info!("{}", msg);
        state.broadcast(address, &msg).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_can_be_parsed() {
        let bind_addr = "127.0.0.1:0";
        let socket = valid_socket_addr(bind_addr);
        assert!(socket.to_string().starts_with("127.0.0.1:"));
    }
}
