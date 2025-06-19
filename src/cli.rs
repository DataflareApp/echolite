use clap::Parser;
use protocol::{Salt, to_hashed_password};
use rand::Rng;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tracing::level_filters::LevelFilter;

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

#[derive(Debug, Clone)]
pub struct Password(Arc<String>);

impl Password {
    fn from_str(value: &str) -> Result<Self, String> {
        Ok(Password(Arc::new(value.to_string())))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn rand_salt(&self) -> Salt {
        let mut salt: Salt = [0; 16];
        rand::rng().fill(&mut salt);
        salt
    }

    pub fn verify(&self, salt: Salt, hashed_password: [u8; 32]) -> bool {
        to_hashed_password(self.0.as_str(), salt) == hashed_password
    }
}
