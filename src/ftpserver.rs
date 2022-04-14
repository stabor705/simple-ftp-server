use std::net::{TcpListener, ToSocketAddrs};
use std::io::Result;
use crate::protocol_interpreter::ProtocolInterpreter;

pub struct FtpServer {}

impl FtpServer {
    pub fn run<A: ToSocketAddrs>(addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        let pi = ProtocolInterpreter {};
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = pi.handle_client(stream) {
                        log::error!("An error while handling connection: {:?}", e);
                    }
                }
                Err(e) => log::error!("An error occurred before connection took place: {}", e)
                }
        }
        Ok(())
    }
}