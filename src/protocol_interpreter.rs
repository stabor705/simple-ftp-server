use crate::data_transfer_process::{DataTransferProcess, DataType, DataStructure, TransferMode, DataFormat};

use std::net::{TcpListener, TcpStream, IpAddr, ToSocketAddrs, SocketAddr};
use std::io::{Result, Write, Read, Error};
use std::io;
use std::time::{Duration};
use std::str::{FromStr, from_utf8};
use std::string::ToString;

use strum::EnumMessage;
use strum_macros::{Display, EnumString, EnumMessage};
use crate::protocol_interpreter::Reply::{CantOpenDataConnection, LocalProcessingError};

//#[derive(PartialEq, Hash, Clone, Copy)]
#[derive(EnumMessage, PartialEq, Clone, Copy)]
enum Reply {
    #[strum(message = "Command okay")]
    CommandOk = 200,
    #[strum(message = "Command not implemented, superfluous at this site")]
    CommandNotImplemented = 202,
    // 211
    #[strum(message = "Directory status")]
    DirectoryStatus,
    //214
    //215
    #[strum(message = "Service ready for new user")]
    ServiceReady = 220,
    #[strum(message = "Service closing control connection")]
    ServiceClosing = 221,
    #[strum(message = "Data connection open; no transfer in progress")]
    DataConnectionOpen = 225,
    #[strum(message = "Closing data connection. Requested file action successful")]
    FileActionSuccessful = 226,
    #[strum(message = "Entering passive mode")]
    EnteringPassiveMode = 227,
    #[strum(message = "User logged in, proceed")]
    UserLoggedIn = 230,
    #[strum(message = "Requested file action okay, proceed")]
    FileActionOk = 250,
    //257

    #[strum(message = "User name okay, need password")]
    UsernameOk = 331,
    //332
    #[strum(message = "Requested file action pending further information")]
    PendingFurtherInformation = 350,

    #[strum(message = "Service not available, closing control connection")]
    ServiceNotAvailable = 421,
    #[strum(message = "Can't open data connection")]
    CantOpenDataConnection = 425,
    #[strum(message = "Connection closed; transfer aborted")]
    ConnectionClosed = 426,
    #[strum(message = "Requested file action not taken. File unavailable")]
    FileActionNotTaken = 450,
    #[strum(message = "Requested action aborted: local error in processing")]
    LocalProcessingError = 451,
    #[strum(message = "Requested action not taken. Insufficient storage space in system")]
    InsufficientStorageSpace = 452,

    #[strum(message = "Syntax error, command unrecognized")]
    SyntaxError = 500,
    #[strum(message = "Syntax error in parameters or arguments")]
    SyntaxErrorArg = 501,
    #[strum(message = "Command not implemented")]
    NotImplemented = 502,
    #[strum(message = "Bad sequence of commands")]
    BadCommandSequence = 503,
    #[strum(message = "Command not implemented for that parameter")]
    BadParameter = 504,
    #[strum(message = "Not logged in")]
    NotLoggedIn = 530,
    #[strum(message = "Need account for storing files")]
    NeedAccountForStoring = 532,
    #[strum(message = "Requested action not taken. File unavailable")]
    FileUnavailable = 550,
    #[strum(message = "Requested action aborted: page type unknown")]
    PageTypeUnknown = 551,
    #[strum(message = "Requested file action aborted. Exceeded storage allocation")]
    ExceededStorageAllocation = 552,
    #[strum(message = "Requested action not taken. File name unknown")]
    FileNameUnknown = 553
}

impl From<io::Error> for Reply {
    fn from(e: Error) -> Self {
        use io::ErrorKind::*;

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
    // Not implemented

    Acct,
    Cwd,
    Cdup,
    Smnt,
    Rein,
    Pasv,
    Retr,
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

pub struct Context {
    pub client_ip: IpAddr,
    pub client_data_port: u16,
    pub server_data_port: u16,
    pub has_quit: bool,
    pub username: String,
    pub password: String,
    pub data_type: DataType,
    pub data_structure: DataStructure,
    pub transfer_mode: TransferMode
}

impl Context {
    fn new(addr: SocketAddr) -> Context {
        Context {
            client_ip: addr.ip(),
            client_data_port: addr.port(),
            server_data_port: 20,
            has_quit: false,
            username: "anonymous".to_owned(),
            password: "anonymous".to_owned(),
            data_type: DataType::ASCII(DataFormat::NonPrint),
            data_structure: DataStructure::FileStructure,
            transfer_mode: TransferMode::Stream
        }
    }
}

pub struct ProtocolInterpreter{
    dtp: DataTransferProcess
}

const CRLF: &'static str = "\r\n";

impl ProtocolInterpreter {
    pub fn new(dtp: DataTransferProcess) -> ProtocolInterpreter {
        ProtocolInterpreter { dtp }
    }

    pub fn run<A: ToSocketAddrs>(&mut self, addr: A) -> Result<()> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_new_connection(stream) {
                        log::error!("An error while handling connection: {:?}", e);
                    }
                }
                Err(e) => log::error!("An error occurred before connection took place: {}", e)
            }
        }
        Ok(())
    }

    fn handle_new_connection(&mut self, mut stream: TcpStream) -> Result<()> {
        //TODO: Get rid of this unwrap
        let peer_addr = stream.peer_addr().unwrap();
        log::info!("Got a new connection from {}", peer_addr);
        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        stream.set_write_timeout(Some(Duration::from_secs(60)))?;
        Self::send_reply(Reply::ServiceReady, &mut stream)?;

        let mut ctx = Context::new(peer_addr);
        while !ctx.has_quit  {
            let message = Self::read_message(&mut stream)?;
            let command = Command::from_str(message.as_str());
            let reply = match command {
                Ok(command) => self.dispatch_command(command, &mut ctx),
                Err(_) => Reply::SyntaxError
            };
            if reply == Reply::EnteringPassiveMode {
                let p1 = ctx.server_data_port >> 8;
                let p2 = ctx.server_data_port & 0b0000000011111111;
                let addr_info = format!("({},{},{},{},{},{})", 127, 0, 0, 1, p1, p2);
                Self::send_reply_with_parameter(reply, &mut stream, addr_info.as_str());
                self.dtp.connect(SocketAddr::new(ctx.client_ip, ctx.client_data_port));
            } else {
                Self::send_reply(reply, &mut stream)?;
            }
        }
        log::info!("Connection with client {} properly closed.", peer_addr);
        Ok(())
    }

    fn send_reply(reply: Reply, stream: &mut TcpStream) -> Result<()> {
        let text = format!("{} {}", reply as u32, reply.get_message().unwrap());
        log::debug!("----> {}", text);
        stream.write_all(text.as_bytes())?;
        stream.write_all(CRLF.as_bytes())?;
        Ok(())
    }

    fn send_reply_with_parameter(reply: Reply, stream: &mut TcpStream, parameter: &str) -> Result<()> {
        let text = format!("{} {} {}", reply as u32, reply.get_message().unwrap(), parameter);
        log::debug!("----> {}", text);
        stream.write_all(text.as_bytes())?;
        stream.write_all(CRLF.as_bytes())?;
        Ok(())
    }

    fn read_message(stream: &mut TcpStream) -> Result<String> {
        //TODO: is it a right way to do it?
        //TODO: max message len
        let mut message = String::new();
        loop {
            let mut buf = [0 as u8; 256];
            let n = stream.read(&mut buf)?;
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

    fn dispatch_command(&mut self, command: Command, ctx: &mut Context) -> Reply {
        let args = &command.args;
        match command.instruction {
            Instruction::Quit => Self::quit(ctx),
            Instruction::Noop => Reply::CommandOk,
            Instruction::Port => Self::port(args, ctx),
            Instruction::User => Self::username(args, ctx),
            Instruction::Pass => Self::password(args, ctx),
            Instruction::Type => Self::type_(args, ctx),
            Instruction::Stru => Self::stru(args, ctx),
            Instruction::Mode => Self::mode(args, ctx),
            Instruction::Pasv => self.pasv(args, ctx),
            _ => Reply::CommandNotImplemented
        }
    }

    fn quit(ctx: &mut Context) -> Reply {
        ctx.has_quit = true;
        Reply::ServiceClosing
    }

    fn port(args: &Arguments, ctx: &mut Context) -> Reply
    {
        ctx.client_data_port = match args.get_arg(0) {
            Ok(data_port) => data_port,
            Err(e) => return e.into()
        };
        Reply::CommandOk
    }

    fn username(args: &Arguments, ctx: &mut Context) -> Reply
    {
        ctx.username = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::UsernameOk
    }

    fn password(args: &Arguments, ctx: &mut Context) -> Reply
    {
        ctx.password = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::UserLoggedIn
    }

    fn mode(args: &Arguments, ctx: &mut Context) -> Reply
    {
        ctx.transfer_mode = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::CommandOk
    }

    fn stru(args: &Arguments, ctx: &mut Context) -> Reply
    {
        ctx.data_structure = match args.get_arg(0) {
            Ok(x) => x,
            Err(e) => return e.into()
        };
        Reply::CommandOk
    }

    fn type_(args: &Arguments, ctx: &mut Context) -> Reply {
        let data_type = match args.get_arg(0) {
            Ok(data_type) => data_type,
            Err(e) => return e.into()
        };
        match data_type {
            //TODO: much them both at the same time?
            DataType::ASCII(_) => {
                let data_format = match args.get_optional_arg(1) {
                    Ok(data_format) => data_format,
                    Err(e) => return e.into()
                };
                ctx.data_type = DataType::ASCII(data_format);
            }
            DataType::EBCDIC(_) => {
                let data_format = match args.get_optional_arg(1) {
                    Ok(data_format) => data_format,
                    Err(e) => return e.into()
                };
                ctx.data_type = DataType::ASCII(data_format);
            }
            DataType::Image => ctx.data_type = DataType::Image,
            DataType::Local(_) => {
                let byte_size = match args.get_arg(1) {
                    Ok(byte_size) => byte_size,
                    Err(e) => return e.into()
                };
                ctx.data_type = DataType::Local(byte_size);
            }
        }
        Reply::CommandOk
    }

    fn pasv(&mut self, args: &Arguments, ctx: &mut Context) -> Reply {
        ctx.server_data_port = match self.dtp.make_passive() {
            Ok(port) => port,
            Err(e) => return e.into()
        };
        Reply::EnteringPassiveMode
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