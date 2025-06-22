mod ext;
mod flags;

use argon2::{Algorithm, Argon2, Params as Argon2Params, Version as Argon2Version};
use ext::{ReadExt, WriteExt};
use rand::Rng;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub use flags::*;
use zeroize::{Zeroize, ZeroizeOnDrop};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Async Runtime Error: {0}")]
    AsyncRuntime(#[from] tokio::task::JoinError),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Varint Error")]
    Varint,
    #[error("Invalid Argon2 Params")]
    Argon2Params,
    #[error("Argon2 Error: {0}")]
    Argon2(argon2::Error),
    #[error("Unknown Status: {0}")]
    UnknownStatus(u8),
    #[error("Unknown Command: {0}")]
    UnknownCommand(u8),
    #[error("Unknown Value: {0}")]
    UnknownValue(u8),
    #[error("Invalid query values length: values {0}, columns {1}")]
    InvalidValuesLength(usize, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

pub async fn write_protocol_version<W: AsyncWrite + Unpin>(writer: &mut W) -> Result<()> {
    let version = Version { major: 1, minor: 0 };
    writer.write_u8(version.major).await?;
    writer.write_u8(version.minor).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_protocol_version<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Version> {
    let major = reader.read_u8().await?;
    let minor = reader.read_u8().await?;
    Ok(Version { major, minor })
}

pub type Salt = [u8; 16];
pub type HashedPassword = [u8; 32];

pub async fn write_salt<W: AsyncWrite + Unpin>(writer: &mut W, salt: Salt) -> Result<()> {
    writer.write_all(&salt).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_salt<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Salt> {
    let mut buf: Salt = [0; 16];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            m_cost: 65536,
            t_cost: 8,
            p_cost: 1,
        }
    }
}

pub async fn write_hash_params<W: AsyncWrite + Unpin>(
    writer: &mut W,
    params: Params,
) -> Result<()> {
    writer.write_len(params.m_cost as u64).await?;
    writer.write_len(params.t_cost as u64).await?;
    writer.write_len(params.p_cost as u64).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_hash_params<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Params> {
    fn check_size(size: u64) -> Result<u32> {
        if size > u32::MAX as u64 {
            return Err(Error::Argon2Params);
        }
        Ok(size as u32)
    }
    Ok(Params {
        m_cost: check_size(reader.read_len().await?)?,
        t_cost: check_size(reader.read_len().await?)?,
        p_cost: check_size(reader.read_len().await?)?,
    })
}

pub async fn to_hash_password<P: AsRef<str>>(
    password: P,
    client_salt: Salt,
    server_salt: Salt,
    params: Params,
) -> Result<HashedPassword> {
    #[derive(Zeroize, ZeroizeOnDrop)]
    struct Password(Vec<u8>);

    let params = Argon2Params::new(params.m_cost, params.t_cost, params.p_cost, Some(32))
        .map_err(Error::Argon2)?;
    let hasher = Argon2::new(Algorithm::Argon2id, Argon2Version::V0x13, params);
    let password = Password(password.as_ref().as_bytes().to_vec());

    let mut salt = [0; 32];
    salt[..16].copy_from_slice(&client_salt);
    salt[16..].copy_from_slice(&server_salt);

    tokio::task::spawn_blocking(move || {
        let mut out = [0; 32];
        hasher
            .hash_password_into(&password.0, &salt, &mut out)
            .map_err(Error::Argon2)?;
        Ok(out)
    })
    .await?
}

pub fn rand_salt() -> Salt {
    let mut salt: Salt = [0; 16];
    rand::rng().fill(&mut salt);
    salt
}

pub async fn write_auth_password<W: AsyncWrite + Unpin, P: AsRef<str>>(
    writer: &mut W,
    password: P,
    client_salt: Salt,
    server_salt: Salt,
    params: Params,
) -> Result<()> {
    let p = to_hash_password(password, client_salt, server_salt, params).await?;
    writer.write_all(&p).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_auth_password<R: AsyncRead + Unpin>(reader: &mut R) -> Result<HashedPassword> {
    let mut buf = [0; 32];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

pub async fn write_connect<W: AsyncWrite + Unpin, P: AsRef<str>>(
    writer: &mut W,
    path: P,
    flags: Flags,
) -> Result<()> {
    writer.write_string(path).await?;
    writer.write_i32(flags.bits()).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_connect<R: AsyncRead + Unpin>(reader: &mut R) -> Result<(String, Flags)> {
    let path = reader.read_string().await?;
    let flags = reader.read_i32().await?;
    Ok((path, Flags::from_flags(flags)))
}

#[derive(Debug)]
pub enum Status {
    Ok,
    Err(String),
}

impl Status {
    #[inline]
    pub fn to_result(self) -> Result<(), String> {
        match self {
            Status::Ok => Ok(()),
            Status::Err(err) => Err(err),
        }
    }
}

pub async fn write_status<W: AsyncWrite + Unpin>(writer: &mut W, status: Status) -> Result<()> {
    match status {
        Status::Ok => {
            writer.write_u8(0).await?;
        }
        Status::Err(err) => {
            writer.write_u8(1).await?;
            writer.write_string(err).await?;
        }
    }
    writer.flush().await?;
    Ok(())
}

pub async fn read_status<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Status> {
    match reader.read_u8().await? {
        0 => Ok(Status::Ok),
        1 => Ok(Status::Err(reader.read_string().await?)),
        n => Err(Error::UnknownStatus(n)),
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Ping,
    Disconnect,
    SimpleExecute { sql: String },
    SimpleQuery { sql: String },
    Transaction { sqls: Vec<String> },
    // SetDbConfig
    // SetLimit
    // LoadExtension
    // Prepare
}

pub async fn write_command<W: AsyncWrite + Unpin>(writer: &mut W, cmd: Command) -> Result<()> {
    match cmd {
        Command::Ping => {
            writer.write_u8(0).await?;
        }
        Command::Disconnect => {
            writer.write_u8(1).await?;
        }
        Command::SimpleExecute { sql } => {
            writer.write_u8(2).await?;
            writer.write_string(sql).await?;
        }
        Command::SimpleQuery { sql } => {
            writer.write_u8(3).await?;
            writer.write_string(sql).await?;
        }
        Command::Transaction { sqls } => {
            writer.write_u8(4).await?;
            writer.write_len(sqls.len() as u64).await?;
            for sql in sqls {
                writer.write_string(sql).await?;
            }
        }
    }
    writer.flush().await?;
    Ok(())
}

pub async fn read_command<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Command> {
    let cmd = match reader.read_u8().await? {
        0 => Command::Ping,
        1 => Command::Disconnect,
        2 => {
            let sql = reader.read_string().await?;
            Command::SimpleExecute { sql }
        }
        3 => {
            let sql = reader.read_string().await?;
            Command::SimpleQuery { sql }
        }
        4 => {
            let len = reader.read_len().await? as usize;
            let mut sqls = Vec::with_capacity(len);
            for _ in 0..len {
                sqls.push(reader.read_string().await?);
            }
            Command::Transaction { sqls }
        }
        other => return Err(Error::UnknownCommand(other)),
    };
    Ok(cmd)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub columns: Vec<Column>,
    pub values: Vec<Value>,
    pub rows_affected: u64,
    pub duration: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
    pub datatype: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    I64(i64),
    F64(f64),
    Bytes(Vec<u8>),
    Text(Vec<u8>),
}

async fn write_columns<W: AsyncWrite + Unpin>(writer: &mut W, columns: &[Column]) -> Result<()> {
    writer.write_len(columns.len() as u64).await?;
    for column in columns {
        writer.write_string(&column.name).await?;
        writer.write_string(&column.datatype).await?;
    }
    Ok(())
}

async fn read_columns<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<Column>> {
    let len = reader.read_len().await? as usize;
    let mut columns = Vec::with_capacity(len);
    for _ in 0..len {
        let name = reader.read_string().await?;
        let datatype = reader.read_string().await?;
        columns.push(Column { name, datatype });
    }
    Ok(columns)
}

async fn write_values<W: AsyncWrite + Unpin>(writer: &mut W, values: &[Value]) -> Result<()> {
    writer.write_len(values.len() as u64).await?;
    for value in values {
        match value {
            Value::Null => {
                writer.write_u8(0).await?;
            }
            Value::I64(v) => {
                if *v >= 0 {
                    writer.write_u8(1).await?;
                    writer.write_len(*v as u64).await?;
                } else {
                    writer.write_u8(2).await?;
                    let encoded = ((*v << 1) ^ (*v >> 63)) as u64; // ZigZag
                    writer.write_len(encoded).await?;
                }
            }
            Value::F64(v) => {
                writer.write_u8(3).await?;
                writer.write_f64(*v).await?;
            }
            Value::Bytes(v) => {
                if v.is_empty() {
                    writer.write_u8(4).await?;
                } else {
                    writer.write_u8(5).await?;
                    writer.write_bytes(v).await?;
                }
            }
            Value::Text(v) => {
                if v.is_empty() {
                    writer.write_u8(6).await?;
                } else {
                    writer.write_u8(7).await?;
                    writer.write_bytes(v).await?;
                }
            }
        }
    }
    Ok(())
}

async fn read_values<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Vec<Value>> {
    let len = reader.read_len().await? as usize;
    let mut values = Vec::with_capacity(len);
    for _ in 0..len {
        let type_id = reader.read_u8().await?;
        let value = match type_id {
            0 => Value::Null,
            1 => Value::I64(reader.read_len().await? as i64),
            2 => {
                let encoded = reader.read_len().await?;
                let decoded = ((encoded >> 1) as i64) ^ -((encoded & 1) as i64); // ZigZag
                Value::I64(decoded)
            }
            3 => Value::F64(reader.read_f64().await?),
            4 => Value::Bytes(Vec::new()),
            5 => Value::Bytes(reader.read_bytes().await?),
            6 => Value::Text(Vec::new()),
            7 => Value::Text(reader.read_bytes().await?),
            type_id => return Err(Error::UnknownValue(type_id)),
        };
        values.push(value);
    }
    Ok(values)
}

pub async fn write_query<W: AsyncWrite + Unpin>(writer: &mut W, query: Query) -> Result<()> {
    write_columns(writer, &query.columns).await?;
    write_values(writer, &query.values).await?;
    writer.write_len(query.rows_affected).await?;
    writer.write_len(query.duration).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_query<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Query> {
    let columns = read_columns(reader).await?;
    let values = read_values(reader).await?;
    let rows_affected = reader.read_len().await?;
    let duration = reader.read_len().await?;
    if columns.is_empty() && !values.is_empty() {
        return Err(Error::InvalidValuesLength(values.len(), 0));
    }
    if !values.is_empty() && values.len() % columns.len() != 0 {
        return Err(Error::InvalidValuesLength(values.len(), columns.len()));
    }
    Ok(Query {
        columns,
        values,
        rows_affected,
        duration,
    })
}

#[cfg(test)]
mod tests {}
