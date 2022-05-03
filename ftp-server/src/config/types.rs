use std::default::Default;
use std::net::Ipv4Addr;

use ftp::{User, UserData};

use log::LevelFilter;

pub struct Config {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub timeout: u32,
    pub users: Vec<User>,
    pub file_log_opts: FileLogOpts,
    pub console_log_opts: ConsoleLogOpts,
    pub syslog_opts: SysLogOpts,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ip: Ipv4Addr::LOCALHOST,
            port: 21,
            timeout: 180,
            users: Vec::new(),
            file_log_opts: FileLogOpts::default(),
            console_log_opts: ConsoleLogOpts::default(),
            syslog_opts: SysLogOpts::default(),
        }
    }
}

impl Config {
    pub fn merge<C: ?Sized>(&mut self, changes: &C)
    where
        C: ConfigChanges,
    {
        changes.apply(self)
    }

    pub fn push_user(&mut self, username: String, password: String, dir: String) {
        self.users.push(User {
            username,
            data: UserData { password, dir },
        })
    }
}

pub trait ConfigChanges {
    fn apply(&self, config: &mut Config);
}

pub struct FileLogOpts {
    pub file_path: String,
    pub level: LevelFilter,
}

impl Default for FileLogOpts {
    fn default() -> Self {
        FileLogOpts {
            file_path: String::new(),
            level: LevelFilter::Off,
        }
    }
}

pub struct ConsoleLogOpts {
    pub level: LevelFilter,
}

impl Default for ConsoleLogOpts {
    fn default() -> Self {
        ConsoleLogOpts {
            level: LevelFilter::Debug,
        }
    }
}

pub struct SysLogOpts {
    pub level: LevelFilter,
}

impl Default for SysLogOpts {
    fn default() -> Self {
        SysLogOpts {
            level: LevelFilter::Error,
        }
    }
}
