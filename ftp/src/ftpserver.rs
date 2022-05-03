use std::default::Default;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::time::Duration;

use crate::protocol_interpreter::ProtocolInterpreter;
use crate::user::*;

use anyhow::Result;

pub struct FtpConfig {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub users: Vec<User>,
    pub conn_timeout: Duration,
}

impl Default for FtpConfig {
    fn default() -> Self {
        FtpConfig {
            ip: Ipv4Addr::LOCALHOST,
            port: 0,
            users: Vec::new(),
            conn_timeout: Duration::from_secs(180),
        }
    }
}

pub struct FtpServer {
    listener: TcpListener,
    config: FtpConfig,
}

impl FtpServer {
    pub fn new(config: FtpConfig) -> std::io::Result<FtpServer> {
        Ok(FtpServer {
            listener: TcpListener::bind((config.ip, config.port))?,
            config,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn run(self) {
        let mut pi = ProtocolInterpreter::new(self.config.users, self.config.conn_timeout);
        for client in self.listener.incoming() {
            match client {
                Ok(client) => {
                    if let Err(e) = pi.handle_client(client) {
                        log::error!("An error while handling connection: {:?}", e);
                    }
                }
                Err(e) => log::error!("An error occurred before connection took place: {}", e),
            }
        }
    }

    pub fn do_one_listen(self) -> Result<()> {
        let mut pi = ProtocolInterpreter::new(self.config.users, self.config.conn_timeout);
        let (client, _) = self.listener.accept()?;
        pi.handle_client(client)?;
        Ok(())
    }
}
