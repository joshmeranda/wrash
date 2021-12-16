use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum WrashError {
    NonZeroExit(i32),
    FailedIo(std::io::Error),
    Custom(String),
}

impl Display for WrashError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WrashError::NonZeroExit(n) => write!(f, "command exited with nonzero exit code '{}'", n),
            WrashError::FailedIo(err) => write!(f, "failed io operation: {}", err),
            WrashError::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl PartialEq for WrashError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WrashError::NonZeroExit(left), WrashError::NonZeroExit(right)) => right == left,
            // right now we don't care too much about the specifics of the error only that they are the right type
            (WrashError::FailedIo(left), WrashError::FailedIo(right)) => left.kind() == right.kind(),
            (WrashError::Custom(left), WrashError::Custom(right)) => left == right,
            _ => false,
            // _ => self == other
        }
    }
}

impl Error for WrashError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WrashError::FailedIo(err) => Some(err),
            _ => None
        }
    }
}

impl From<i32> for WrashError {
    fn from(n: i32) -> Self { WrashError::NonZeroExit(n) }
}

impl From<std::io::Error> for WrashError {
    fn from(err: std::io::Error) -> Self { WrashError::FailedIo(err) }
}

impl From<String> for WrashError {
    fn from(s: String) -> Self { WrashError::Custom(s) }
}