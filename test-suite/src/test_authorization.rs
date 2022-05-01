use crate::TestEnvironment;

use ftp_client::FtpStream;

#[test]
fn test_simple_login() {
    let env = TestEnvironment::new();
    let mut ftp = FtpStream::connect(env.server_addr).unwrap();
    ftp.login("test", "test").unwrap();
    ftp.quit().unwrap();
}

#[test]
fn test_wrong_credentials() {
    let env = TestEnvironment::new();
    let mut ftp = FtpStream::connect(env.server_addr).unwrap();
    assert!(ftp.login("this user", "does not exists").is_err());
    ftp.quit().unwrap();
}

#[test]
fn not_authorized_action() {
    let env = TestEnvironment::new();
    let mut ftp = FtpStream::connect(env.server_addr).unwrap();
    assert!(ftp.pwd().is_err());
    ftp.quit().unwrap();
}

#[test]
fn authorized_action() {
    let env = TestEnvironment::new();
    let mut ftp = FtpStream::connect(env.server_addr).unwrap();
    ftp.login("test", "test").unwrap();
    ftp.pwd().unwrap();
    ftp.quit().unwrap();
}
