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
            WrashErrorInner::FailedIo(err) => write!(f, "failed io operation: {}", err),
            WrashErrorInner::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl PartialEq for WrashErrorInner {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WrashErrorInner::NonZeroExit(left), WrashErrorInner::NonZeroExit(right)) => right == left,
            // right now we don't care too much about the specifics of the error only that they are the right type
            (WrashErrorInner::FailedIo(left), WrashErrorInner::FailedIo(right)) => left.kind() == right.kind(),
            (WrashErrorInner::Custom(left), WrashErrorInner::Custom(right)) => left == right,
            _ => false,
            // _ => self == other
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