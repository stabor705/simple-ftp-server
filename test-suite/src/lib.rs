#[cfg(test)]
mod test_authorization;
#[cfg(test)]
mod test_basic_commands;

use std::fs::{create_dir, File};
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Once;
use std::thread;

use ftp::FtpServer;

use simplelog::*;
use tempdir::TempDir;

struct TestEnvironment {
    dir: TempDir,
    server_addr: SocketAddr,
}

static INIT_LOG: Once = Once::new();

fn initialize_logger() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Warn,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create("test.log").unwrap(),
        ),
    ])
    .unwrap();
}

#[allow(dead_code)]
impl TestEnvironment {
    pub fn new() -> TestEnvironment {
        INIT_LOG.call_once(initialize_logger);
        let dir = TempDir::new("ftp-test").unwrap();
        let mut ftp_server = FtpServer::builder()
            .add_user(
                "test".to_owned(),
                "test".to_owned(),
                dir.path().to_string_lossy().to_string(),
            )
            .build()
            .unwrap();
        let server_addr = ftp_server.addr().unwrap();
        thread::spawn(move || {
            ftp_server.do_one_listen().unwrap();
        });
        TestEnvironment { dir, server_addr }
    }

    pub fn create_empty_file<P: AsRef<Path>>(&self, path: P) {
        File::create(self.dir.path().join(path)).unwrap();
    }

    pub fn create_file<P: AsRef<Path>>(&self, path: P, contents: &[u8]) {
        let mut file = File::create(self.dir.path().join(path)).unwrap();
        file.write_all(contents).unwrap();
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Vec<u8> {
        let mut file = File::open(self.dir.path().join(path)).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();
        buf
    }

    pub fn create_dir<P: AsRef<Path>>(&self, path: P) {
        create_dir(self.dir.path().join(path)).unwrap();
    }

    pub fn file_exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.dir.path().join(path).exists()
    }
}
