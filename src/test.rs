use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::fs::File;
use std::path::Path;
use std::io::{Read, Write};
use std::ops::Drop;
use std::str::FromStr;

use crate::ftpserver::FtpServer;
use crate::protocol_interpreter::{Reply, Client, Command, CrlfStream, HostPort};
use crate::config::Config;

use tempdir::TempDir;
use simplelog::*;
use crate::protocol_interpreter::Reply::{DataConnectionOpen, DirectoryStatus};

struct TestSession {
    dir: TempDir,
    stream: CrlfStream,
    data_stream: Option<TcpStream>
}

impl TestSession {
    pub fn start() -> TestSession {
        let dir = TempDir::new("ftp-test").unwrap();
        let config = Config {
            ip: Ipv4Addr::LOCALHOST,
            control_port: 0,
            dir_root: dir.path().to_string_lossy().into_owned()
        };
        let mut ftp = FtpServer::new(config).unwrap();
        let addr = ftp.addr().unwrap();
        thread::spawn(move || {
            ftp.do_one_listen();
        });
        let stream = CrlfStream::new(TcpStream::connect(addr).unwrap());
        let mut session = TestSession { dir, stream, data_stream: None };
        session.expect_reply(Reply::ServiceReady);
        session
    }

    pub fn send_command(&mut self, command: Command) {
        self.stream.send_message(command.to_line().as_str()).unwrap();
    }

    pub fn expect_reply(&mut self, reply: Reply) {
        let msg = self.stream.read_message().unwrap();
        assert_eq!(msg, reply.to_string())
    }

    pub fn expect_pasv_reply(&mut self) {
        let msg = self.stream.read_message().unwrap();
        let host_port = HostPort::from_str(msg.rsplit(' ').next().unwrap()).unwrap();
        self.data_stream = Some(TcpStream::connect((host_port.ip, host_port.port)).unwrap())
    }

    pub fn expect_data(&mut self, data: &[u8]) {
        let mut received = Vec::new();
        loop {
            let mut buf = [0 as u8; 256];
            let n = self.data_stream.as_ref().unwrap().read(&mut buf).unwrap();
            if n == 0 { break; }
            received.extend_from_slice(&buf[0..n]);
        }
        assert_eq!(data, received.as_slice())
    }

    pub fn create_file(&self, path: &str) {
        File::create(self.dir.path().join(Path::new(path))).unwrap();
    }
}

impl Drop for TestSession {
    fn drop(&mut self) {
        self.send_command(Command::Quit);
        self.expect_reply(Reply::ServiceClosing);
    }
}

#[test]
fn test_connect_and_quit() {
    TestSession::start();
}

#[test]
fn test_nlist() {
    let mut session = TestSession::start();
    session.create_file("1.txt");
    session.create_file("2.txt");
    session.create_file("3.txt");
    session.send_command(Command::Pasv);
    session.expect_pasv_reply();
    session.send_command(Command::Nlst(None));
    session.expect_reply(Reply::OpeningDataConnection);
    session.expect_reply(Reply::DirectoryStatus);
    session.expect_data("3.txt\r\n2.txt\r\n1.txt\r\n".as_bytes());
}