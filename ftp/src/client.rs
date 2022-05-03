use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use crate::DataTransferProcess;
use crate::HostPort;

use anyhow::{Error, Result};

pub struct Client {
    pub data_ip: Ipv4Addr,
    pub data_port: u16,
    pub has_quit: bool,
    pub username: Option<String>,

    commands_impl: Box<dyn CommandsImpl>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("client is not authorized")]
    NotLoggedIn,
    // This special error is neede, since it doesn't make sense to print
    // working directory in this implementation, but FTP specification
    // doesn't list 530 as correct reply code for PWD command, so a workaround
    // producing other reply is needed
    #[error("client is not authorized to pwd")]
    PwdWhileNotLoggedIn,
}

impl Client {
    pub fn new(ip: Ipv4Addr) -> Client {
        Client {
            data_ip: ip,
            data_port: 0,
            has_quit: false,
            username: None,
            commands_impl: Box::new(NotLoggedIn {}),
        }
    }

    pub fn quit(&mut self) {
        self.has_quit = true;
    }

    pub fn port(&mut self, host_port: HostPort) {
        self.data_ip = host_port.ip;
        self.data_port = host_port.port;
    }

    pub fn user(&mut self, username: String) {
        self.username = Some(username);
    }

    pub fn authorize(&mut self, root_dir: &str, conn_timeout: Duration) {
        self.commands_impl = Box::new(LoggedIn::new(root_dir, conn_timeout));
    }

    pub fn pasv(&mut self) -> Result<HostPort> {
        self.commands_impl.pasv()
    }

    pub fn retr(&mut self, path: &str) -> Result<()> {
        self.commands_impl.retr(path)
    }

    pub fn stor(&mut self, path: &str) -> Result<()> {
        self.commands_impl.stor(path)
    }

    pub fn nlst(&mut self, path: Option<String>) -> Result<()> {
        self.commands_impl.nlst(path)
    }

    pub fn pwd(&self) -> Result<String> {
        self.commands_impl.pwd()
    }

    pub fn cwd(&mut self, path: &str) -> Result<()> {
        self.commands_impl.cwd(path)
    }

    pub fn mkd(&self, path: &str) -> Result<()> {
        self.commands_impl.mkd(path)
    }

    pub fn dele(&self, path: &str) -> Result<()> {
        self.commands_impl.dele(path)
    }

    pub fn rnfr(&mut self, path: &str) -> Result<()> {
        self.commands_impl.rnfr(path)
    }

    pub fn rnto(&mut self, path: &str) -> Result<()> {
        self.commands_impl.rnto(path)
    }

    pub fn cdup(&mut self) -> Result<()> {
        self.commands_impl.cdup()
    }

    pub fn list(&mut self, path: Option<String>) -> Result<()> {
        self.commands_impl.list(path)
    }

    pub fn connect_dtp(&mut self) -> Result<()> {
        self.commands_impl
            .connect_dtp(SocketAddr::new(IpAddr::V4(self.data_ip), self.data_port))
    }
}

trait CommandsImpl {
    fn pasv(&mut self) -> Result<HostPort>;
    fn retr(&mut self, path: &str) -> Result<()>;
    fn stor(&mut self, path: &str) -> Result<()>;
    fn nlst(&mut self, path: Option<String>) -> Result<()>;
    fn pwd(&self) -> Result<String>;
    fn cwd(&mut self, path: &str) -> Result<()>;
    fn mkd(&self, path: &str) -> Result<()>;
    fn dele(&self, path: &str) -> Result<()>;
    fn rnfr(&mut self, path: &str) -> Result<()>;
    fn rnto(&mut self, path: &str) -> Result<()>;
    fn cdup(&mut self) -> Result<()>;
    fn list(&mut self, path: Option<String>) -> Result<()>;
    fn connect_dtp(&mut self, addr: SocketAddr) -> Result<()>;
}

struct LoggedIn {
    dtp: DataTransferProcess,
}

impl LoggedIn {
    pub fn new(root_dir: &str, conn_timeout: Duration) -> LoggedIn {
        LoggedIn {
            dtp: DataTransferProcess::new(root_dir.to_string(), conn_timeout),
        }
    }
}

impl CommandsImpl for LoggedIn {
    fn pasv(&mut self) -> Result<HostPort> {
        let addr = self.dtp.make_passive()?;
        let ip = match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => panic!("IPv6 is not supported"),
        };
        Ok(HostPort::new(ip, addr.port()))
    }

    fn retr(&mut self, path: &str) -> Result<()> {
        self.dtp.send_file(path)?;
        Ok(())
    }

    fn stor(&mut self, path: &str) -> Result<()> {
        self.dtp.receive_file(path)?;
        Ok(())
    }

    fn nlst(&mut self, path: Option<String>) -> Result<()> {
        self.dtp.send_dir_nlisting(path)?;
        Ok(())
    }

    fn pwd(&self) -> Result<String> {
        Ok(self.dtp.get_working_dir())
    }

    fn cwd(&mut self, path: &str) -> Result<()> {
        self.dtp.change_working_dir(path)?;
        Ok(())
    }

    fn mkd(&self, path: &str) -> Result<()> {
        self.dtp.make_dir(path)?;
        Ok(())
    }

    fn dele(&self, path: &str) -> Result<()> {
        self.dtp.delete_file(path)?;
        Ok(())
    }

    fn rnfr(&mut self, path: &str) -> Result<()> {
        self.dtp.prepare_rename(path)?;
        Ok(())
    }

    fn rnto(&mut self, path: &str) -> Result<()> {
        self.dtp.rename(path)?;
        Ok(())
    }

    fn cdup(&mut self) -> Result<()> {
        self.dtp.change_working_dir("..")?;
        Ok(())
    }

    fn list(&mut self, path: Option<String>) -> Result<()> {
        self.dtp.send_dir_listing(path)?;
        Ok(())
    }

    fn connect_dtp(&mut self, addr: SocketAddr) -> Result<()> {
        self.dtp.connect(addr)?;
        Ok(())
    }
}

struct NotLoggedIn {}

impl CommandsImpl for NotLoggedIn {
    fn pasv(&mut self) -> Result<HostPort> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn retr(&mut self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn stor(&mut self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn nlst(&mut self, _path: Option<String>) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn pwd(&self) -> Result<String> {
        Err(Error::new(AuthError::PwdWhileNotLoggedIn))
    }

    fn cwd(&mut self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn mkd(&self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn dele(&self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn rnfr(&mut self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn rnto(&mut self, _path: &str) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn cdup(&mut self) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn list(&mut self, _path: Option<String>) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }

    fn connect_dtp(&mut self, _addr: SocketAddr) -> Result<()> {
        Err(Error::new(AuthError::NotLoggedIn))
    }
}
