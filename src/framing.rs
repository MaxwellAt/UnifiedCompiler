use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Writes a length-prefixed bincode-serialized message to an async stream.
pub async fn send_frame<W, T>(writer: &mut W, message: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let encoded = bincode::serialize(message)
        .context("Failed to serialize message with bincode")?;
    let len = encoded.len() as u32;

    writer.write_u32(len).await.context("Failed to write frame length")?;
    writer.write_all(&encoded).await.context("Failed to write frame payload")?;
    writer.flush().await.context("Failed to flush frame")?;
    Ok(())
}

/// Reads a length-prefixed bincode-serialized message from an async stream.
pub async fn recv_frame<R, T>(reader: &mut R) -> Result<Option<T>>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let len = match reader.read_u32().await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e).context("Failed to read frame length"),
    };

    // Safety limit: 128 MB per frame
    if len > 128 * 1024 * 1024 {
        anyhow::bail!("Frame size exceeds 128MB limit: {} bytes", len);
    }

    let mut buf = vec![0u8; len as usize];
    reader
        .read_exact(&mut buf)
        .await
        .context("Failed to read frame payload")?;

    let message: T = bincode::deserialize(&buf)
        .context("Failed to deserialize frame with bincode")?;
    Ok(Some(message))
}
