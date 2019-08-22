use actix::MailboxError;
use failure::Fail;
use std::io;
use tokio::timer;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Actix mailbox error")]
    MailboxError(MailboxError),

    #[fail(display = "Tokio timer error")]
    TimerError(timer::Error),

    #[fail(display = "I/O error")]
    IOError(io::Error),

    #[fail(display = "Actix WAMP error")]
    WampError(actix_wamp::Error),

    #[fail(display = "Golem RPC error")]
    GolemRPCError(golem_rpc_api::Error),

    #[fail(display = "Keyboard interrupt")]
    KeyboardInterrupt(tokio_ctrlc_error::KeyboardInterrupt),

    #[fail(display = "Tokio Ctrl-C error")]
    CtrlcError(tokio_ctrlc_error::IoError),

    #[fail(display = "Chrono error")]
    ChronoError(chrono::ParseError),

    #[fail(display = "Zero timeout error")]
    ZeroTimeoutError,
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<MailboxError> for Error {
    fn from(err: MailboxError) -> Self {
        Error::MailboxError(err)
    }
}

impl From<timer::Error> for Error {
    fn from(err: timer::Error) -> Self {
        Error::TimerError(err)
    }
}

impl From<actix_wamp::Error> for Error {
    fn from(err: actix_wamp::Error) -> Self {
        Error::WampError(err)
    }
}

impl From<golem_rpc_api::Error> for Error {
    fn from(err: golem_rpc_api::Error) -> Self {
        Error::GolemRPCError(err)
    }
}

impl From<tokio_ctrlc_error::IoError> for Error {
    fn from(err: tokio_ctrlc_error::IoError) -> Self {
        Error::CtrlcError(err)
    }
}

impl From<tokio_ctrlc_error::KeyboardInterrupt> for Error {
    fn from(err: tokio_ctrlc_error::KeyboardInterrupt) -> Self {
        Error::KeyboardInterrupt(err)
    }
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Error::ChronoError(err)
    }
}
