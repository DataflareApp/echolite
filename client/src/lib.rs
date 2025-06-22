use protocol::*;
pub use protocol::{Column, Error as ProtocolError, Flags, Query, Value, Version, consts::*};
use tokio::io::{AsyncRead, AsyncWrite, BufStream};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Protocol(#[from] protocol::Error),
    #[error("Unsupported Version: {0:?}")]
    UnsupportedVersion(Version),
    #[error("Response: {0}")]
    Status(String),
    #[error("Only UTF-8 'TEXT' value is supported")]
    InvalidUtf8,
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Connection<T> {
    stream: BufStream<T>,
}

impl<T> Connection<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn connect<P: AsRef<str>, D: AsRef<str>>(
        stream: T,
        password: P,
        path: D,
        flags: Flags,
    ) -> Result<Self> {
        let mut stream = BufStream::new(stream);

        let version = read_protocol_version(&mut stream).await?;
        if version.major != 1 {
            return Err(Error::UnsupportedVersion(version));
        }

        let client_salt = rand_salt();
        write_salt(&mut stream, client_salt).await?;

        let server_salt = read_salt(&mut stream).await?;
        let params = read_hash_params(&mut stream).await?;

        write_auth_password(&mut stream, password, client_salt, server_salt, params).await?;
        Self::status(&mut stream).await?;

        write_connect(&mut stream, path, flags).await?;
        Self::status(&mut stream).await?;

        Ok(Self { stream })
    }

    async fn status(reader: &mut BufStream<T>) -> Result<()> {
        read_status(reader)
            .await?
            .to_result()
            .map_err(Error::Status)?;
        Ok(())
    }

    pub async fn ping(&mut self) -> Result<()> {
        write_command(&mut self.stream, Command::Ping).await?;
        Self::status(&mut self.stream).await?;
        Ok(())
    }

    pub async fn execute<S: Into<String>>(&mut self, sql: S) -> Result<()> {
        write_command(&mut self.stream, Command::SimpleExecute { sql: sql.into() }).await?;
        Self::status(&mut self.stream).await?;
        Ok(())
    }

    pub async fn query<S: Into<String>>(&mut self, sql: S) -> Result<Query> {
        write_command(&mut self.stream, Command::SimpleQuery { sql: sql.into() }).await?;
        Self::status(&mut self.stream).await?;
        let query = read_query(&mut self.stream).await?;
        Ok(query)
    }

    pub async fn transaction<I: IntoIterator<Item = S>, S: ToString>(
        &mut self,
        sqls: I,
    ) -> Result<()> {
        let sqls = sqls.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
        write_command(&mut self.stream, Command::Transaction { sqls }).await?;
        Self::status(&mut self.stream).await?;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        write_command(&mut self.stream, Command::Disconnect).await?;
        Ok(())
    }
}
