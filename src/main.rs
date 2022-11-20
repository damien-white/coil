use color_eyre::Report;

#[tokio::main]
async fn main() -> Result<(), Report> {
    coil::telemetry::attach_tracing_logger()
        .expect("received a malformed or invalid tracing directive");
    coil::bootstrap().await.expect("bootstrap process failed");

    Ok(())
}
