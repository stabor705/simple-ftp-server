use std::default::Default;
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::time::Duration;

use crate::protocol_interpreter::ProtocolInterpreter;
use crate::user::*;

use anyhow::{Error, Result};

pub struct FtpConfig {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub users: UserStore,
    pub conn_timeout: Duration,
}

impl Default for FtpConfig {
    fn default() -> Self {
        FtpConfig {
            ip: Ipv4Addr::LOCALHOST,
            port: 0,
            users: UserStore::default(),
            conn_timeout: Duration::from_secs(180),
        }
    }
}

pub struct FtpServerBuilder {
    config: FtpConfig,
}

impl FtpServerBuilder {
    pub fn new() -> FtpServerBuilder {
        FtpServerBuilder {
            config: FtpConfig::default(),
        }
    }

    pub fn set_ip(mut self, ip: Ipv4Addr) -> Self {
        self.config.ip = ip;
        self
    }

    pub fn set_listening_port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn add_user(mut self, username: Username, password: Password, dir: String) -> Self {
        self.config.users.insert(username, User { password, dir });
        self
    }

    pub fn set_connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.conn_timeout = timeout;
        self
    }

    pub fn build(self) -> Result<FtpServer> {
        FtpServer::new(self.config)
    }
}

pub struct FtpServer {
    listener: TcpListener,
    config: Option<FtpConfig>,
}

impl FtpServer {
    pub fn builder() -> FtpServerBuilder {
        FtpServerBuilder::new()
    }

    pub fn new(config: FtpConfig) -> Result<FtpServer> {
        Ok(FtpServer {
            listener: TcpListener::bind((config.ip, config.port))?,
            config: Some(config),
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn run(&mut self) -> Result<()> {
        let config = self
            .config
            .take()
            .ok_or(Error::msg("FtpServer should not be run 2 times."))?;
        let mut pi = ProtocolInterpreter::new(config.users, config.conn_timeout);
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
        Ok(())
    }

    pub fn do_one_listen(&mut self) -> Result<()> {
        let config = self
            .config
            .take()
            .ok_or(Error::msg("FtpServer should not be run 2 times."))?;
        let mut pi = ProtocolInterpreter::new(config.users, config.conn_timeout);
        let (client, _) = self.listener.accept()?;
        pi.handle_client(client)?;
        Ok(())
    }
}
