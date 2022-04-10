use std::net::{TcpStream, SocketAddr, TcpListener, Ipv4Addr, IpAddr};
use std::fs::{File};
use std::path::Path;
use std::io::{Read, Write, Result, Error, ErrorKind};
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::thread::sleep;

use strum_macros::{Display, EnumString};

#[derive(Display, EnumString)]
pub enum DataTypes {
    #[strum(serialize="A")]
    ASCII(DataFormats),
    #[strum(serialize="E")]
    EBCDIC(DataFormats),
    #[strum(serialize="I")]
    Image,
    #[strum(serialize="L")]
    Local(u8)
}

#[derive(Display, EnumString)]
pub enum DataFormats {
    #[strum(serialize="N")]
    NonPrint,
    #[strum(serialize="T")]
    TelnetFormatEffectors,
    #[strum(serialize="C")]
    CarriageControl
}

impl Default for DataFormats {
    fn default() -> Self {
        Self::NonPrint
    }
}

#[derive(Display, EnumString)]
pub enum DataStructures {
    #[strum(serialize="F")]
    FileStructure,
    #[strum(serialize="R")]
    RecordStructure,
    #[strum(serialize="P")]
    PageStructure
}

#[derive(Display, EnumString)]
pub enum TransferModes {
    #[strum(serialize="S")]
    Stream,
    #[strum(serialize="B")]
    Block,
    #[strum(serialize="C")]
    Compressed
}

pub struct DataTransferProcess {
    root: String,
    mode: Box<dyn Mode>,
    stream: Option<TcpStream>
}

impl DataTransferProcess {
    pub fn new(root: String) -> DataTransferProcess {
        DataTransferProcess {
            root,
            mode: Box::new(Active {}),
            stream: None
        }
    }

    pub fn make_passive(&mut self) -> Result<u16> {
        let passive = Passive::new(Duration::from_secs(120))?;
        let port = passive.port();
        self.mode = Box::new(passive);
        log::info!("DTP started listening on port {}", port);
        Ok(port)
    }

    pub fn make_active(&mut self) {
        self.mode = Box::new(Active {});
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Result<()> {
        self.stream = Some(self.mode.connect(addr)?);
        log::debug!("Stopped listening");
        Ok(())
    }

    pub fn send_file(&mut self, path: &str) -> Result<()> {
        let mut stream = match &self.stream {
            Some(stream) => stream,
            None => return Err(Error::from(ErrorKind::NotConnected))
        };
        let path = Path::new(&self.root).join(path);
        let mut file = File::open(path)?;
        loop {
            //TODO: How big should it be?
            let mut buf = [0; 512];
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            stream.write_all(&buf[0..n]);
        }
        Ok(())
    }
}

trait Mode {
    fn connect(&self, addr: SocketAddr) -> Result<TcpStream>;
}

struct Active {}

impl Mode for Active {
    fn connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        TcpStream::connect(addr)
    }
}

struct Passive {
    listener: TcpListener,
    timeout: Duration
}

impl Passive {
    pub fn new(timeout: Duration) -> Result<Passive> {
        Ok(Passive {
            listener: TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?,
            timeout
        })
    }

    pub fn port(&self) -> u16 {
        //TODO: I don't know why this function can error. Gotta get rid of this unwrap someday.
        self.listener.local_addr().unwrap().port()
    }
}

impl Mode for Passive {
    fn connect(&self, addr: SocketAddr) -> Result<TcpStream> {
        let start = Instant::now();
        log::debug!("Started listening");
        while start.elapsed() < self.timeout {
            match self.listener.accept() {
                Ok((stream, in_addr)) => {
                    if in_addr.ip() == addr.ip() {
                        log::info!("Accepting data connection from {}", in_addr);
                        return Ok(stream);
                    } else {
                        log::info!("Dropping connection from {}. Incorrect ip address.", in_addr);
                    }
                }
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        sleep(Duration::from_millis(250));
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Err(Error::from(ErrorKind::TimedOut))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializing_parameters() {
        let data_type = "A N".parse::<DataType>()?;
        assert_eq!(data_type.data_type, DataTypes::ASCII);
        assert_eq!(data_type.data_format, DataFormats::NonPrint);
    }
}