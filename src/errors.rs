use std::fmt::{Display, Formatter};
use std::io;

#[derive(Debug)]
pub enum Error {
    IoError(&'static str, io::Error),
    CorruptedStore(&'static str),
}

impl Display for Error {
    fn fmt(&self,
           f: &mut Formatter)
           -> ::std::result::Result<(), ::std::fmt::Error> {
        match *self {
            Error::IoError(ref msg, ref err) => {
                write!(f, "I/O error: {}", msg)?;
                err.fmt(f)
            }
            Error::CorruptedStore(ref msg) => {
                write!(f, "Corrupted store: {}", msg)
            }
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(_, _) => "I/O error",
            Error::CorruptedStore(_) => "Corrupted store",
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::IoError(_, ref o_error) => {
                Some(o_error)
            }
            _ => None,
        }
    }
}

impl From<(&'static str, io::Error)> for Error {
    fn from((msg, err): (&'static str, io::Error)) -> Error {
        Error::IoError(msg, err)
    }
}

pub type Result<T> = ::std::result::Result<T, Error>;
