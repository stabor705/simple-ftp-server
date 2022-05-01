use std::fmt::Debug;
use std::net::Ipv4Addr;
use std::str::FromStr;

use fallible_iterator::FallibleIterator;

#[derive(PartialEq)]
pub struct HostPort {
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl HostPort {
    pub fn new(ip: Ipv4Addr, port: u16) -> HostPort {
        HostPort { ip, port }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Could not parse hostport address")]
pub struct ParseHostPortError {}

impl FromStr for HostPort {
    type Err = ParseHostPortError;
    fn from_str(s: &str) -> Result<HostPort, ParseHostPortError> {
        let nums: Vec<u8> = fallible_iterator::convert(s.split(',').map(|c| c.parse::<u8>()))
            .collect()
            .map_err(|_| ParseHostPortError {})?;
        if nums.len() < 6 {
            return Err(ParseHostPortError {});
        }
        let ip = Ipv4Addr::new(nums[0], nums[1], nums[2], nums[3]);
        let port = ((nums[4] as u16) << 8) + nums[5] as u16;
        Ok(HostPort { ip, port })
    }
}

impl ToString for HostPort {
    fn to_string(&self) -> String {
        let ip = self.ip.octets();
        let p1 = self.port >> 8;
        let p2 = self.port & 0xFF;
        format!("{},{},{},{},{},{}", ip[0], ip[1], ip[2], ip[3], p1, p2)
    }
}

impl Default for HostPort {
    fn default() -> Self {
        HostPort {
            ip: Ipv4Addr::LOCALHOST,
            port: 0,
        }
    }
}
