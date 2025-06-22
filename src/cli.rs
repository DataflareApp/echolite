use crate::Error;
use clap::Parser;
use protocol::{Params, Salt, to_hash_password};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::level_filters::LevelFilter;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Parser, Debug)]
#[clap(version)]
pub struct Args {
    /// Set listen address
    #[clap(short,  long, name = "ADDRESS|IP|PORT", env = "ECHOLITE_BIND", value_parser = to_socket_addr, default_value_t = DEFAULT_BIND)]
    pub bind: SocketAddr,

    /// Set auth password
    #[clap(short, long, env = "ECHOLITE_PASSWORD", value_parser = Password::from_str)]
    pub password: Password,

    /// Set log level
    #[clap(
        short,
        long,
        name = "LOG_LEVEL",
        env = "ECHOLITE_LOG",
        default_value = "info"
    )]
    pub log: LevelFilter,
}

const IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const PORT: u16 = 4567;
const DEFAULT_BIND: SocketAddr = SocketAddr::new(IP, PORT);

fn to_socket_addr(s: &str) -> Result<SocketAddr, String> {
    // 0.0.0.0:80
    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Ok(addr);
    }
    // 0.0.0.0 / ::
    if let Ok(ip) = s.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, PORT));
    }
    // 80
    if let Ok(port) = s.parse::<u16>() {
        return Ok(SocketAddr::new(IP, port));
    }
    Err(format!("Cannot parse `{}` to SocketAddr", s))
}

static LIMIT: Semaphore = Semaphore::const_new(2);

#[derive(Debug, Clone)]
pub struct Password(Arc<SecurePassword>);

#[derive(Debug, Zeroize, ZeroizeOnDrop)]
struct SecurePassword(String);

impl Password {
    fn from_str(value: &str) -> Result<Self, String> {
        Ok(Password(Arc::new(SecurePassword(value.to_string()))))
    }

    pub fn is_empty(&self) -> bool {
        self.0.0.is_empty()
    }

    pub async fn verify(
        &self,
        client_salt: Salt,
        server_salt: Salt,
        params: Params,
        client_password: [u8; 32],
    ) -> Result<bool, Error> {
        let _limit = LIMIT.acquire().await?;
        let server_password =
            to_hash_password(self.0.0.as_str(), client_salt, server_salt, params).await?;
        Ok(server_password == client_password)
    }
}
