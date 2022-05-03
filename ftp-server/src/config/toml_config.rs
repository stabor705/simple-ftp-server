use std::collections::HashMap;
use std::convert::Into;
use std::net::Ipv4Addr;
use std::str::FromStr;

use super::{Config, ConfigChanges};

use log::LevelFilter;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TomlConfig {
    server: Option<ServerConfig>,
    #[serde(rename(deserialize = "user"))]
    users: Option<HashMap<String, User>>,
    #[serde(rename(deserialize = "log"))]
    log_opts: Option<LogOpts>,
}

impl FromStr for TomlConfig {
    type Err = toml::de::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let config = toml::from_str(s)?;
        Ok(config)
    }
}

impl ConfigChanges for TomlConfig {
    fn apply(&self, config: &mut Config) {
        if let Some(server) = &self.server {
            if let Some(ip) = server.ip {
                config.ip = ip;
            }
            if let Some(port) = server.port {
                config.port = port;
            }
            if let Some(timeout) = server.timeout {
                config.timeout = timeout;
            }
        }
        if let Some(users) = &self.users {
            for (username, user) in users {
                config.push_user(
                    username.clone(),
                    user.password.clone(),
                    user.directory.clone(),
                )
            }
        }
        if let Some(log_opts) = &self.log_opts {
            if let Some(file_log_opts) = log_opts.file_log_opts.clone() {
                config.log.file = Some(super::FileLogOpts {
                    file_path: file_log_opts.path.clone(),
                    level: file_log_opts.level.into()
                });
            }
            if let Some(console_log_opts) = log_opts.console_log_opts.clone() {
                config.log.console.level= console_log_opts.level.into();
            }
            if let Some(syslog_opts) = log_opts.syslog_opts.clone() {
                config.log.sys.level = syslog_opts.level.into();
            }
        }
    }
}

#[derive(Deserialize)]
struct ServerConfig {
    ip: Option<Ipv4Addr>,
    port: Option<u16>,
    timeout: Option<u64>,
}

#[derive(Deserialize)]
struct User {
    password: String,
    directory: String,
}

#[derive(Deserialize, Clone)]
enum LogLevel {
    #[serde(rename(deserialize = "off"))]
    Off,
    #[serde(rename(deserialize = "error"))]
    Error,
    #[serde(rename(deserialize = "warn"))]
    Warn,
    #[serde(rename(deserialize = "info"))]
    Info,
    #[serde(rename(deserialize = "debug"))]
    Debug,
    #[serde(rename(deserialize = "trace"))]
    Trace,
}

// impl From<LevelFilter> for LogLevel{
//     fn from(level_filter: LevelFilter) -> Self {
//         match level_filter {
//             LevelFilter::Off => LogLevel::Off,
//             LevelFilter::Error => LogLevel::Error,
//             LevelFilter::Warn => LogLevel::Warn,
//             LevelFilter::Info => LogLevel::Info,
//             LevelFilter::Debug => LogLevel::Debug,
//             LevelFilter::Trace => LogLevel::Trace,
//         }
//     }
// }

impl Into<LevelFilter> for LogLevel {
    fn into(self) -> LevelFilter {
        match self {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Deserialize, Clone)]
struct FileLogOpts {
    path: String,
    level: LogLevel,
}

#[derive(Deserialize, Clone)]
struct ConsoleLogOpts {
    level: LogLevel,
}

#[derive(Deserialize, Clone)]
struct SysLogOpts {
    level: LogLevel,
}

#[derive(Deserialize)]
struct LogOpts {
    #[serde(rename(deserialize = "file"))]
    file_log_opts: Option<FileLogOpts>,
    #[serde(rename(deserialize = "console"))]
    console_log_opts: Option<ConsoleLogOpts>,
    #[serde(rename(deserialize = "syslog"))]
    syslog_opts: Option<SysLogOpts>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_parsing() {
        let input = r#"
            [server]
            port = 2137
            timeout = 190
            [user.Henryk]
            password = "a very secret password"
            directory = "/home/henryk"
            [user.Maria]
            password = "123"
            directory = "/home/maria/ftp"
            [log.file]
            path = "/var/log/ftp.log"
            level = "warn"
        "#;
        let config: TomlConfig = toml::from_str(input).unwrap();
        let server = config.server.unwrap();
        assert_eq!(server.ip, None);
        assert_eq!(server.port, Some(2137));
        let users = config.users.unwrap();
        assert_eq!(users["Henryk"].password, "a very secret password");
        assert_eq!(users["Henryk"].directory, "/home/henryk");
        assert_eq!(users["Maria"].password, "123");
        assert_eq!(users["Maria"].directory, "/home/maria/ftp");
        let log_opts = config.log_opts.unwrap();
        assert!(log_opts.console_log_opts.is_none());
        assert!(log_opts.syslog_opts.is_none());
        let file_log_opts = log_opts.file_log_opts.unwrap();
        assert_eq!(file_log_opts.path, "/var/log/ftp.log");
        assert_eq!(file_log_opts.path, "/var/log/ftp.log");
    }
}
