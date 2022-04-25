use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::HostPort;
use crate::data_transfer_process::{DataType, DataStructure, TransferMode, DataFormat};

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

    // Not implemented

    Acct,
    Cwd,
    Cdup,
    Smnt,
    Rein,
    Stou,
    Appe,
    Allo,
    Rest,
    Rnfr,
    Rnto,
    Abor,
    Dele,
    Rmd,
    Mkd,
    Pwd,
    List,
    Site,
    Syst,
    Stat,
    Help,
}

//TODO: More info
#[derive(Debug)]
pub enum CommandError {
    ArgMissing,
    BadArg,
    InvalidCommand
}

impl Display for CommandError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use CommandError::*;

        match *self {
            ArgMissing => write!(f, "missing required argument"),
            BadArg => write!(f, "invalid format of provided argument"),
            InvalidCommand => write!(f, "command not found")
        }
    }
}

impl std::error::Error for CommandError {}

impl Command {
    pub fn parse_line(s: &str) -> Result<Command, CommandError> {
        use Command::*;

        let (command, arg) = match s.split_once(' ') {
            Some((command, arg)) => (command, Some(arg)),
            None => (s, None)
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
                let host_port = arg.ok_or(CommandError::ArgMissing)?
                    .parse().map_err(|_| CommandError::BadArg)?;
                Port(host_port)
            }
            Type(_) => {
                let arg = arg.ok_or(CommandError::ArgMissing)?;
                let (data_type, arg) = match arg.split_once(' ') {
                    Some((data_type, arg)) => (data_type, Some(arg)),
                    None => (arg, None)
                };
                let data_type = DataType::from_str(data_type).map_err(|_| CommandError::BadArg)?;
                let data_type = match data_type {
                    DataType::ASCII(_) => {
                        let data_format: DataFormat = match arg {
                            Some(data_format) => {
                                data_format.parse().map_err(|_| CommandError::BadArg)?
                            }
                            None => DataFormat::default()
                        };
                        DataType::ASCII(data_format)
                    }
                    DataType::EBCDIC(_) => {
                        let data_format: DataFormat = match arg {
                            Some(data_format) => {
                                data_format.parse().map_err(|_| CommandError::BadArg)?
                            }
                            None => DataFormat::default()
                        };
                        DataType::EBCDIC(data_format)
                    }
                    DataType::Image => DataType::Image,
                    DataType::Local(_) => {
                        let byte_size: u8 = arg.ok_or(CommandError::ArgMissing)?
                            .parse().map_err(|_| CommandError::BadArg)?;
                        DataType::Local(byte_size)
                    }
                };
                Type(data_type)
            }
            Stru(_) => {
                let data_structure: DataStructure = arg.ok_or(CommandError::ArgMissing)?
                    .parse().map_err(|_| CommandError::BadArg)?;
                Stru(data_structure)
            }
            Mode(_) => {
                let mode: TransferMode = arg.ok_or(CommandError::ArgMissing)?
                    .parse().map_err(|_| CommandError::BadArg)?;
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
            _ => command
        };
        Ok(command)
    }
}

