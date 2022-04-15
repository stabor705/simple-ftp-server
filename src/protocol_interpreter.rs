use std::fmt::format;
use std::fs::read;
use crate::data_transfer_process::{DataTransferProcess, DataType, DataStructure, TransferMode, DataFormat};

use std::net::{TcpListener, TcpStream, IpAddr, ToSocketAddrs, SocketAddr, Ipv4Addr};
use std::io::{Result, Write, Read, Error, ErrorKind};
use std::io;
use std::time::{Duration};
use std::str::{FromStr, from_utf8};
use std::string::ToString;

use strum::EnumMessage;
use strum_macros::{Display, EnumString, EnumMessage};
use crate::protocol_interpreter::Reply::NoReply;

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

    NoReply
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

            NoReply => unreachable!()
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

#[derive(Display, EnumString, PartialEq, Debug)]
#[strum(ascii_case_insensitive)]
enum Instruction {
    // Implemented

    User,
    Pass,
    Quit,
    Port,
    Type,
    Stru,
    Mode,
    Noop,
    Retr,
    Pasv,

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
    Nlst,
    Site,
    Syst,
    Stat,
    Help,
}

struct Command {
    pub instruction: Instruction,
    pub args: Arguments
}

struct Arguments {
    args: Vec<String>
}

impl Arguments {
    fn get_arg<V>(&self, idx: usize) -> std::result::Result<V, ArgError>
        where V: FromStr
    {
        self.args.get(idx).ok_or(ArgError::ExpectedArgument)?
            .parse::<V>().map_err(|_| ArgError::BadArgument)
    }

    fn get_optional_arg<V>(&self, idx: usize) -> std::result::Result<V, ArgError>
        where V: FromStr + Default
    {
        match self.args.get(idx) {
            Some(arg) => {
                match arg.parse::<V>() {
                    Ok(arg) => Ok(arg),
                    Err(_) => Err(ArgError::BadArgument)
                }
            }
            None => Ok(V::default())
        }
    }
}

enum ArgError {
    ExpectedArgument,
    BadArgument
}

impl From<ArgError> for Reply {
    fn from(arg_error: ArgError) -> Self {
        use ArgError::*;
        match arg_error {
            ExpectedArgument => Reply::SyntaxErrorArg,
            BadArgument => Reply::SyntaxErrorArg
        }
    }
}

impl FromStr for Command {
    type Err = io::Error;
    fn from_str(string: &str) -> Result<Command> {
        let (instruction, args) = match string.split_once(' ') {
            Some((command, args)) => (command, Some(args)),
            None => (string, None)
        };
        let instruction = instruction.parse::<Instruction>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let args: Vec<String> = match args {
            Some(args) => args.split(' ').map(| x | x.to_owned()).collect(),
            None => Vec::new()
        };
        Ok(Command { instruction, args: Arguments { args } })
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
            let command = match Command::from_str(message.as_str()) {
                Ok(command) => command,
                Err(_) => {
                    client.send_message(Reply::SyntaxError.to_string().as_str());
                    continue;
                }
            };
            let args = &command.args;
            for action in Self::dispatch_command(&command) {
                let reply = action(args, &mut client);
                if reply != Reply::NoReply {
                    client.send_message(reply.to_string().as_str());
                }
            }
        }
        log::info!("Connection with client {} properly closed.", client.ip);
        Ok(())
    }

    fn dispatch_command(command: &Command) -> Vec<fn(&Arguments, &mut Client) -> Reply>
    {
        match command.instruction {
            Instruction::Quit => vec![Self::quit],
            Instruction::Noop => vec![Self::noop] ,
            Instruction::Port => vec![Self::port],
            Instruction::User => vec![Self::username],
            Instruction::Pass => vec![Self::password],
            Instruction::Type => vec![Self::type_],
            Instruction::Stru => vec![Self::stru],
            Instruction::Mode => vec![Self::mode],
            Instruction::Pasv => vec![Self::pasv],
            Instruction::Retr => vec![Self::connect_dtp, Self::retr],
            _ => vec![Self::not_implemented]
        }
    }

    fn noop(args: &Arguments, client: &mut Client) -> Reply {
        Reply::CommandOk
    }

    fn not_implemented(args: &Arguments, client: &mut Client) -> Reply {
        Reply::CommandNotImplemented
    }

    fn quit(args: &Arguments, client: &mut Client) -> Reply {
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

    fn port(args: &Arguments, client: &mut Client) -> Reply
    {
        let input: String = match args.get_arg(0) {
            Ok(input) => input,
            Err(e) => return e.into()
        };
        let addr = match Self::parse_port(input.as_str()) {
            Ok(addr) => addr,
            Err(e) => return e.into()
        };
        client.data_port = addr.port();
        Reply::CommandOk
    }

    fn username(args: &Arguments, client: &mut Client) -> Reply
    {
        client.username = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::UsernameOk
    }

    fn password(args: &Arguments, client: &mut Client) -> Reply
    {
        client.password = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::UserLoggedIn
    }

    fn mode(args: &Arguments, client: &mut Client) -> Reply
    {
        client.dtp.transfer_mode = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::CommandOk
    }

    fn stru(args: &Arguments, client: &mut Client) -> Reply
    {
        client.dtp.data_structure = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::CommandOk
    }

    fn type_(args: &Arguments, client: &mut Client) -> Reply {
        let data_type = match args.get_arg(0) {
            Ok(data_type) => data_type,
            Err(e) => return e.into()
        };
        match data_type {
            //TODO: match them both at the same time?
            DataType::ASCII(_) => {
                let data_format = match args.get_optional_arg(1) {
                    Ok(data_format) => data_format,
                    Err(e) => return e.into()
                };
                client.dtp.data_type = DataType::ASCII(data_format);
            }
            DataType::EBCDIC(_) => {
                let data_format = match args.get_optional_arg(1) {
                    Ok(data_format) => data_format,
                    Err(e) => return e.into()
                };
                client.dtp.data_type = DataType::ASCII(data_format);
            }
            DataType::Image => client.dtp.data_type = DataType::Image,
            DataType::Local(_) => {
                let byte_size = match args.get_arg(1) {
                    Ok(byte_size) => byte_size,
                    Err(e) => return e.into()
                };
                client.dtp.data_type = DataType::Local(byte_size);
            }
        }
        Reply::CommandOk
    }

    fn pasv(args: &Arguments, client: &mut Client) -> Reply {
        let addr = match client.dtp.make_passive() {
            Ok(addr) => addr,
            Err(e) => return e.into()
        };
        let ip = match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(ip) => unreachable!()
        };
        Reply::EnteringPassiveMode((ip, addr.port()))
    }

    fn retr(args: &Arguments, client: &mut Client) -> Reply {
        let path: String = match args.get_arg(0) {
            Ok(path) => path,
            Err(e) => return e.into()
        };
        if let Err(e) = client.dtp.send_file(path.as_str(), SocketAddr::new(client.ip, client.data_port)) {
            return e.into();
        }
        Reply::FileActionSuccessful
    }

    fn connect_dtp(args: &Arguments, client: &mut Client) -> Reply {
        match client.dtp.connect(SocketAddr::new(client.ip, client.data_port)) {
            None => NoReply,
            Some(res) => match res {
                Ok(()) => Reply::OpeningDataConnection,
                Err(e) => e.into()
            }
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
        assert_eq!(command.command, Instruction::QUIT);
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
