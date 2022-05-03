use crate::config::*;
use ftp::{FtpConfig, FtpServer};

use clap::Parser;
use user_error::UserFacingError;
use simplelog::{TermLogger, WriteLogger, SharedLogger, CombinedLogger, TerminalMode, ColorChoice};

use std::concat;
use std::fs::{read_to_string, File};
use std::io::ErrorKind;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

type Result<T> = std::result::Result<T, UserFacingError>;

pub struct App {}

impl App {
    pub fn run() -> Result<()> {
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

        Self::initialize_logger(config.log)?;

        let ftp_config = FtpConfig {
            ip: config.ip,
            port: config.port,
            users: config.users,
            conn_timeout: Duration::from_secs(config.timeout)
        };

        Self::validate_ftp_config(&ftp_config)?;

        Self::run_server(ftp_config)?;
        Ok(())
    }

    fn run_server(ftp_config: FtpConfig) -> Result<()> {
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

    fn fallible_config_read(path: &str) -> Result<String> {
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

    fn decode_toml(toml_path: &str, toml_input: &str) -> Result<TomlConfig> {
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

    fn validate_ftp_config(ftp_config: &FtpConfig) -> Result<()> {
        for user in &ftp_config.users {
            let dir = &user.data.dir;
            if !Path::new(dir).exists() {
                let error = UserFacingError::new(
                                format!("Invalid configuration for user {}", user.username)
                            )
                            .reason(format!("Data directory {} does not extist", dir))
                            .help("Make sure that you valid directory path in your config file");
                return Err(error);
            }
        }
        Ok(())
    }

    fn initialize_logger(log_opts: LogOpts) -> Result<()> {
        let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();
        let term_logger = TermLogger::new(
            log_opts.console.level,
            simplelog::Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto
        );
        loggers.push(term_logger);
        if let Some(file_log_opts) = log_opts.file {
            let file = match File::create(&file_log_opts.file_path) {
                Ok(file) => file,
                Err(err) => {
                    return Err(UserFacingError::new("Could not create log file")
                        .help(err.to_string()))
                }
            };
            let file_logger = WriteLogger::new(
                file_log_opts.level,
                simplelog::Config::default(),
                file
            );
            loggers.push(file_logger);
        }
        // This unwrap should never panic, because init return error
        // only if logging system was initialized more than one time
        CombinedLogger::init(loggers).unwrap();
        Ok(())
    }
}
