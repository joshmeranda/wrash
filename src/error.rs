use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
pub enum WrashErrorInner {
    NonZeroExit(i32),
    FailedIo(std::io::Error),
    Custom(String),
}

impl Display for WrashErrorInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            WrashErrorInner::NonZeroExit(n) => write!(f, "command exited with nonzero exit code '{}'", n),
            WrashErrorInner::FailedIo(err) => write!(f, "would not write to writer: {}", err),
            WrashErrorInner::Custom(s) => write!(f, "{}", s),
        }
    }
}


impl Error for WrashErrorInner {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WrashErrorInner::FailedIo(err) => Some(err),
            _ => None
        }
    }
}

impl From<i32> for WrashErrorInner {
    fn from(n: i32) -> Self { WrashErrorInner::NonZeroExit(n) }
}

impl From<std::io::Error> for WrashErrorInner {
    fn from(err: std::io::Error) -> Self { WrashErrorInner::FailedIo(err) }
}

impl From<String> for WrashErrorInner {
    fn from(s: String) -> Self { WrashErrorInner::Custom(s) }
}