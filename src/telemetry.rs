use color_eyre::Report;
use tracing::Level;
use tracing_subscriber::{filter::Directive, fmt::format::FmtSpan, EnvFilter};

// TODO: Remove the tracing "init" logic, replacing with library-appropriate code.

/// Attaches a tracing subscriber to the application,
pub fn attach_tracing_logger() -> Result<(), Report> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "tokio=debug");
    }

    let default_level: Directive = Level::INFO.into();
    let filtering_directive = "coil=debug".parse().unwrap_or(default_level);

    let tracing_filter = EnvFilter::from_default_env().add_directive(filtering_directive);
    tracing_subscriber::fmt()
        .with_env_filter(tracing_filter)
        .with_span_events(FmtSpan::FULL)
        .init();

    Ok(())
}
