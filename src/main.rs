mod server;
mod protocol_interpreter;
mod data_transfer_process;
mod ftpserver;

use simplelog::*;

use std::fs::File;
use crate::ftpserver::FtpServer;

fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(),
                            TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, Config::default(),
                             File::create("log.log").unwrap())

        ]
    ).unwrap();
    FtpServer::run("127.0.0.1:2137");
}