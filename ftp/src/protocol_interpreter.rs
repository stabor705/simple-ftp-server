use crate::data_transfer_process::{DataTransferProcess, DataType, DataStructure, TransferMode, DataRepr};

use std::net::{TcpStream, IpAddr, SocketAddr, Ipv4Addr};
use std::io;
use std::io::{Write, Read};
use std::time::{Duration};
use std::str::from_utf8;
use std::string::ToString;

use crate::Reply;
use crate::Command;
use crate::HostPort;

use anyhow::{Result, Error};

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
            let mut buf = [0 as u8; 8192];
            let n = self.stream.read(&mut buf)?;
            if n == 0 {
                return Err(Error::new(io::Error::from(io::ErrorKind::ConnectionAborted,
                                                      "Client quit unexpectedly.")));
            }
            //TODO:
            //There are absolutely no guarantees about fragmentation of data received.
            //Thus, this approach is not correct. What if CR is send first, and then LF in other
            //packet? Gonna fix is after moving to non_utf8 string
            let new_text = from_utf8(&buf[0..n]).unwrap();
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
    pub ip: Ipv4Addr,
    pub data_port: u16,
    pub has_quit: bool,
    pub username: String,
    pub password: String,
    pub data_repr: DataRepr,

    stream: CrlfStream
}

impl Client {
    pub fn new(stream: TcpStream) -> Result<Client> {
        let addr = stream.peer_addr()?;
        let ip = match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => panic!("IPv6 is not supported")
        };
        stream.set_read_timeout(Some(Duration::from_secs(60)))?;
        stream.set_write_timeout(Some(Duration::from_secs(60)))?;

        Ok(Client {
            ip,
            data_port: addr.port(),
            has_quit: false,
            username: "anonymous".to_owned(),
            password: "anonymous".to_owned(),
            data_repr: DataRepr::default(),

            stream: CrlfStream::new(stream)
        })
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
        let mut client = Client::new(stream)?;
        log::info!("Got a new connection from {}", client.ip);
        client.send_reply(Reply::ServiceReady)?;

        while !client.has_quit  {
            let command = match client.read_command() {
                Ok(command) => command,
                Err(e) => {
                    log::error!("{}", e);
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
            client.send_reply(reply)?;
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

    fn port(client: &mut Client, host_port: HostPort) -> Result<Reply> {
        client.data_port = host_port.port;
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

    fn pasv(&mut self, _client: &mut Client) -> Result<Reply> {
        let addr = self.dtp.make_passive()?;
        let ip = match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => panic!("IPv6 is not supported")
        };
        Ok(Reply::EnteringPassiveMode(HostPort { ip, port: addr.port() }))
    }

    fn retr(&mut self, client: &mut Client, path: String) -> Result<Reply> {
        self.connect_dtp(client)?;
        self.dtp.send_file(path.as_str())?;
        Ok(Reply::FileActionSuccessful)
    }

    fn stor(&mut self, client: &mut Client, path: String) -> Result<Reply> {
        self.connect_dtp(client)?;
        self.dtp.receive_file(path.as_str())?;
        Ok(Reply::FileActionSuccessful)
    }

    fn nlist(&mut self, client: &mut Client, path: Option<String>) -> Result<Reply> {
        self.connect_dtp(client)?;
        self.dtp.send_dir_nlisting(path)?;
        Ok(Reply::FileActionSuccessful)
    }

    fn connect_dtp(&mut self, client: &mut Client) -> Result<()> {
        if let Some(res) = self.dtp.connect(SocketAddr::new(IpAddr::V4(client.ip), client.data_port)) {
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
        let reply = Reply::EnteringPassiveMode(HostPort {ip: Ipv4Addr::LOCALHOST, port: 8888});
        assert_eq!(reply.to_string(), "227 Entering passive mode (127,0,0,1,34,184)");
        let reply = Reply::Created("very-important-directory".to_owned());
        assert_eq!(reply.to_string(), "257 \"very-important-directory\" created")
    }
}