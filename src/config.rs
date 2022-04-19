use std::default::Default;
use std::net::{IpAddr, Ipv4Addr};
use std::net::IpAddr::V4;

pub struct Config {
    pub ip: IpAddr,
    pub control_port: u16,
    pub dir_root: String
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ip: V4(Ipv4Addr::LOCALHOST),
            control_port: 0,
            dir_root: ".".to_owned()
        }
    }
}