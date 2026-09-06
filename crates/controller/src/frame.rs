// PIMProbe - touch probing for the Nestworks C500.
// Copyright (c) 2026 Konstantin Tcepliaev <f355@f355.org>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub const MAX_FRAME_SIZE: usize = 1024 * 1024;
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<(u8, Vec<u8>)> {
    let mut header = [0; 5];
    reader.read_exact(&mut header).await?;
    let size = u32::from_be_bytes(header[1..].try_into().unwrap()) as usize;
    if size > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "oversize bridge frame",
        ));
    }
    let mut payload = vec![0; size];
    reader.read_exact(&mut payload).await?;
    Ok((header[0], payload))
}
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    kind: u8,
    payload: &[u8],
) -> io::Result<()> {
    if payload.len() > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "oversize bridge frame",
        ));
    }
    let mut header = [kind; 5];
    header[1..].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    writer.write_all(&header).await?;
    writer.write_all(payload).await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn fragmented_stream_and_multiple_frames() {
        let (mut writer, mut reader) = tokio::io::duplex(1);
        let task = tokio::spawn(async move {
            write_frame(&mut writer, b'D', b"ok\n").await.unwrap();
            write_frame(&mut writer, b'D', b"error:9\n").await.unwrap();
        });
        assert_eq!(
            read_frame(&mut reader).await.unwrap(),
            (b'D', b"ok\n".to_vec())
        );
        assert_eq!(
            read_frame(&mut reader).await.unwrap(),
            (b'D', b"error:9\n".to_vec())
        );
        task.await.unwrap();
        assert!(read_frame(&mut reader).await.is_err());
        let mut output = Vec::new();
        assert!(write_frame(&mut output, b'Q', &vec![0; MAX_FRAME_SIZE + 1])
            .await
            .is_err());
        assert!(output.is_empty());
    }
    #[tokio::test]
    async fn framing_limits_and_truncation() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, b'Q', b"G54\n").await.unwrap();
        assert_eq!(bytes, b"Q\0\0\0\x04G54\n");
        assert_eq!(
            read_frame(&mut bytes.as_slice()).await.unwrap(),
            (b'Q', b"G54\n".to_vec())
        );
        assert!(read_frame(&mut &bytes[..7]).await.is_err());
        assert!(read_frame(&mut &b"D\x00\x10\x00\x01"[..]).await.is_err());
    }
}
