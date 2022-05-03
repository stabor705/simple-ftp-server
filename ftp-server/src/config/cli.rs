use std::net::Ipv4Addr;

use clap::Parser;

use super::{Config, ConfigChanges};

#[derive(Parser)]
#[clap(version, author)]
pub struct CliConfig {
    /// Sets the path to toml configuration file
    #[clap(name = "config", short, long)]
    pub config_file: Option<String>,

    /// Sets the ip address server will try to use
    #[clap(short, long)]
    pub ip: Option<Ipv4Addr>,
    /// Sets the port number the server will try to bind to
    #[clap(short, long)]
    pub port: Option<u16>,
}

impl ConfigChanges for CliConfig {
    fn apply(&self, config: &mut Config) {
        if let Some(ip) = self.ip {
            config.ip = ip;
        }
        if let Some(port) = self.port {
            config.port = port;
        }
    }
}
