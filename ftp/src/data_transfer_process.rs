use std::fs::*;
use std::io::{Error, ErrorKind, Result, Write, copy};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

use fallible_iterator::FallibleIterator;
use path_dedot::ParseDot;
use strum_macros::{Display, EnumString};

#[derive(Display, EnumString)]
pub enum DataType {
    #[strum(serialize = "A")]
    ASCII(DataFormat),
    #[strum(serialize = "E")]
    EBCDIC(DataFormat),
    #[strum(serialize = "I")]
    Image,
    #[strum(serialize = "L")]
    Local(u8),
}

impl Default for DataType {
    fn default() -> Self {
        DataType::ASCII(DataFormat::default())
    }
}

#[derive(Display, EnumString)]
pub enum DataFormat {
    #[strum(serialize = "N")]
    NonPrint,
    #[strum(serialize = "T")]
    TelnetFormatEffectors,
    #[strum(serialize = "C")]
    CarriageControl,
}

impl Default for DataFormat {
    fn default() -> Self {
        Self::NonPrint
    }
}

#[derive(Display, EnumString)]
pub enum DataStructure {
    #[strum(serialize = "F")]
    FileStructure,
    #[strum(serialize = "R")]
    RecordStructure,
    #[strum(serialize = "P")]
    PageStructure,
}

impl Default for DataStructure {
    fn default() -> Self {
        DataStructure::FileStructure
    }
}

#[derive(Display, EnumString)]
pub enum TransferMode {
    #[strum(serialize = "S")]
    Stream,
    #[strum(serialize = "B")]
    Block,
    #[strum(serialize = "C")]
    Compressed,
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
    pub transfer_mode: TransferMode,
}

pub struct DataTransferProcess {
    root: PathBuf,
    working_dir: PathBuf,
    conn_timeout: Duration,
    mode: Box<dyn Mode + Sync + Send>,
    client: Option<TcpStream>,
    renaming_from: Option<PathBuf>,
}

impl DataTransferProcess {
    pub fn new(root: String, conn_timeout: Duration) -> DataTransferProcess {
        DataTransferProcess {
            root: PathBuf::from(root),
            working_dir: PathBuf::from("/"),
            conn_timeout,
            mode: Box::new(Active {}),
            client: None,
            renaming_from: None,
        }
    }

    pub fn make_passive(&mut self) -> Result<SocketAddr> {
        let passive = Passive::new(self.conn_timeout)?;
        let addr = passive.addr()?;
        self.mode = Box::new(passive);
        log::info!("DTP started listening on port {}", addr);
        Ok(addr)
    }

    pub fn connect(&mut self, addr: SocketAddr) -> Result<()> {
        if self.client.is_some() {
            panic!("Tried opening data connection with one already opened.");
            // Which means a problem with code logic. That makes it unrecoverable
            // error to me.
        }
        self.client = Some(self.mode.connect(addr)?);
        Ok(())
    }

    fn build_path<P: AsRef<Path>>(&self, rel_path: P) -> Result<PathBuf> {
        if rel_path.as_ref().is_absolute() {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        // Unfortunately this workaround is needed, since path_dedot
        // requires absolute path in order to not go up in directory hierarchy
        // beyond root directory, but, on the other hand, Path::join used with
        // absolute path as argument will just return the argument.
        // Thus, we have to use path with "/" at beginning with working_dir
        // in order to path_dedot work correctly, but get rid of it when
        // joining with root directory path.
        // TODO: It can be done properly by creating needed functions
        // instead of relying on libraries
        // TODO: It probably needs to return error when trying to go out of
        // root ("/.." for instance) instead of silently not changing state
        let rhs: PathBuf = self
            .working_dir
            .join(rel_path)
            .parse_dot()?
            .iter()
            .skip(1)
            .collect();
        Ok(self.root.join(rhs))
    }

    pub fn send_file(&mut self, path: &str) -> Result<()> {
        let mut client = self
            .client
            .take()
            .ok_or(Error::from(ErrorKind::NotConnected))?;
        let path = self.build_path(path)?;
        let mut file = File::open(path)?;
        copy(&mut file, &mut client)?;
        Ok(())
    }

    pub fn receive_file(&mut self, path: &str) -> Result<()> {
        let mut client = self
            .client
            .take()
            .ok_or(Error::from(ErrorKind::NotConnected))?;
        let path = self.build_path(path)?;
        let mut file = File::create(path)?;
        copy(&mut client, &mut file)?;
        Ok(())
    }

    pub fn send_dir_nlisting(&mut self, path: Option<String>) -> Result<()> {
        let mut client = self
            .client
            .take()
            .ok_or(Error::from(ErrorKind::NotConnected))?;
        let listing = self.get_dir_listing(&path.unwrap_or("".to_string()))?;
        for filename in listing {
            client.write_all(filename.as_bytes())?;
            client.write_all("\r\n".as_bytes())?;
        }
        Ok(())
    }

    fn get_dir_listing(&self, path: &str) -> Result<Vec<String>> {
        let dir = self.build_path(path)?;
        let listing = fallible_iterator::convert(read_dir(dir)?)
            .map(|entry| Ok(entry.file_name().to_string_lossy().into_owned()))
            .collect()?;
        Ok(listing)
    }

    pub fn get_working_dir(&self) -> String {
        self.working_dir.to_string_lossy().to_string()
    }

    pub fn change_working_dir(&mut self, path: &str) -> Result<()> {
        let new_path = self.build_path(path)?;
        if !new_path.exists() {
            return Err(Error::from(ErrorKind::NotFound));
        }
        self.working_dir = self.working_dir.join(path).parse_dot()?.into_owned();
        Ok(())
    }

    pub fn make_dir(&self, path: &str) -> Result<()> {
        create_dir(self.build_path(path)?)?;
        Ok(())
    }

    pub fn delete_file(&self, path: &str) -> Result<()> {
        remove_file(self.build_path(path)?)?;
        Ok(())
    }

    pub fn prepare_rename(&mut self, from: &str) -> Result<()> {
        let from = self.build_path(from)?;
        if !from.exists() {
            return Err(Error::from(ErrorKind::NotFound));
        }
        self.renaming_from = Some(from);
        Ok(())
    }

    pub fn rename(&mut self, to: &str) -> Result<()> {
        //TODO: use custom error
        let from = self.renaming_from.take().ok_or(Error::new(
            ErrorKind::InvalidData,
            "Tried renaming file without specifying renaming_from path",
        ))?;
        let to = self.build_path(to)?;
        rename(from, to)?;
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
    timeout: Duration,
}

impl Passive {
    pub fn new(timeout: Duration) -> Result<Passive> {
        Ok(Passive {
            listener: TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?,
            timeout,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr> {
        //TODO: I don't know why this function can error. Gotta get rid of this unwrap someday.
        self.listener.local_addr()
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
                        return Ok(stream);
                    } else {
                        log::warn!(
                            "Dropping connection from {}. Unexpected ip address.",
                            in_addr
                        );
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
