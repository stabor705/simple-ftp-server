mod protocol_interpreter;
mod data_transfer_process;
mod ftpserver;
mod test;

use simplelog::*;
use anyhow::Result;

use std::fs::File;
use crate::ftpserver::FtpServer;

fn main() -> Result<()> {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(),
                            TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, Config::default(),
                             File::create("log.log").unwrap())

        ]
    ).unwrap();
    let ftp = FtpServer::new("127.0.0.1:2137")?;
    ftp.run();
    Ok(())
}