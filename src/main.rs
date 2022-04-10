mod server;
mod protocol_interpreter;
mod data_transfer_process;

use protocol_interpreter::ProtocolInterpreter;

use simplelog::*;

use std::fs::File;
use crate::data_transfer_process::DataTransferProcess;

fn main() {
    CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Warn, Config::default(),
                            TerminalMode::Mixed, ColorChoice::Auto),
            WriteLogger::new(LevelFilter::Debug, Config::default(),
                             File::create("log.log").unwrap())

        ]
    ).unwrap();
    let mut pi = ProtocolInterpreter::new(DataTransferProcess::new(".".to_owned()));
    pi.run("127.0.0.1:2137");
}