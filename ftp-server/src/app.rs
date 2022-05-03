use crate::config::{CliConfig, Config, TomlConfig};
use ftp::{FtpConfig, FtpServer};

use clap::Parser;
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

        let cli_config = CliConfig::parse();

        let toml_config = if let Some(toml_path) = &cli_config.config_file {
            let toml_input = Self::fallible_config_read(toml_path)?;
            Some((toml_path.to_string(), toml_input))
        } else {
            Self::read_default_config()
        };

        if let Some((toml_path, toml_input)) = toml_config {
            let toml_config = Self::decode_toml(&toml_path, &toml_input)?;
            config.merge(&toml_config);
        }

        config.merge(&cli_config);

        let ftp_config = FtpConfig {
            ip: config.ip,
            port: config.port,
            users: config.users,
            conn_timeout: Duration::from_secs(180),
        };

        Self::run_server(ftp_config)?;
        Ok(())
    }

    fn run_server(ftp_config: FtpConfig) -> Result<(), UserFacingError> {
        let ftp_server = match FtpServer::new(ftp_config.clone()) {
            Ok(server) => server,
            Err(err) => {
                let error = UserFacingError::new(format!("Failed to bind on port {}", ftp_config.port));
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

    fn fallible_config_read(path: &str) -> Result<String, UserFacingError> {
        match read_to_string(path) {
            Ok(config) => Ok(config),
            Err(err) => {
                let error = UserFacingError::new(format!("Could not read {} config file", path));
                let error = match err.kind() {
                    ErrorKind::NotFound => error.reason("File not found"),
                    ErrorKind::PermissionDenied => {
                        error.reason("Insufficient permissions to open the file")
                    }
                    ErrorKind::InvalidData => error.reason("Config file is probably invalid UTF-8"),
                    _ => error.reason("It is due to unexpected reasons"),
                };
                let error = error.help(err.to_string());
                return Err(error);
            }
        }
    }

    fn read_default_config() -> Option<(String, String)> {
        static TOML_CONFIG_PATHS: &[&str] = &["config.toml"];

        for path in TOML_CONFIG_PATHS {
            if let Ok(config) = read_to_string(path) {
                return Some((path.to_string(), config));
            }
        }
        None
    }

    fn decode_toml(toml_path: &str, toml_input: &str) -> Result<TomlConfig, UserFacingError> {
        match TomlConfig::from_str(toml_input) {
            Ok(toml_config) => Ok(toml_config),
            Err(err) => {
                let error = UserFacingError::new(format!("Unable to decode {} file", toml_path))
                    .reason("Could not deserialize toml input");
                let error = match err.line_col() {
                    None => error,
                    Some((line, col)) => {
                        error.help(format!("The problem is on line {} column {}", line, col))
                    }
                };
                let error = error.help(err.to_string());
                return Err(error);
            }
        }
    }
}
