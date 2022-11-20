//! Common utilities and convenience methods.
use std::path::Path;

use color_eyre::Report;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

/// Read from a file and return its contents.
pub async fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, Report> {
    let mut file = File::open(path.as_ref()).await?;
    let buffer = Vec::new();

    file.read_to_end(&mut buffer.to_vec()).await?;

    let decoded = String::from_utf8_lossy(&buffer[..buffer.len()]);
    tracing::debug!("Read from file:\n{decoded}");

    file.flush().await?;

    Ok(buffer)
}

/// Write the contents of a buffer to a given file path.
pub async fn write_to_file<P: AsRef<Path>>(path: P, buffer: &[u8]) -> Result<(), Report> {
    let mut file = File::create(path.as_ref()).await?;
    file.write_all(buffer).await?;
    file.flush().await?;

    tracing::debug!(
        "Wrote {} bytes to file: {}",
        buffer.len(),
        path.as_ref().display()
    );

    Ok(())
}
