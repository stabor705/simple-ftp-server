use std::ffi::OsString;
use std::net::{TcpStream, SocketAddr, TcpListener, Ipv4Addr, IpAddr, ToSocketAddrs};
use std::fs::{DirEntry, File, read_dir};
use std::path::Path;
use std::io::{Read, Write, Result, Error, ErrorKind};
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::thread::sleep;

use strum_macros::{Display, EnumString};
use crate::data_transfer_process::TransferMode::Stream;

#[derive(Display, EnumString)]
pub enum DataType {
    #[strum(serialize="A")]
    ASCII(DataFormat),
    #[strum(serialize="E")]
    EBCDIC(DataFormat),
    #[strum(serialize="I")]
    Image,
    #[strum(serialize="L")]
    Local(u8)
}

impl Default for DataType {
    fn default() -> Self {
        DataType::ASCII(DataFormat::default())
    }
}

#[derive(Display, EnumString)]
pub enum DataFormat {
    #[strum(serialize="N")]
    NonPrint,
    #[strum(serialize="T")]
    TelnetFormatEffectors,
    #[strum(serialize="C")]
    CarriageControl
}

impl Default for DataFormat {
    fn default() -> Self {
        Self::NonPrint
    }
}

#[derive(Display, EnumString)]
pub enum DataStructure {
    #[strum(serialize="F")]
    FileStructure,
    #[strum(serialize="R")]
    RecordStructure,
    #[strum(serialize="P")]
    PageStructure
}

impl Default for DataStructure {
    fn default() -> Self {
        DataStructure::FileStructure
    }
}

#[derive(Display, EnumString)]
pub enum TransferMode {
    #[strum(serialize="S")]
    Stream,
    #[strum(serialize="B")]
    Block,
    #[strum(serialize="C")]
    Compressed
}

impl Default for TransferMode {
    fn default() -> Self {
       TransferMode::Stream
    }
}

pub struct DataTransferProcess {
    working_dir: String,
    mode: Box<dyn Mode>,
    client: Option<TcpStream>,
    pub data_type: DataType,
    pub data_structure: DataStructure,
    pub transfer_mode: TransferMode
}

impl DataTransferProcess {
    pub fn new(root: String) -> DataTransferProcess {
        DataTransferProcess {
            working_dir: root,
            mode: Box::new(Active {}),
            client: None,
            data_type: DataType::ASCII(DataFormat::NonPrint),
            data_structure: DataStructure::FileStructure,
            transfer_mode: TransferMode::Stream
        }
    }

    pub fn make_passive(&mut self) -> Result<SocketAddr> {
        let passive = Passive::new(Duration::from_secs(120))?;
        let addr = passive.addr();
        self.mode = Box::new(passive);
        log::info!("DTP started listening on port {}", addr);
        Ok(addr)
    }

    pub fn make_active(&mut self) {
        self.mode = Box::new(Active {});
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Option<Result<()>> {
        match self.client {
            Some(_) => None,
            None => match self.mode.connect(addr) {
                Ok(client) => {
                    self.client = Some(client);
                    Some(Ok(()))
                }
                Err(e) => Some(Err(e))
            }
        }
    }

    pub fn send_file(&mut self, path: &str) -> Result<()> {
        let mut client = self.get_client()?;
        let path = Path::new(&self.working_dir).join(path);
        let mut file = File::open(path)?;
        loop {
            //TODO: How big should it be?
            let mut buf = [0; 512];
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            client.write_all(&buf[0..n]);
        }
        Ok(())
    }

    pub fn send_dir_listing(&mut self, path: Option<String>) -> Result<()> {
        let mut client = self.get_client()?;
        let dir = match path {
            Some(path) => Path::new(&self.working_dir).join(path),
            None => Path::new(&self.working_dir).to_path_buf()
        };
        let mut listing: Vec<String> = Vec::new();
        for entry in read_dir(dir)? {
            match entry {
                Ok(entry) => listing.push(entry.file_name().to_string_lossy().into_owned()),
                Err(e) => return Err(e)
            }
        }
        for filename in listing {
            client.write_all(filename.as_bytes())?;
            client.write_all("\r\n".as_bytes())?;
        }
        self.client = None;
        Ok(())
    }

    fn get_client(&self) -> Result<(&TcpStream)> {
        Ok(self.client.as_ref().ok_or(Error::from(ErrorKind::NotConnected))?)
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

    pub fn addr(&self) -> SocketAddr {
        //TODO: I don't know why this function can error. Gotta get rid of this unwrap someday.
        self.listener.local_addr().unwrap()
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
        assert_eq!(data_type.data_type, DataType::ASCII);
        assert_eq!(data_type.data_format, DataFormat::NonPrint);
    }
}