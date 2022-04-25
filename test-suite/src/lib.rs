use std::thread;
use std::fs::File;
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Once;

use ftp::FtpServer;

use tempdir::TempDir;
use simplelog::*;
use ftp_client::FtpStream;

struct TestEnvironment {
    dir: TempDir,
    server_addr: SocketAddr
}

static INIT_LOG: Once = Once::new();

fn initialize_logger() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(),
                            TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, Config::default(),
                             File::create("test.log").unwrap()),
        ]
    ).unwrap();
}

#[allow(dead_code)]
impl TestEnvironment {
    pub fn new() -> TestEnvironment {
        INIT_LOG.call_once(initialize_logger);
        let dir = TempDir::new("ftp-test").unwrap();
        let config = ftp::Config {
            ip: Ipv4Addr::LOCALHOST,
            control_port: 0,
            dir_root: dir.path().to_string_lossy().to_string()
        };
        let mut ftp_server = FtpServer::new(config).unwrap();
        let server_addr = ftp_server.addr().unwrap();
        thread::spawn(move || {
            ftp_server.do_one_listen().unwrap();
        });
        TestEnvironment { dir, server_addr }
    }

    pub fn create_empty_file(&self, path: &str) {
        File::create(self.dir.path().join(path)).unwrap();
    }

    pub fn create_file(&self, path: &str, contents: &[u8]) {
        let mut file = File::create(self.dir.path().join(path)).unwrap();
        file.write_all(contents).unwrap();
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_and_quit() {
        let env = TestEnvironment::new();
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        ftp.quit().unwrap();
    }

    #[test]
    fn test_nlist() {
        let env = TestEnvironment::new();
        env.create_empty_file("1");
        env.create_empty_file("2");
        env.create_empty_file("3");
        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let mut list = ftp.nlst(None).unwrap();
        ftp.quit();
        list.sort();
        assert_eq!(list, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_file_receiving() {
        let env = TestEnvironment::new();
        let filename = "a very important file with a very long name lol.txt";
        let text = "Hello World!";
        env.create_file(filename, text.as_bytes());

        let mut ftp = FtpStream::connect(env.server_addr).unwrap();
        let cursor = ftp.simple_retr(filename).unwrap();
        assert_eq!(cursor.into_inner().as_slice(), text.as_bytes());
        ftp.quit().unwrap();
    }
}