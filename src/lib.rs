use core::time::Duration;

pub mod cli;
pub mod utils;

/// Establish a connection to the server using the configured transport.
///
/// Note: This should be run on a client machine to connect to a server.
pub async fn connect_client() {
    // Wait for the server to be ready
    tokio::time::sleep(Duration::from_millis(1500)).await;
}
