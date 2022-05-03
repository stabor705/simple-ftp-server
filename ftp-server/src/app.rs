use crate::{Config, TomlConfig};
use ftp::{FtpConfig, FtpServer};

use user_error::UserFacingError;

use std::concat;
use std::fs::read_to_string;
use std::io::ErrorKind;
use std::str::FromStr;
use std::time::Duration;

pub struct App {}

impl App {
    pub fn run() -> Result<(), UserFacingError> {
        let mut config = Config::default();

        if let Some((path, toml_input)) = Self::read_file_config() {
            match TomlConfig::from_str(&toml_input) {
                Ok(toml_config) => {
                    config.merge(&toml_config);
                }
                Err(err) => {
                    let error = UserFacingError::new(format!("Unable to decode {} file", path))
                        .reason("Could not deserialize toml input");
                    let error = match err.line_col() {
                        None => error,
                        Some((line, col)) => {
                            error.help(format!("The problem is on line {} column {}", line, col))
                        }
                    };
                    let error = error.help(format!("{}", err));
                    return Err(error);
                }
            }
        }

        let ftp_config = FtpConfig {
            ip: config.ip,
            port: config.port,
            users: config.users,
            conn_timeout: Duration::from_secs(180),
        };

        let ftp_server = match FtpServer::new(ftp_config) {
            Ok(server) => server,
            Err(err) => {
                let error = UserFacingError::new(format!("Failed to bind on port {}", config.port));
                let error = match err.kind() {
                    ErrorKind::PermissionDenied => error
                        .reason("Lacking required permissions")
                        .help("Try to run as sudo or bind on other port"),
                    ErrorKind::AddrInUse => error.reason("Port is already in use").help(concat!(
                        "Try changing port server tries to use or ",
                        "find process that uses requested port and ",
                       "kill it"
                    )),
                    ErrorKind::AddrNotAvailable => error
                        .reason("A nonexistent interface was requested")
                        .help("Change ip you are trying to bind on"),
                    _ => error
                        .reason("Encountered unexpected error")
                        .help(format!("Action returned with error {}", err)),
                };
                return Err(error);
            }
        };
        ftp_server.run();
        Ok(())
    }

    fn read_file_config() -> Option<(&'static str, String)> {
        static TOML_CONFIG_PATHS: &[&str] = &["config.toml"];

        for path in TOML_CONFIG_PATHS {
            if let Ok(config) = read_to_string(path) {
                return Some((path, config));
            }
        }
        None
    }
}
