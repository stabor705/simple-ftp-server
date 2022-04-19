mod protocol_interpreter;
mod data_transfer_process;
mod ftpserver;
mod test;
mod config;

use simplelog::*;
use anyhow::Result;

use std::fs::File;
use crate::ftpserver::FtpServer;
use crate::config as ftp_config;

fn main() -> Result<()> {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(),
                            TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, Config::default(),
                             File::create("log.log").unwrap())

        ]
    ).unwrap();
    let mut ftp = FtpServer::new(ftp_config::Config::default())?;
    ftp.run();
    Ok(())
}