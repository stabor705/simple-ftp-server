use std::clone::Clone;
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream};
use std::string::ToString;
use std::time::Duration;

use crate::user::*;
use crate::Client;
use crate::Reply;
use crate::{Command, CommandError};

use anyhow::{Context, Error, Result};

pub struct CrlfStream {
    stream: TcpStream,
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
        let mut msg = String::new();
        loop {
            let mut buf = [0 as u8; 1024];
            let n = self.stream.read(&mut buf)?;
            if n == 0 {
                return Err(Error::new(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Client quit unexpectedly.",
                )));
            }
            //TODO:
            //Even though it isn't statistically probable, I don't think that there is any
            //guarantee about CRLF being sent in one pocket. It could be split into two pockets.
            //I will ignore that for now, but this function will not be correct until I fix it.
            let new_text = std::str::from_utf8(&buf[0..n])?; // ASCII should also be a valid utf8
            if let Some(p) = new_text.find(CRLF) {
                msg += &new_text[..p];
                break;
            } else {
                msg += &new_text;
            }
            if msg.len() > 1024 {
                return Err(Error::new(CommandError::InvalidCommand))
                    .with_context(|| format!("Client's command was way too long {}", msg));
            }
        }
        Ok(msg)
    }
}

pub struct ProtocolInterpreter {
    users: HashMap<Username, UserData>,
    conn_timeout: Duration,
}

impl ProtocolInterpreter {
    pub fn new(users: Vec<User>, conn_timeout: Duration) -> ProtocolInterpreter {
        let users: HashMap<String, UserData> = users
            .iter()
            .map(|user| (user.username.clone(), user.data.clone()))
            .collect();
        ProtocolInterpreter {
            users,
            conn_timeout,
        }
    }

    pub fn handle_client(&mut self, stream: TcpStream) -> Result<()> {
        let ip = stream.peer_addr()?.ip();
        log::info!("Got a new connection from {}", ip);
        let ip = match ip {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => panic!("Got connection with IPv6. This should not have happened"),
        };
        let mut stream = CrlfStream::new(stream);
        let mut client = Client::new(ip);
        Self::send_reply(&mut stream, Reply::ServiceReady)?;

        while !client.has_quit {
            let command = match Self::read_command(&mut stream) {
                Ok(command) => command,
                Err(err) => {
                    if err.is::<CommandError>() {
                        let err: CommandError = err.downcast().unwrap();
                        Self::send_reply(&mut stream, Reply::SyntaxError)?;
                        log::debug!("{}", err);
                        continue;
                    } else if err.is::<std::io::Error>() {
                        let err: std::io::Error = err.downcast().unwrap();
                        log::error!("{}", err);
                        break;
                    }
                    break;
                }
            };
            let reply = match self.dispatch_command(command, &mut client, &mut stream) {
                Ok(reply) => reply,
                Err(err) => {
                    log::warn!("Client's request could not be honored: {}", err);
                    err.into()
                }
            };
            Self::send_reply(&mut stream, reply)?;
        }
        log::info!("Connection with client {} properly closed.", client.data_ip);
        Ok(())
    }

    fn send_reply(stream: &mut CrlfStream, reply: Reply) -> Result<()> {
        let msg = reply.to_string();
        log::debug!("----> {}", msg);
        stream.send_message(msg.as_str())?;
        Ok(())
    }

    pub fn read_command(stream: &mut CrlfStream) -> Result<Command> {
        let msg = stream.read_message()?;
        log::debug!("<---- {}", msg);
        let command = Command::parse_line(msg.as_str())?;
        Ok(command)
    }

    fn dispatch_command(
        &self,
        command: Command,
        client: &mut Client,
        stream: &mut CrlfStream,
    ) -> Result<Reply> {
        match command {
            Command::Quit => {
                client.quit();
                Ok(Reply::ServiceClosing)
            }
            Command::Port(host_port) => {
                client.port(host_port);
                Ok(Reply::CommandOk)
            }
            Command::User(username) => {
                client.user(username);
                Ok(Reply::UsernameOk)
            }
            Command::Pass(pass) => {
                let username = match &client.username {
                    Some(username) => username,
                    // Using PASS before USER
                    None => return Ok(Reply::BadCommandSequence),
                };
                let user = match self.users.get(username) {
                    Some(user) => user,
                    None => return Ok(Reply::NotLoggedIn),
                };
                if pass == user.password {
                    client.authorize(&user.dir, self.conn_timeout);
                    Ok(Reply::UserLoggedIn)
                } else {
                    Ok(Reply::NotLoggedIn)
                }
            }
            /*Ignored for now*/
            Command::Mode(_) => Ok(Reply::CommandOk),
            Command::Stru(_) => Ok(Reply::CommandOk),
            Command::Type(_) => Ok(Reply::CommandOk),
            /*Ignored for now*/
            Command::Pasv => {
                let host_port = client.pasv()?;
                Ok(Reply::EnteringPassiveMode(host_port))
            }
            Command::Retr(path) => {
                Self::connect_dtp(stream, client)?;
                client.retr(&path)?;
                Ok(Reply::ClosingDataConnection)
            }
            Command::Nlst(path) => {
                Self::connect_dtp(stream, client)?;
                client.nlst(path)?;
                Ok(Reply::ClosingDataConnection)
            }
            Command::Stor(path) => {
                Self::connect_dtp(stream, client)?;
                client.stor(&path)?;
                Ok(Reply::ClosingDataConnection)
            }
            Command::Pwd => {
                let working_dir = client.pwd()?;
                Ok(Reply::Created(working_dir))
            }
            Command::Cwd(path) => {
                client.cwd(&path)?;
                Ok(Reply::FileActionOk)
            }
            Command::Mkd(path) => {
                client.mkd(&path)?;
                Ok(Reply::Created(path))
            }
            Command::Dele(path) => {
                client.dele(&path)?;
                Ok(Reply::FileActionOk)
            }
            Command::Rnfr(from) => {
                client.rnfr(&from)?;
                Ok(Reply::PendingFurtherInformation)
            }
            Command::Rnto(to) => {
                client.rnto(&to)?;
                Ok(Reply::FileActionOk)
            }
            Command::Cdup => {
                client.cdup()?;
                Ok(Reply::CommandOk)
            }
            Command::List(path) => {
                Self::connect_dtp(stream, client)?;
                client.list(path)?;
                Ok(Reply::FileActionOk)
            }
            _ => Ok(Reply::NotImplemented),
        }
    }

    fn connect_dtp(stream: &mut CrlfStream, client: &mut Client) -> Result<()> {
        client.connect_dtp()?;
        Self::send_reply(stream, Reply::OpeningDataConnection)?;
        Ok(())
    }
}
