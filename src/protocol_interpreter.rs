use crate::data_transfer_process::{DataTransferProcess, DataType, DataStructure, TransferMode, DataFormat};

use std::net::{TcpListener, TcpStream, IpAddr, ToSocketAddrs, SocketAddr, Ipv4Addr};
use std::io::{Result, Write, Read, Error, ErrorKind};
use std::io;
use std::time::{Duration};
use std::str::{FromStr, from_utf8};
use std::string::ToString;
use std::collections::HashMap;
use std::fmt::format;

use strum::EnumMessage;
use strum_macros::{Display, EnumString, EnumMessage};

//#[derive(PartialEq, Hash, Clone, Copy)]
#[derive(EnumMessage, PartialEq)]
enum Reply {
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

impl From<io::Error> for Reply {
    fn from(e: Error) -> Self {
        use io::ErrorKind::*;
        use Reply::*;
        log::error!("Error {}", e);

        match e.kind() {
            ConnectionRefused => CantOpenDataConnection,
            _ => LocalProcessingError
        }
    }
}

#[derive(EnumString)]
#[strum(ascii_case_insensitive)]
enum Command {
    // Implemented

    User(String),
    Pass(String),
    Quit,
    Port(u16),
    Type(DataType),
    Stru(DataStructure),
    Mode(TransferMode),
    Noop,
    Retr(String),
    Pasv,
    Nlst(Option<String>),

    // Not implemented

    Acct,
    Cwd,
    Cdup,
    Smnt,
    Rein,
    Stor,
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

impl Command {
    fn parse_line(s: &str) -> Result<Command> {
        use Command::*;

        let mut words = s.split(' ');
        let command = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?
            .parse::<Command>().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
        let command = match command {
            User(_) => {
                let username = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?;
                User(username.to_owned())
            }
            Pass(_) => {
                let pass = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?;
                Pass(pass.to_owned())
            }
            Port(_) => {
                let port: u16 = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?
                    .parse().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
                Port(port)
            }
            Type(_) => {
                let data_type: DataType = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?
                    .parse().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
                let data_type = match data_type {
                    DataType::ASCII(_) => {
                        let data_format: DataFormat = match words.next() {
                            Some(data_format) => {
                                data_format.parse()
                                    .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?
                            }
                            None => DataFormat::default()
                        };
                        DataType::ASCII(data_format)
                    }
                    DataType::EBCDIC(_) => {
                        let data_format: DataFormat = match words.next() {
                            Some(data_format) => {
                                data_format.parse()
                                    .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?
                            }
                            None => DataFormat::default()
                        };
                        DataType::EBCDIC(data_format)
                    }
                    DataType::Image => DataType::Image,
                    DataType::Local(_) => {
                        let byte_size: u8 = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?
                            .parse().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
                        DataType::Local(byte_size)
                    }
                };
                Type(data_type)
            }
            Stru(_) => {
                let data_structure: DataStructure = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?
                    .parse().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
                Stru(data_structure)
            }
            Mode(_) => {
                let mode: TransferMode = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?
                    .parse().map_err(|e| Error::new(ErrorKind::InvalidInput, e))?;
                Mode(mode)
            }
            Retr(_) => {
                let path = words.next().ok_or(Error::from(ErrorKind::InvalidInput))?;
                Pass(path.to_owned())
            }
            Nlst(_) => {
                let path = words.next().and_then(|x| Some(x.to_owned()));
                Nlst(path)
            }
            _ => command
        };
        Ok(command)
    }
}

pub struct Client {
    pub ip: IpAddr,
    pub data_port: u16,
    pub has_quit: bool,
    pub username: String,
    pub password: String,
    pub dtp: DataTransferProcess,

    stream: TcpStream
}

impl Client {
    fn new(stream: TcpStream) -> Client {
        let addr = stream.peer_addr().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(60))).unwrap();

        Client {
            ip: addr.ip(),
            data_port: addr.port(),
            has_quit: false,
            username: "anonymous".to_owned(),
            password: "anonymous".to_owned(),
            dtp: DataTransferProcess::new(".".to_owned()),

            stream
        }
    }

    fn send_message(&mut self, msg: &str) -> Result<()> {
        log::debug!("----> {}", msg);
        self.stream.write_all(msg.as_bytes())?;
        self.stream.write_all(CRLF.as_bytes())?;
        Ok(())
    }

    fn read_message(&mut self) -> Result<String> {
        //TODO: is it a right way to do it?
        //TODO: max message len
        let mut message = String::new();
        loop {
            let mut buf = [0 as u8; 256];
            let n = self.stream.read(&mut buf)?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Client shut connection"));
            }
            let new_text = from_utf8(&buf[0..n])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
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
        log::debug!("<---- {}", message);
        Ok(message)
    }

}

pub struct ProtocolInterpreter {}

const CRLF: &'static str = "\r\n";

impl ProtocolInterpreter {
    pub fn handle_client(&self, mut stream: TcpStream) -> Result<()> {
        //TODO: Get rid of this unwrap
        let mut client = Client::new(stream);
        log::info!("Got a new connection from {}", client.ip);
        client.send_message(Reply::ServiceReady.to_string().as_str())?;

        while !client.has_quit  {
            let message = client.read_message()?;
            let command = match Command::parse_line(message.as_str()) {
                Ok(command) => command,
                Err(_) => {
                    client.send_message(Reply::SyntaxError.to_string().as_str());
                    continue;
                }
            };
            let reply = Self::dispatch_command(command, &mut client);
            client.send_message(reply.to_string().as_str());
        }
        log::info!("Connection with client {} properly closed.", client.ip);
        Ok(())
    }

    fn dispatch_command(command: Command, client: &mut Client) -> Reply
    {
        match command {
            Command::Quit => Self::quit(client),
            Command::Port(port) => Self::port(client, port),
            Command::User(username) => Self::username(client, username),
            Command::Pass(pass) => Self::password(client, pass),
            Command::Mode(mode) => Self::mode(client, mode),
            Command::Stru(data_structure) => Self::stru(client, data_structure),
            Command::Type(data_type) => Self::type_(client, data_type),
            Command::Pasv => Self::pasv(client),
            Command::Retr(path) => Self::retr(client, path),
            Command::Nlst(path) => Self::nlist(client, path),
            _ => Reply::CommandOk
        }
    }

    fn quit(client: &mut Client) -> Reply {
        client.has_quit = true;
        Reply::ServiceClosing
    }

    fn parse_port(input: &str) -> Result<SocketAddr> {
        // TODO: Use some 3rd party map iterator that can fail and generally make this function less gross
        let mut nums = Vec::new();
        for num in input.split(',') {
            let num = match num.parse::<u8>() {
                Ok(num) => num,
                Err(e) => return Err(Error::new(ErrorKind::InvalidInput, e))
            };
            nums.push(num);
        }
        if nums.len() != 6 {
            return Err(Error::from(ErrorKind::InvalidInput));
        }
        let ip = IpAddr::V4(Ipv4Addr::new(nums[0], nums[1], nums[2], nums[3]));
        let port: u16 = ((nums[4] as u16) << 8) + nums[5] as u16;
        Ok(SocketAddr::new(ip, port))
    }

    fn port(client: &mut Client, port: u16) -> Reply {
        client.data_port = port;
        Reply::CommandOk
    }

    fn username(client: &mut Client, username: String) -> Reply
    {
        client.username = username;
        Reply::UsernameOk
    }

    fn password(client: &mut Client, pass: String) -> Reply
    {
        client.password = pass;
        Reply::UserLoggedIn
    }

    fn mode(client: &mut Client, mode: TransferMode) -> Reply {
        client.dtp.transfer_mode = mode;
        Reply::CommandOk
    }

    fn stru(client: &mut Client, data_structure: DataStructure) -> Reply {
        client.dtp.data_structure = data_structure;
        Reply::CommandOk
    }

    fn type_(client: &mut Client, data_type: DataType) -> Reply {
        client.dtp.data_type = data_type;
        Reply::CommandOk
    }

    fn pasv(client: &mut Client) -> Reply {
        let addr = match client.dtp.make_passive() {
            Ok(addr) => addr,
            Err(e) => return e.into()
        };
        let ip = match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(ip) => unreachable!() //TODO: it's gross
        };
        Reply::EnteringPassiveMode((ip, addr.port()))
    }

    fn retr(client: &mut Client, path: String) -> Reply {
        if let Err(e) = Self::connect_dtp(client) {
            return e.into();
        }
        if let Err(e) = client.dtp.send_file(path.as_str()) {
            return e.into();
        }
        Reply::FileActionSuccessful
    }

    fn nlist(client: &mut Client, path: Option<String>) -> Reply {
        if let Err(e) = Self::connect_dtp(client) {
            return e.into();
        }
        match client.dtp.send_dir_listing(path) {
            Ok(()) => Reply::DirectoryStatus,
            Err(e) => e.into()
        }
    }

    fn connect_dtp(client: &mut Client) -> Result<()> {
        if let Some(res) = client.dtp.connect(SocketAddr::new(client.ip, client.data_port)) {
            match res {
                Ok(_) => {
                    client.send_message(Reply::OpeningDataConnection.to_string().as_str())?;
                    Ok(())
                }
                Err(e) => Err(e)
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol_interpreter::Reply::CommandOk;
    use super::*;

    #[test]
    fn test_reply_string() {
        let reply = Reply::ServiceReady;
        assert_eq!(reply.to_string(), "220 Service ready");
    }

    #[test]
    fn test_command_from_string() {
        let command = Command::from_str("QuIt zabilem grubasa").unwrap();
        assert_eq!(command.command, Command::QUIT);
        assert_eq!(command.arg.unwrap(), "zabilem grubasa");

        let command = Command::from_str("");
        assert!(command.is_err());

        let command = Command::from_str("dupa kupa");
        assert!(command.is_err());

        let command = Command::from_str("LITWO ojczyzno moja ty jestes");
        assert!(command.is_err());

        let command = Command::from_str("QUIT");
        assert!(command.is_ok());
    }

    #[test]
    fn mode_from_arg() {
        let mode = "S".parse::<TransferMode>().unwrap();
    }
}
