//! Common utilities shared amongst the project.

use std::path::Path;

use bytes::{Bytes, BytesMut};
use color_eyre::Report;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

pub async fn read_from_file<P: AsRef<Path>>(path: P) -> Result<Bytes, Report> {
    let mut file = File::open(path.as_ref()).await?;
    let buffer = BytesMut::new();

    file.read_to_end(&mut buffer.to_vec()).await?;

    let decoded = BasicCodec::decode(&buffer[..buffer.len()]);
    println!("Read from file:\n{decoded}");

    file.flush().await?;

    Ok(buffer.freeze())
}

pub async fn write_to_file<P: AsRef<Path>>(path: P, buffer: &[u8]) -> Result<(), Report> {
    let mut file = File::create(path.as_ref()).await?;

    let message = BasicCodec::decode(&buffer[..buffer.len()]);
    println!("writing message to file:\n{message}");

    file.write_all(buffer).await?;

    println!(
        "Wrote {} bytes to file: {}",
        buffer.len(),
        path.as_ref().display()
    );

    file.flush().await?;

    Ok(())
}

/// Simple codec used to handle UTF-8 string and byte slices.
///
/// NOTE: This codec should not be used in a production setting.
pub struct BasicCodec;

impl BasicCodec {
    /// Encode a UTF-8 string slice, returning an instance of `Bytes`.
    pub fn encode(source: &str) -> Bytes {
        Bytes::copy_from_slice(source.as_bytes())
    }

    /// Decode a UTF-8 byte slice lossily, returning a `String`.
    pub fn decode(source: &[u8]) -> String {
        String::from_utf8_lossy(source).to_string()
    }
}
