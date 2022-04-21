use crate::data_transfer_process::{DataTransferProcess, DataType, DataStructure, TransferMode, DataFormat, DataRepr};

use std::net::{TcpStream, IpAddr, SocketAddr, Ipv4Addr};
use std::io::{Write, Read};
use std::time::{Duration};
use std::str::from_utf8;
use std::string::ToString;
use std::fmt::{Debug, Display, Formatter};

use strum::EnumMessage;
use strum_macros::{EnumString, EnumMessage};
use fallible_iterator::FallibleIterator;
use anyhow::{Result, Error};

#[allow(dead_code)]
#[derive(EnumMessage, PartialEq)]
pub enum Reply {
    #[strum(message = "Opening data connection")]
    OpeningDataConnection,

    #[strum(message = "Command okay")]
    CommandOk,
    #[strum(message = "Command not implemented, superfluous at this site")]
    CommandNotImplemented,
    // 211
    #[strum(message = "Directory status")]
    DirectoryStatus,
    //214
    //215
    #[strum(message = "Service ready for new user")]
    ServiceReady,
    #[strum(message = "Service closing control connection")]
    ServiceClosing,
    #[strum(message = "Data connection open; no transfer in progress")]
    DataConnectionOpen,
    #[strum(message = "Closing data connection. Requested file action successful")]
    FileActionSuccessful,
    #[strum(message = "Entering passive mode ({})")]
    EnteringPassiveMode((Ipv4Addr, u16)),
    #[strum(message = "User logged in, proceed")]
    UserLoggedIn,
    #[strum(message = "Requested file action okay, proceed")]
    FileActionOk,
    #[strum(message = "\"{}\" created")]
    Created(String),

    #[strum(message = "User name okay, need password")]
    UsernameOk,
    //332
    #[strum(message = "Requested file action pending further information")]
    PendingFurtherInformation,

    #[strum(message = "Service not available, closing control connection")]
    ServiceNotAvailable,
    #[strum(message = "Can't open data connection")]
    CantOpenDataConnection,
    #[strum(message = "Connection closed; transfer aborted")]
    ConnectionClosed,
    #[strum(message = "Requested file action not taken. File unavailable")]
    FileActionNotTaken,
    #[strum(message = "Requested action aborted: local error in processing")]
    LocalProcessingError,
    #[strum(message = "Requested action not taken. Insufficient storage space in system")]
    InsufficientStorageSpace,

    #[strum(message = "Syntax error, command unrecognized")]
    SyntaxError,
    #[strum(message = "Syntax error in parameters or arguments")]
    SyntaxErrorArg,
    #[strum(message = "Command not implemented")]
    NotImplemented,
    #[strum(message = "Bad sequence of commands")]
    BadCommandSequence,
    #[strum(message = "Command not implemented for that parameter")]
    BadParameter,
    #[strum(message = "Not logged in")]
    NotLoggedIn,
    #[strum(message = "Need account for storing files")]
    NeedAccountForStoring,
    #[strum(message = "Requested action not taken. File unavailable")]
    FileUnavailable,
    #[strum(message = "Requested action aborted: page type unknown")]
    PageTypeUnknown,
    #[strum(message = "Requested file action aborted. Exceeded storage allocation")]
    ExceededStorageAllocation,
    #[strum(message = "Requested action not taken. File name unknown")]
    FileNameUnknown,
}

impl Reply {
    fn status_code(&self) -> u32 {
        use Reply::*;
        match self {
            OpeningDataConnection => 150,

            CommandOk => 200,
            CommandNotImplemented => 202,
            // 211
            DirectoryStatus => 212,
            //214
            //215
            ServiceReady => 220,
            ServiceClosing => 221,
            DataConnectionOpen => 225,
            FileActionSuccessful => 226,
            EnteringPassiveMode(_) => 227,
            UserLoggedIn => 230,
            FileActionOk => 250,
            Created(_) => 257,

            UsernameOk => 331,
            //332
            PendingFurtherInformation => 350,

            ServiceNotAvailable => 421,
            CantOpenDataConnection => 425,
            ConnectionClosed => 426,
            FileActionNotTaken => 450,
            LocalProcessingError => 451,
            InsufficientStorageSpace => 452,

            SyntaxError => 500,
            SyntaxErrorArg => 501,
            NotImplemented => 502,
            BadCommandSequence => 503,
            BadParameter => 504,
            NotLoggedIn => 530,
            NeedAccountForStoring => 532,
            FileUnavailable => 550,
            PageTypeUnknown => 551,
            ExceededStorageAllocation => 552,
            FileNameUnknown => 553,
        }
    }
}

impl ToString for Reply {
    fn to_string(&self) -> String {
        use Reply::*;
        let response = format!("{} {}", self.status_code(), self.get_message().unwrap());
        match self {
            EnteringPassiveMode((ip, port)) => {
                let h = ip.octets();
                let p1 = port >> 8;
                let p2 = port & 0b0000000011111111;
                response.replace("{}", format!("{},{},{},{},{},{}", h[0], h[1], h[2], h[3], p1, p2).as_str())
            }
            Created(pathname) => response.replace("{}", pathname),
            _ => response
        }
    }
}

impl From<Error> for Reply {
    fn from(e: Error) -> Self {
        use Reply::*;

        if e.is::<ArgError>() {
            SyntaxErrorArg
        } else if e.is::<std::io::Error>() {
            let error: std::io::Error = e.downcast().unwrap();
            match error {
                _ => {
                    log::error!("Encountered unexpected io error {}", error);
                    LocalProcessingError
                }
            }
        } else {
            log::error!("Encountered unexpected error {}", e);
            LocalProcessingError
        }
    }
}

#[derive(EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive)]
pub enum Command {
    // Implemented

    User(String),
    Pass(String),
    Quit,
    Port(([u8; 4], u16)),
    Type(DataType),
    Stru(DataStructure),
    Mode(TransferMode),
    Noop,
    Retr(String),
    Pasv,
    Nlst(Option<String>),
    Stor(String),

    // Not implemented

    Acct,
    Cwd,
    Cdup,
    Smnt,
    Rein,
    Stou,
    Appe,
    Allo,
    Rest,
    Rnfr,
    Rnto,
    Abor,
    Dele,
    Rmd,
    Mkd,
    Pwd,
    List,
    Site,
    Syst,
    Stat,
    Help,
}

#[derive(Debug)]
enum ArgError {
    ArgMissing,
    BadArg
}

impl Display for ArgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ArgError::*;

        match *self {
            ArgMissing => write!(f, "missing required argument"),
            BadArg => write!(f, "invalid format of provided argument")
        }
    }
}

impl std::error::Error for ArgError {}

impl Command {
    fn parse_line(s: &str) -> Result<Command> {
        use Command::*;

        let mut words = s.split(' ');
        let command = words.next().ok_or(ArgError::ArgMissing)?
            .parse::<Command>()?;
        let command = match command {
            User(_) => {
                let username = words.next().ok_or(ArgError::ArgMissing)?;
                User(username.to_owned())
            }
            Pass(_) => {
                let pass = words.next().ok_or(ArgError::ArgMissing)?;
                Pass(pass.to_owned())
            }
            Port(_) => {
                let b: Vec<u8> = fallible_iterator::convert(
                    s.split(',').map(|c| c.parse::<u8>())
                ).collect()?;
                if b.len() < 6 {
                    return Err(Error::new(ArgError::BadArg));
                }
                let mut ip: [u8; 4] = [0; 4];
                ip.clone_from_slice(&b[0..4]);
                let port = ((b[4] as u16) << 8) + b[5] as u16;
                Port((ip, port))
            }
            Type(_) => {
                let data_type: DataType = words.next().ok_or(ArgError::ArgMissing)?
                    .parse()?;
                let data_type = match data_type {
                    DataType::ASCII(_) => {
                        let data_format: DataFormat = match words.next() {
                            Some(data_format) => {
                                data_format.parse()?
                            }
                            None => DataFormat::default()
                        };
                        DataType::ASCII(data_format)
                    }
                    DataType::EBCDIC(_) => {
                        let data_format: DataFormat = match words.next() {
                            Some(data_format) => {
                                data_format.parse()?
                            }
                            None => DataFormat::default()
                        };
                        DataType::EBCDIC(data_format)
                    }
                    DataType::Image => DataType::Image,
                    DataType::Local(_) => {
                        let byte_size: u8 = words.next().ok_or(ArgError::ArgMissing)?
                            .parse()?;
                        DataType::Local(byte_size)
                    }
                };
                Type(data_type)
            }
            Stru(_) => {
                let data_structure: DataStructure = words.next().ok_or(ArgError::ArgMissing)?
                    .parse()?;
                Stru(data_structure)
            }
            Mode(_) => {
                let mode: TransferMode = words.next().ok_or(ArgError::ArgMissing)?
                    .parse()?;
                Mode(mode)
            }
            Retr(_) => {
                let path = words.next().ok_or(ArgError::ArgMissing)?;
                Pass(path.to_owned())
            }
            Stor(_) => {
                let path = words.next().ok_or(ArgError::ArgMissing)?;
                Stor(path.to_owned())
            }
            Nlst(_) => {
                let path = words.next().and_then(|x| Some(x.to_owned()));
                Nlst(path)
            }
            _ => command
        };
        Ok(command)
    }

    pub fn to_line(&self) -> String {
        use Command::*;
        match self {
            User(username) => format!("{} {}", self.to_string(), username),
            Pass(pass) => format!("{} {}", self.to_string(), pass),
            Port((ip, port)) => {
                let p1 = port >> 8;
                let p2 = port & 0b0000000011111111;
                format!("{} ({},{},{},{},{},{})", self.to_string(), ip[0], ip[1], ip[2], ip[3], p1, p2)
            }
            Type(data_type) => format!("{} {}", self.to_string(), data_type),
            Stru(data_structure) => format!("{} {}", self.to_string(), data_structure),
            Mode(transfer_mode) => format!("{} {}", self.to_string(), transfer_mode),
            Retr(path) => format!("{} {}", self.to_string(), path),
            Nlst(path) => match path {
                Some(path) => format!("{} {}", self.to_string(), path),
                None => self.to_string()
            }
            Stor(path) => format!("{} {}", self.to_string(), path),
            _ => self.to_string()
        }
    }
}

pub struct CrlfStream {
    stream: TcpStream
}

const CRLF: &'static str = "\r\n";

impl CrlfStream {

    pub fn new(stream: TcpStream) -> CrlfStream {
        CrlfStream { stream }
    }

    pub fn send_message(&mut self, msg: &str) -> Result<()> {
        self.stream.write_all(msg.as_bytes())?;
        self.stream.write_all(CRLF.as_bytes())?;
        Ok(())
    }

    pub fn read_message(&mut self) -> Result<String> {
        //TODO: is it a right way to do it?
        //TODO: max message len
        let mut message = String::new();
        loop {
            let mut buf = [0 as u8; 256];
            let n = self.stream.read(&mut buf)?;
            if n == 0 {
                return Err(Error::msg("Client shut connection"));
            }
            //TODO: I don't think that I want to use utf8 here
            let new_text = from_utf8(&buf[0..n])?;
            match new_text.find(CRLF) {
                None => message.push_str(new_text),
                Some(pos) => {
                    message.push_str(&new_text[0..pos]);
                    if pos != new_text.len() - 2 {
                        log::warn!("A part of some command has been discarded: {}", new_text);
                    }
                    break;
                }
            }
        }
        Ok(message)
    }
}

pub struct Client {
    pub ip: IpAddr,
    pub data_port: u16,
    pub has_quit: bool,
    pub username: String,
    pub password: String,
    pub data_repr: DataRepr,

    stream: CrlfStream
}

impl Client {
    pub fn new(stream: TcpStream) -> Client {
        let addr = stream.peer_addr().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(60))).unwrap();

        Client {
            ip: addr.ip(),
            data_port: addr.port(),
            has_quit: false,
            username: "anonymous".to_owned(),
            password: "anonymous".to_owned(),
            data_repr: DataRepr::default(),

            stream: CrlfStream::new(stream)
        }
    }

    pub fn send_reply(&mut self, reply: Reply) -> Result<()> {
        let msg = reply.to_string();
        log::debug!("----> {}", msg);
        self.stream.send_message(msg.as_str())?;
        Ok(())
    }

    pub fn read_command(&mut self) -> Result<Command> {
        let msg = self.stream.read_message()?;
        log::debug!("<---- {}", msg);
        let command = Command::parse_line(msg.as_str())?;
        Ok(command)
    }

}

pub struct ProtocolInterpreter<'a> {
    dtp: &'a mut DataTransferProcess
}


impl<'a> ProtocolInterpreter<'a> {
    pub fn new(dtp: &mut DataTransferProcess) -> ProtocolInterpreter {
        ProtocolInterpreter { dtp }
    }

    pub fn handle_client(&mut self, stream: TcpStream) -> Result<()> {
        //TODO: Get rid of this unwrap
        let mut client = Client::new(stream);
        log::info!("Got a new connection from {}", client.ip);
        client.send_reply(Reply::ServiceReady)?;

        while !client.has_quit  {
            let command = match client.read_command() {
                Ok(command) => command,
                Err(_) => {
                    client.send_reply(Reply::SyntaxError)?;
                    continue;
                }
            };
            let reply = match self.dispatch_command(command, &mut client) {
                Ok(reply) => reply,
                Err(e) => {
                    log::warn!("{}", e);
                    e.into()
                }
            };
            client.send_reply(reply);
        }
        log::info!("Connection with client {} properly closed.", client.ip);
        Ok(())
    }

    fn dispatch_command(&mut self, command: Command, client: &mut Client) -> Result<Reply>
    {
        match command {
            Command::Quit => Self::quit(client),
            Command::Port(host_port) => Self::port(client, host_port),
            Command::User(username) => Self::username(client, username),
            Command::Pass(pass) => Self::password(client, pass),
            Command::Mode(mode) => Self::mode(client, mode),
            Command::Stru(data_structure) => Self::stru(client, data_structure),
            Command::Type(data_type) => Self::type_(client, data_type),
            Command::Pasv => self.pasv(client),
            Command::Retr(path) => self.retr(client, path),
            Command::Nlst(path) => self.nlist(client, path),
            Command::Stor(path) => self.stor(client, path),
            _ => Ok(Reply::CommandOk)
        }
    }

    fn quit(client: &mut Client) -> Result<Reply> {
        client.has_quit = true;
        Ok(Reply::ServiceClosing)
    }

    fn port(client: &mut Client, host_port: ([u8; 4], u16)) -> Result<Reply> {
        client.data_port = host_port.1;
        Ok(Reply::CommandOk)
    }

    fn username(client: &mut Client, username: String) -> Result<Reply>
    {
        client.username = username;
        Ok(Reply::UsernameOk)
    }

    fn password(client: &mut Client, pass: String) -> Result<Reply>
    {
        client.password = pass;
        Ok(Reply::UserLoggedIn)
    }

    fn mode(client: &mut Client, mode: TransferMode) -> Result<Reply> {
        client.data_repr.transfer_mode = mode;
        Ok(Reply::CommandOk)
    }

    fn stru(client: &mut Client, data_structure: DataStructure) -> Result<Reply> {
        client.data_repr.data_structure = data_structure;
        Ok(Reply::CommandOk)
    }

    fn type_(client: &mut Client, data_type: DataType) -> Result<Reply> {
        client.data_repr.data_type = data_type;
        Ok(Reply::CommandOk)
    }

    fn pasv(&mut self, client: &mut Client) -> Result<Reply> {
        let addr = self.dtp.make_passive()?;
        let ip = match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(ip) => unreachable!() //TODO: it's gross
        };
        Ok(Reply::EnteringPassiveMode((ip, addr.port())))
    }

    fn retr(&mut self, client: &mut Client, path: String) -> Result<Reply> {
        self.connect_dtp(client)?;
        self.dtp.send_file(path.as_str())?;
        Ok(Reply::FileActionSuccessful)
    }

    fn stor(&mut self, client: &mut Client, path: String) -> Result<Reply> {
        self.connect_dtp(client)?;
        self.dtp.receive_file(path.as_str());
        Ok(Reply::FileActionSuccessful)
    }

    fn nlist(&mut self, client: &mut Client, path: Option<String>) -> Result<Reply> {
        self.connect_dtp(client)?;
        self.dtp.send_dir_listing(path)?;
        Ok(Reply::DirectoryStatus)
    }

    fn connect_dtp(&mut self, client: &mut Client) -> Result<()> {
        if let Some(res) = self.dtp.connect(SocketAddr::new(client.ip, client.data_port)) {
            match res {
                Ok(_) => {
                    client.send_reply(Reply::OpeningDataConnection)?;
                    Ok(())
                }
                Err(e) => Err(Error::new(e))
            }
        } else {
            Ok(())
        }
    }
}

#[allow(unused_imports)] // For some reason compiler thinks super::* is not use
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reply_creation() {
        let reply = Reply::CommandOk;
        assert_eq!(reply.to_string(), "200 Command okay");
        let reply = Reply::EnteringPassiveMode((Ipv4Addr::new(127, 0, 0, 1), 8888));
        assert_eq!(reply.to_string(), "227 Entering passive mode (127,0,0,1,34,184)");
        let reply = Reply::Created("very-important-directory".to_owned());
        assert_eq!(reply.to_string(), "257 \"very-important-directory\" created")
    }
}
