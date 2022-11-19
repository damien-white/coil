use color_eyre::Report;

use coil::cli::CommandSwitch;
use tracing::Level;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Report> {
    // TODO: Refactor tracing initialization
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "tokio=debug");
    }

    // Configure `tracing` subscriber that logs traces emitted by the server.
    let filtering_directive = "coil=debug".parse().unwrap_or_else(|err| {
        eprintln!("using INFO level due to invalid filter directive: {err:?}");
        Level::INFO.into()
    });
    let tracing_filter = EnvFilter::from_default_env().add_directive(filtering_directive);
    tracing_subscriber::fmt()
        .with_env_filter(tracing_filter)
        .with_span_events(FmtSpan::FULL)
        .init();

    // Parse the node mode (dialer/listener) and address arguments from user
    let args = std::env::args();
    CommandSwitch::parse_from_args(args).await?;

    Ok(())
}
