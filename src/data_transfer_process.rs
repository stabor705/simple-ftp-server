use crate::protocol_interpreter::Context;

use std::net::{TcpStream, SocketAddr};
use std::fs::{File};
use std::io;
use std::path::Path;
use std::io::{Read, Result, Write};
use std::str::FromStr;

use strum_macros::{Display, EnumString};
use crate::data_transfer_process::DataFormats::NonPrint;

#[derive(Display, EnumString)]
pub enum DataTypes {
    #[strum(serialize="A")]
    ASCII(DataFormats),
    #[strum(serialize="E")]
    EBCDIC(DataFormats),
    #[strum(serialize="I")]
    Image,
    #[strum(serialize="L")]
    Local(u8)
}

#[derive(Display, EnumString)]
pub enum DataFormats {
    #[strum(serialize="N")]
    NonPrint,
    #[strum(serialize="T")]
    TelnetFormatEffectors,
    #[strum(serialize="C")]
    CarriageControl
}

impl Default for DataFormats {
    fn default() -> Self {
        NonPrint
    }
}

#[derive(Display, EnumString)]
pub enum DataStructures {
    #[strum(serialize="F")]
    FileStructure,
    #[strum(serialize="R")]
    RecordStructure,
    #[strum(serialize="P")]
    PageStructure
}

#[derive(Display, EnumString)]
pub enum TransferModes {
    #[strum(serialize="S")]
    Stream,
    #[strum(serialize="B")]
    Block,
    #[strum(serialize="C")]
    Compressed
}

pub struct DataTransferProcess {
    root: String
}

impl DataTransferProcess {
    pub fn new(root: String) -> DataTransferProcess {
        DataTransferProcess { root }
    }

    pub fn send_file(&self, arg: &str, ctx: &Context) -> Result<()> {
        let mut stream = TcpStream::connect(SocketAddr::new(ctx.ip, ctx.data_port))?;
        let path = Path::new(&self.root).join(arg);
        let mut file = File::open(path)?;
        loop {
            //TODO: How big should it be?
            let mut buf = [0; 512];
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            stream.write_all(&buf[0..n]);
        }
        Ok(())
    }

    pub fn store(&self, arg: &str, ctx: &Context) -> Result<()> {
        let mut file = File::create(Path::new(&self.root).join(arg))?;
        let mut stream = TcpStream::connect(SocketAddr::new(ctx.ip, ctx.data_port))?;
        loop {
            //TODO: How big should it be?
            let mut buf = [0; 512];
            let n = stream.read(&mut buf)?;
            if n == 0 { break; }
            file.write_all(&buf)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializing_parameters() {
        let data_type = "A N".parse::<DataType>()?;
        assert_eq!(data_type.data_type, DataTypes::ASCII);
        assert_eq!(data_type.data_format, DataFormats::NonPrint);
    }
}