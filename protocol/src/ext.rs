use crate::{Error, Result};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub trait WriteExt: AsyncWrite + Unpin {
    /// Writes a length as a variable-length integer (varint)
    async fn write_len(&mut self, mut len: u64) -> Result<()> {
        if len < 0x80 {
            self.write_u8(len as u8).await?;
        } else {
            while len >= 0x80 {
                self.write_u8((len & 0x7F) as u8 | 0x80).await?;
                len >>= 7;
            }
            self.write_u8(len as u8).await?;
        }
        Ok(())
    }

    async fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_len(bytes.len() as u64).await?;
        self.write_all(bytes).await?;
        Ok(())
    }

    async fn write_string<S: AsRef<str>>(&mut self, val: S) -> Result<()> {
        let bytes = val.as_ref().as_bytes();
        self.write_bytes(bytes).await
    }
}

pub trait ReadExt: AsyncRead + Unpin {
    async fn read_len(&mut self) -> Result<u64> {
        let mut len = 0_u64;
        let mut shift = 0;
        loop {
            let byte = self.read_u8().await?;
            if shift >= u64::BITS {
                return Err(Error::Varint);
            }
            let value = (byte & 0x7F) as u64;
            match value.checked_shl(shift) {
                Some(shifted) => len |= shifted,
                None => return Err(Error::Varint),
            }
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        Ok(len)
    }

    async fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let len = self.read_len().await? as usize;
        let mut buf = vec![0; len];
        self.read_exact(&mut buf).await?;
        Ok(buf)
    }

    async fn read_string(&mut self) -> Result<String> {
        let buf = self.read_bytes().await?;
        Ok(String::from_utf8(buf)?)
    }
}

impl<W: AsyncWrite + Unpin> WriteExt for W {}
impl<R: AsyncRead + Unpin> ReadExt for R {}

// TODO
#[cfg(test)]
mod tests {}
