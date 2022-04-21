use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::fs::File;
use std::io::Write;
use std::ops::Drop;

use crate::ftpserver::FtpServer;
use crate::protocol_interpreter::{Reply, Client, Command, CrlfStream};
use crate::config::Config;

use tempdir::TempDir;
use simplelog::*;

struct TestSession {
    dir: TempDir,
    stream: CrlfStream,
    data_stream: Option<TcpStream>
}

impl TestSession {
    pub fn start() -> TestSession {
        let dir = TempDir::new("ftp-test").unwrap();
        let config = Config {
            ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
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