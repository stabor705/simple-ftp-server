use std::net::{SocketAddr, TcpListener, ToSocketAddrs};

use crate::protocol_interpreter::ProtocolInterpreter;

use anyhow::Result;
use crate::data_transfer_process::DataTransferProcess;

pub struct FtpServer {
    listener: TcpListener
}

impl FtpServer {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<FtpServer> {
        Ok(FtpServer {
            listener: TcpListener::bind(addr)?
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn run(&self) -> Result<()> {
        let mut dtp = DataTransferProcess::new(".".to_owned());
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

    pub fn do_one_listen(&self) -> Result<()> {
        let mut dtp = DataTransferProcess::new(".".to_owned());
        let mut pi = ProtocolInterpreter::new(&mut dtp);
        let (client, _) = self.listener.accept()?;
        pi.handle_client(client)?;
        Ok(())
    }
}