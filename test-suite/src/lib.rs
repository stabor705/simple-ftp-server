mod test_basic_commands;

use std::fs::{create_dir, File};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
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
        let config = ftp::Config {
            ip: Ipv4Addr::LOCALHOST,
            control_port: 0,
            dir_root: dir.path().to_string_lossy().to_string(),
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

    pub fn create_dir(&self, path: &str) {
        create_dir(self.dir.path().join(path)).unwrap();
    }
}
