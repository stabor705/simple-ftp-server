use std::fs::File;

use ftp::FtpServer;

use simplelog::*;
use anyhow::Result;

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