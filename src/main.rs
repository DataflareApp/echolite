mod cli;
mod sqlite;

use crate::cli::Password;
use clap::Parser;
use protocol::*;
use sqlite::Sqlite;
use std::io::Error as IoError;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::BufStream;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use tracing::{error, info, trace, warn};
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    Io(#[from] IoError),
    #[error("Protocol Error: {0}")]
    Protocol(#[from] protocol::Error),
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SQLite: Invalid Flags For File Open Operations")]
    InvalidFlags,
    #[error("Tokio Semaphore Acquire Error: {0}")]
    Semaphore(#[from] tokio::sync::AcquireError),
}

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false))
        .with(
            Targets::new()
                .with_target(env!("CARGO_PKG_NAME"), args.log)
                .with_default(LevelFilter::OFF),
        )
        .init();

    if args.password.is_empty() {
        warn!("Authorization password is not set!!!");
    }
    if !args.bind.ip().is_loopback() {
        warn!("Binding to non-loopback address!!!");
    }

    let listener = TcpListener::bind(args.bind).await.unwrap_or_else(|err| {
        error!("Failed to bind to {}: {}", args.bind, err);
        std::process::exit(1);
    });

    let addr = listener.local_addr().unwrap_or_else(|err| {
        error!("Failed to get local address: {}", err);
        std::process::exit(1);
    });
    info!("Listening on: {}", addr);

    if let Err(err) = run(listener, args.password).await {
        error!("Error : {:?}", err);
        std::process::exit(1);
    }
}

async fn run(tcp: TcpListener, password: Password) -> Result<()> {
    loop {
        let (stream, client) = match tcp.accept().await {
            Ok(v) => v,
            Err(e) => {
                error!("Failed to accept TcpStream: {}", e);
                sleep(Duration::from_secs(3)).await;
                continue;
            }
        };
        tokio::spawn(connection(stream, client, password.clone()));
    }
}

#[tracing::instrument(skip(stream, password))]
async fn connection(stream: TcpStream, client: SocketAddr, password: Password) {
    trace!("Accepted TcpStream successfully");
    let stream = BufStream::new(stream);
    info!("Start handling connection");
    match handler(stream, password).await {
        Ok(_) => {
            info!("Connection handling finished");
        }
        Err(error) => {
            error!(?error, "Error handling connection");
        }
    };
}

async fn handler(mut stream: BufStream<TcpStream>, password: Password) -> Result<()> {
    write_protocol_version(&mut stream).await?;

    let client_salt = read_salt(&mut stream).await?;
    let server_salt = rand_salt();
    write_salt(&mut stream, server_salt).await?;

    let params = Params::default();
    write_hash_params(&mut stream, params).await?;

    let hashed = read_auth_password(&mut stream).await?;
    match password
        .verify(client_salt, server_salt, params, hashed)
        .await?
    {
        true => {
            write_status(&mut stream, Status::Ok).await?;
        }
        false => {
            error!("Password verification failed");
            write_status(
                &mut stream,
                Status::Err("Password verification failed".into()),
            )
            .await?;
            return Ok(());
        }
    }

    let (path, flags) = read_connect(&mut stream).await?;
    let mut conn = match Sqlite::connect(&path, flags) {
        Ok(conn) => {
            info!(%flags, %path, "Connected to database successfully");
            write_status(&mut stream, Status::Ok).await?;
            conn
        }
        Err(error) => {
            error!(%flags, %path, %error, "Failed to connect to database");
            write_status(&mut stream, Status::Err(error.to_string())).await?;
            return Ok(());
        }
    };

    loop {
        let command = read_command(&mut stream).await?;
        trace!(?command, "Received");
        match command {
            Command::Ping => {
                write_status(&mut stream, Status::Ok).await?;
            }
            Command::Disconnect => {
                break;
            }
            Command::SimpleExecute { sql } => {
                match conn.execute(&sql) {
                    Ok(_) => {
                        write_status(&mut stream, Status::Ok).await?;
                    }
                    Err(e) => {
                        write_status(&mut stream, Status::Err(e.to_string())).await?;
                    }
                };
            }
            Command::SimpleQuery { sql } => {
                match conn.query(&sql) {
                    Ok(query) => {
                        write_status(&mut stream, Status::Ok).await?;
                        write_query(&mut stream, query).await?;
                    }
                    Err(e) => {
                        write_status(&mut stream, Status::Err(e.to_string())).await?;
                    }
                };
            }
            Command::Transaction { sqls } => match conn.transaction(sqls) {
                Ok(_) => {
                    write_status(&mut stream, Status::Ok).await?;
                }
                Err(e) => {
                    write_status(&mut stream, Status::Err(e.to_string())).await?;
                }
            },
        }
    }
    Ok(())
}
