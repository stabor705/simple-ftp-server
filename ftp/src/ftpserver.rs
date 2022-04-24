use std::net::{SocketAddr, TcpListener};

use crate::protocol_interpreter::ProtocolInterpreter;
use crate::data_transfer_process::DataTransferProcess;
use crate::Config;

use anyhow::Result;

pub struct FtpServer {
    listener: TcpListener,
    config: Config
}

impl FtpServer {
    pub fn new(config: Config) -> Result<FtpServer> {
        Ok(FtpServer {
            listener: TcpListener::bind((config.ip, config.control_port))?,
            config
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn run(&mut self) -> Result<()> {
        let mut dtp = DataTransferProcess::new(self.config.dir_root.clone());
        let mut pi = ProtocolInterpreter::new(&mut dtp);
        for client in self.listener.incoming() {
            match client {
                Ok(client) => {
                    if let Err(e) = pi.handle_client(client) {
                        log::error!("An error while handling connection: {:?}", e);
                    }
                }
                Err(e) => log::error!("An error occurred before connection took place: {}", e)
                }
        }
        Ok(())
    }

    pub fn do_one_listen(&mut self) -> Result<()> {
        let mut dtp = DataTransferProcess::new(self.config.dir_root.clone());
        let mut pi = ProtocolInterpreter::new(&mut dtp);
        let (client, _) = self.listener.accept()?;
        pi.handle_client(client)?;
        Ok(())
    }
}