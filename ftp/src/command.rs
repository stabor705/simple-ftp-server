use std::str::FromStr;

use crate::data_transfer_process::{DataFormat, DataStructure, DataType, TransferMode};
use crate::HostPort;

use strum_macros::EnumString;

#[derive(EnumString, strum_macros::Display)]
#[strum(ascii_case_insensitive)]
pub enum Command {
    // Implemented
    User(String),
    Pass(String),
    Quit,
    Port(HostPort),
    Type(DataType),
    Stru(DataStructure),
    Mode(TransferMode),
    Noop,
    Retr(String),
    Pasv,
    Nlst(Option<String>),
    Stor(String),
    Pwd,
    Cwd(String),
    Mkd(String),
    Dele(String),
    Rnfr(String),
    Rnto(String),
    Cdup,
    List(String),

    // Not implemented
    Acct,
    Smnt,
    Rein,
    Stou,
    Appe,
    Allo,
    Rest,
    Abor,
    Rmd,
    Site,
    Syst,
    Stat,
    Help,
}

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("missing required argument")]
    ArgMissing,
    #[error("provided argument was invalid")]
    BadArg,
    #[error("command not found")]
    InvalidCommand,
}

impl Command {
    pub fn parse_line(s: &str) -> Result<Command, CommandError> {
        use Command::*;

        let (command, arg) = match s.split_once(' ') {
            Some((command, arg)) => (command, Some(arg)),
            None => (s, None),
        };
        let command = Command::from_str(command).map_err(|_| CommandError::InvalidCommand)?;
        let command = match command {
            User(_) => {
                let username = arg.ok_or(CommandError::ArgMissing)?;
                User(username.to_owned())
            }
            Pass(_) => {
                let pass = arg.ok_or(CommandError::ArgMissing)?;
                Pass(pass.to_owned())
            }
            Port(_) => {
                let host_port = arg
                    .ok_or(CommandError::ArgMissing)?
                    .parse()
                    .map_err(|_| CommandError::BadArg)?;
                Port(host_port)
            }
            Type(_) => {
                let arg = arg.ok_or(CommandError::ArgMissing)?;
                let (data_type, arg) = match arg.split_once(' ') {
                    Some((data_type, arg)) => (data_type, Some(arg)),
                    None => (arg, None),
                };
                let data_type = DataType::from_str(data_type).map_err(|_| CommandError::BadArg)?;
                let data_type = match data_type {
                    DataType::ASCII(_) => {
                        let data_format: DataFormat = match arg {
                            Some(data_format) => {
                                data_format.parse().map_err(|_| CommandError::BadArg)?
                            }
                            None => DataFormat::default(),
                        };
                        DataType::ASCII(data_format)
                    }
                    DataType::EBCDIC(_) => {
                        let data_format: DataFormat = match arg {
                            Some(data_format) => {
                                data_format.parse().map_err(|_| CommandError::BadArg)?
                            }
                            None => DataFormat::default(),
                        };
                        DataType::EBCDIC(data_format)
                    }
                    DataType::Image => DataType::Image,
                    DataType::Local(_) => {
                        let byte_size: u8 = arg
                            .ok_or(CommandError::ArgMissing)?
                            .parse()
                            .map_err(|_| CommandError::BadArg)?;
                        DataType::Local(byte_size)
                    }
                };
                Type(data_type)
            }
            Stru(_) => {
                let data_structure: DataStructure = arg
                    .ok_or(CommandError::ArgMissing)?
                    .parse()
                    .map_err(|_| CommandError::BadArg)?;
                Stru(data_structure)
            }
            Mode(_) => {
                let mode: TransferMode = arg
                    .ok_or(CommandError::ArgMissing)?
                    .parse()
                    .map_err(|_| CommandError::BadArg)?;
                Mode(mode)
            }
            Retr(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Retr(path.to_owned())
            }
            Stor(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Stor(path.to_owned())
            }
            Nlst(_) => {
                let path = arg.and_then(|x| Some(x.to_owned()));
                Nlst(path)
            }
            Cwd(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Cwd(path.to_owned())
            }
            Mkd(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Mkd(path.to_owned())
            }
            Dele(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Dele(path.to_owned())
            }
            Rnfr(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Rnfr(path.to_owned())
            }
            Rnto(_) => {
                let path = arg.ok_or(CommandError::ArgMissing)?;
                Rnto(path.to_owned())
            }
            _ => command,
        };
        Ok(command)
    }
}
