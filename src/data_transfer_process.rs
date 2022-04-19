use std::net::{TcpStream, SocketAddr, TcpListener, Ipv4Addr};
use std::fs::{File, read_dir};
use std::path::Path;
use std::io::{Read, Write, Result, Error, ErrorKind};
use std::time::{Duration, Instant};
use std::thread::sleep;

use strum_macros::{Display, EnumString};
use fallible_iterator::FallibleIterator;

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

#[derive(Default)]
pub struct DataRepr {
    pub data_type: DataType,
    pub data_structure: DataStructure,
    pub transfer_mode: TransferMode
}

pub struct DataTransferProcess {
    working_dir: String,
    mode: Box<dyn Mode>,
    client: Option<TcpStream>,
}

impl DataTransferProcess {
    pub fn new(root: String) -> DataTransferProcess {
        DataTransferProcess {
            working_dir: root,
            mode: Box::new(Active {}),
            client: None,
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
        let mut client = self.client.as_ref().ok_or(Error::from(ErrorKind::NotConnected))?;
        let path = Path::new(&self.working_dir).join(path);
        let mut file = File::open(path)?;
        loop {
            //TODO: How big should it be?
            let mut buf = [0; 512];
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            client.write_all(&buf[0..n])?;
        }
        self.client = None;
        Ok(())
    }

    pub fn receive_file(&mut self, path: &str) -> Result<()> {
        let mut client = self.client.as_ref().ok_or(Error::from(ErrorKind::NotConnected))?;
        let path = Path::new(&self.working_dir).join(path);
        let mut file = File::create(path)?;
        loop {
            //TODO: How big should it be?
            let mut buf = [0; 512];
            let n = client.read(&mut buf)?;
            if n == 0 { break; }
            file.write_all(&buf[0..n])?;
        }
        self.client = None;
        Ok(())
    }

    pub fn send_dir_listing(&mut self, path: Option<String>) -> Result<()> {
        let mut client = self.client.as_ref().ok_or(Error::from(ErrorKind::NotConnected))?;
        let listing = self.get_dir_listing(path)?;
        for filename in listing {
            client.write_all(filename.as_bytes())?;
            client.write_all("\r\n".as_bytes())?;
        }
        self.client = None;
        Ok(())
    }

    fn get_dir_listing(&self, path: Option<String>) -> Result<Vec<String>> {
        let dir = match path {
            Some(path) => Path::new(&self.working_dir).join(path),
            None => Path::new(&self.working_dir).to_path_buf()
        };
        let listing = fallible_iterator::convert(read_dir(dir)?)
            .map(|entry| Ok(entry.file_name().to_string_lossy().into_owned())).collect()?;
        Ok(listing)
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