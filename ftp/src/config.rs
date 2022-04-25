use std::default::Default;
use std::net::Ipv4Addr;

pub struct Config {
    pub ip: Ipv4Addr,
    pub control_port: u16,
    pub dir_root: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ip: Ipv4Addr::LOCALHOST,
            control_port: 0,
            dir_root: ".".to_owned(),
        }
    }
}
