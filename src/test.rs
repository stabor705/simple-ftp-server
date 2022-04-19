use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::thread;

use crate::ftpserver::FtpServer;
use crate::protocol_interpreter::{Reply, Client};

fn spawn_server() -> Client {
    let ftp = FtpServer::new((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = ftp.addr().unwrap();
    thread::spawn(move || {
        ftp.do_one_listen();
    });
    Client::new(TcpStream::connect(addr).unwrap())
}

#[test]
fn test_connect_and_quit() {
    let mut client = spawn_server();
    let resp = client.read_message().unwrap();
    assert_eq!(resp, Reply::ServiceReady.to_string());
    client.send_message("Quit");
    let resp = client.read_message().unwrap();
    assert_eq!(resp, Reply::ServiceClosing.to_string());
}

