//! # Error definitions
//!
//! This module contains the `Error` and `Result` types used throughout the
//! whole software.

use std::fmt::{Display, Formatter};
use std::io;

/// An error from dhstore.
///
/// This represents all the errors that can happen anywhere.
#[derive(Debug)]
pub enum Error {
    IoError(&'static str, io::Error),
    CorruptedStore(&'static str),
    InvalidInput(&'static str),
}

impl Display for Error {
    fn fmt(&self,
           f: &mut Formatter)
           -> ::std::result::Result<(), ::std::fmt::Error> {
        match *self {
            Error::IoError(msg, ref err) => {
                write!(f, "I/O error: {}\n", msg)?;
                err.fmt(f)
            }
            Error::CorruptedStore(msg) => {
                write!(f, "Corrupted store: {}", msg)
            }
            Error::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(_, _) => "I/O error",
            Error::CorruptedStore(_) => "Corrupted store",
            Error::InvalidInput(_) => "Invalid input",
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::IoError(_, ref o_error) => Some(o_error),
            _ => None,
        }
    }
}

impl From<(&'static str, io::Error)> for Error {
    fn from((msg, err): (&'static str, io::Error)) -> Error {
        Error::IoError(msg, err)
    }
}

/// Alias for the `Result` type with an error of our `Error` type.
pub type Result<T> = ::std::result::Result<T, Error>;
