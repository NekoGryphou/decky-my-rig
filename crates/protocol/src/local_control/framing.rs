//! Length-prefixed local-control frames with size limits, shared by the service
//! and the notifier.
use std::io::{self, ErrorKind};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R, limit: usize) -> io::Result<Vec<u8>> {
    let mut prefix = [0_u8; 4];
    reader.read_exact(&mut prefix).await?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length == 0 || length > limit {
        return Err(ErrorKind::InvalidData.into());
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).await?;
    Ok(bytes)
}

pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    bytes: &[u8],
    limit: usize,
) -> io::Result<()> {
    if bytes.is_empty() || bytes.len() > limit {
        return Err(ErrorKind::InvalidInput.into());
    }
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .await?;
    writer.write_all(bytes).await
}
