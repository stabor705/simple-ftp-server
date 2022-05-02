use std::io::ErrorKind;
use std::net::Ipv4Addr;

use crate::AuthError;
use crate::CommandError;
use crate::HostPort;

use strum::EnumMessage;
use strum_macros::EnumMessage;

use anyhow::Error;

#[allow(dead_code)]
#[derive(EnumMessage, PartialEq)]
pub enum Reply {
    #[strum(message = "Opening data connection")]
    OpeningDataConnection,

    #[strum(message = "Command okay")]
    CommandOk,
    #[strum(message = "Command not implemented, superfluous at this site")]
    CommandNotImplemented,
    // 211
    #[strum(message = "Directory status")]
    DirectoryStatus,
    //214
    //215
    #[strum(message = "Service ready for new user")]
    ServiceReady,
    #[strum(message = "Service closing control connection")]
    ServiceClosing,
    #[strum(message = "Data connection open; no transfer in progress")]
    DataConnectionOpen,
    #[strum(message = "Closing data connection. Requested file action successful")]
    ClosingDataConnection,
    #[strum(message = "Entering passive mode ({})")]
    EnteringPassiveMode(HostPort),
    #[strum(message = "User logged in, proceed")]
    UserLoggedIn,
    #[strum(message = "Requested file action okay, proceed")]
    FileActionOk,
    #[strum(message = "\"{}\" created")]
    Created(String),

    #[strum(message = "User name okay, need password")]
    UsernameOk,
    //332
    #[strum(message = "Requested file action pending further information")]
    PendingFurtherInformation,

    #[strum(message = "Service not available, closing control connection")]
    ServiceNotAvailable,
    #[strum(message = "Can't open data connection")]
    CantOpenDataConnection,
    #[strum(message = "Connection closed; transfer aborted")]
    ConnectionClosed,
    #[strum(message = "Requested file action not taken. File unavailable")]
    FileActionNotTaken,
    #[strum(message = "Requested action aborted: local error in processing")]
    LocalProcessingError,
    #[strum(message = "Requested action not taken. Insufficient storage space in system")]
    InsufficientStorageSpace,

    #[strum(message = "Syntax error, command unrecognized")]
    SyntaxError,
    #[strum(message = "Syntax error in parameters or arguments")]
    SyntaxErrorArg,
    #[strum(message = "Command not implemented")]
    NotImplemented,
    #[strum(message = "Bad sequence of commands")]
    BadCommandSequence,
    #[strum(message = "Command not implemented for that parameter")]
    BadParameter,
    #[strum(message = "Not logged in")]
    NotLoggedIn,
    #[strum(message = "Need account for storing files")]
    NeedAccountForStoring,
    #[strum(message = "Requested action not taken. File unavailable")]
    FileUnavailable,
    #[strum(message = "Requested action aborted: page type unknown")]
    PageTypeUnknown,
    #[strum(message = "Requested file action aborted. Exceeded storage allocation")]
    ExceededStorageAllocation,
    #[strum(message = "Requested action not taken. File name not allowed")]
    FileNameNotAllowed,
}

impl Reply {
    fn status_code(&self) -> u32 {
        use Reply::*;
        match self {
            OpeningDataConnection => 150,

            CommandOk => 200,
            CommandNotImplemented => 202,
            // 211
            DirectoryStatus => 212,
            //214
            //215
            ServiceReady => 220,
            ServiceClosing => 221,
            DataConnectionOpen => 225,
            ClosingDataConnection => 226,
            EnteringPassiveMode(_) => 227,
            UserLoggedIn => 230,
            FileActionOk => 250,
            Created(_) => 257,

            UsernameOk => 331,
            //332
            PendingFurtherInformation => 350,

            ServiceNotAvailable => 421,
            CantOpenDataConnection => 425,
            ConnectionClosed => 426,
            FileActionNotTaken => 450,
            LocalProcessingError => 451,
            InsufficientStorageSpace => 452,

            SyntaxError => 500,
            SyntaxErrorArg => 501,
            NotImplemented => 502,
            BadCommandSequence => 503,
            BadParameter => 504,
            NotLoggedIn => 530,
            NeedAccountForStoring => 532,
            FileUnavailable => 550,
            PageTypeUnknown => 551,
            ExceededStorageAllocation => 552,
            FileNameNotAllowed => 553,
        }
    }
}

impl ToString for Reply {
    fn to_string(&self) -> String {
        use Reply::*;
        let response = format!("{} {}", self.status_code(), self.get_message().unwrap());
        match self {
            EnteringPassiveMode(host_port) => {
                response.replace("{}", host_port.to_string().as_str())
            }
            Created(pathname) => response.replace("{}", pathname),
            _ => response,
        }
    }
}

//TODO:
impl From<Error> for Reply {
    fn from(e: Error) -> Self {
        use Reply::*;

        if e.is::<CommandError>() {
            let err: CommandError = e.downcast().unwrap();
            match err {
                CommandError::ArgMissing => SyntaxErrorArg,
                CommandError::BadArg => BadParameter,
                CommandError::InvalidCommand => SyntaxError,
            }
        } else if e.is::<std::io::Error>() {
            let err: std::io::Error = e.downcast().unwrap();
            match err.kind() {
                ErrorKind::NotFound => FileUnavailable,
                ErrorKind::PermissionDenied => FileUnavailable,
                ErrorKind::ConnectionRefused => ConnectionClosed,
                ErrorKind::ConnectionReset => ConnectionClosed,
                ErrorKind::ConnectionAborted => ConnectionClosed,
                ErrorKind::AlreadyExists => FileNameNotAllowed,
                ErrorKind::InvalidInput => SyntaxErrorArg,
                //This one can mean requesting ascii type for binary data
                ErrorKind::InvalidData => BadCommandSequence,
                // I think this one is used when client doesn't send anything on data connection
                ErrorKind::TimedOut => CantOpenDataConnection,
                ErrorKind::WriteZero => LocalProcessingError,
                ErrorKind::OutOfMemory => LocalProcessingError,
                _ => {
                    log::error!("Encountered unexpected io error {}", err);
                    LocalProcessingError
                }
            }
        } else if e.is::<AuthError>() {
            let err: AuthError = e.downcast().unwrap();
            match err {
                AuthError::NotLoggedIn => NotLoggedIn,
                AuthError::PwdWhileNotLoggedIn => FileUnavailable,
            }
        } else {
            log::error!("Encountered unexpected error {}", e);
            LocalProcessingError
        }
    }
}

#[allow(unused_imports)] // For some reason compiler thinks super::* is not use
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reply_creation() {
        let reply = Reply::CommandOk;
        assert_eq!(reply.to_string(), "200 Command okay");
        let reply = Reply::EnteringPassiveMode(HostPort {
            ip: Ipv4Addr::LOCALHOST,
            port: 8888,
        });
        assert_eq!(
            reply.to_string(),
            "227 Entering passive mode (127,0,0,1,34,184)"
        );
        let reply = Reply::Created("very-important-directory".to_owned());
        assert_eq!(
            reply.to_string(),
            "257 \"very-important-directory\" created"
        )
    }
}
