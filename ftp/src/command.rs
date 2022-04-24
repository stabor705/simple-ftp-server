use std::fmt::{Display, Formatter};

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

#[derive(Debug)]
pub enum ArgError {
    ArgMissing,
    BadArg
}

impl Display for ArgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ArgError::*;

        match *self {
            ArgMissing => write!(f, "missing required argument"),
            BadArg => write!(f, "invalid format of provided argument")
        }
    }
}

impl std::error::Error for ArgError {}

impl Command {
    pub fn parse_line(s: &str) -> Result<Command, ArgError> {
        use Command::*;

        let mut words = s.split(' ');
        let command = words.next().ok_or(ArgError::ArgMissing)?
            .parse::<Command>().map_err(|_| ArgError::BadArg)?;
        let command = match command {
            User(_) => {
                let username = words.next().ok_or(ArgError::ArgMissing)?;
                User(username.to_owned())
            }
            Pass(_) => {
                let pass = words.next().ok_or(ArgError::ArgMissing)?;
                Pass(pass.to_owned())
            }
            Port(_) => {
                let host_port = words.next().ok_or(ArgError::ArgMissing)?
                    .parse().map_err(|_| ArgError::BadArg)?;
                Port(host_port)
            }
            Type(_) => {
                let data_type: DataType = words.next().ok_or(ArgError::ArgMissing)?
                    .parse().map_err(|_| ArgError::BadArg)?;
                let data_type = match data_type {
                    DataType::ASCII(_) => {
                        let data_format: DataFormat = match words.next() {
                            Some(data_format) => {
                                data_format.parse().map_err(|_| ArgError::BadArg)?
                            }
                            None => DataFormat::default()
                        };
                        DataType::ASCII(data_format)
                    }
                    DataType::EBCDIC(_) => {
                        let data_format: DataFormat = match words.next() {
                            Some(data_format) => {
                                data_format.parse().map_err(|_| ArgError::BadArg)?
                            }
                            None => DataFormat::default()
                        };
                        DataType::EBCDIC(data_format)
                    }
                    DataType::Image => DataType::Image,
                    DataType::Local(_) => {
                        let byte_size: u8 = words.next().ok_or(ArgError::ArgMissing)?
                            .parse().map_err(|_| ArgError::BadArg)?;
                        DataType::Local(byte_size)
                    }
                };
                Type(data_type)
            }
            Stru(_) => {
                let data_structure: DataStructure = words.next().ok_or(ArgError::ArgMissing)?
                    .parse().map_err(|_| ArgError::BadArg)?;
                Stru(data_structure)
            }
            Mode(_) => {
                let mode: TransferMode = words.next().ok_or(ArgError::ArgMissing)?
                    .parse().map_err(|_| ArgError::BadArg)?;
                Mode(mode)
            }
            Retr(_) => {
                let path = words.next().ok_or(ArgError::ArgMissing)?;
                Retr(path.to_owned())
            }
            Stor(_) => {
                let path = words.next().ok_or(ArgError::ArgMissing)?;
                Stor(path.to_owned())
            }
            Nlst(_) => {
                let path = words.next().and_then(|x| Some(x.to_owned()));
                Nlst(path)
            }
            _ => command
        };
        Ok(command)
    }

    pub fn to_line(&self) -> String {
        use Command::*;
        match self {
            User(username) => format!("{} {}", self.to_string(), username),
            Pass(pass) => format!("{} {}", self.to_string(), pass),
            Port(host_port) => format!("{} {}", self.to_string(), host_port.to_string()),
            Type(data_type) => format!("{} {}", self.to_string(), data_type),
            Stru(data_structure) => format!("{} {}", self.to_string(), data_structure),
            Mode(transfer_mode) => format!("{} {}", self.to_string(), transfer_mode),
            Retr(path) => format!("{} {}", self.to_string(), path),
            Nlst(path) => match path {
                Some(path) => format!("{} {}", self.to_string(), path),
                None => self.to_string()
            }
            Stor(path) => format!("{} {}", self.to_string(), path),
            _ => self.to_string()
        }
    }
}

