//! Error results that can occur working with a Clock
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::fmt::{Debug, Display, Formatter, Result};

/// An error that can occur when working with a Clock
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    /// Create a new Error with a given ErrorKind variant
    pub fn new(kind: ErrorKind) -> Error {
        Error { kind }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.kind)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.kind)
    }
}

/// The kinds of errors that can occur when working with a Clock.
pub enum ErrorKind {
    /// Generic error type
    Message(String),

    /// The clock is already running
    ClockAlreadyRunning,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ErrorKind::Message(message) => write!(f, "An error occurred: {}", message),
            ErrorKind::ClockAlreadyRunning => {
                write!(f, "Clock is already running")
            }
        }
    }
}

impl ErrorKind {
    /// Return a new generic ErrorKind::Message with a given string message.
    pub fn new(message: &str) -> ErrorKind {
        ErrorKind::Message(message.to_string())
    }
}
