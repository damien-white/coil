//! OS-specific handlers for receiving and responding to signals.

pub async fn spawn_signal_handler() {
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal.");
        tracing::debug!("Received shutdown signal. Exiting gracefully...");
        std::process::exit(0);
    });
}
