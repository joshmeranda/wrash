use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum ArgumentError {
    /// Found end of line but more content was expected or needed for proper
    /// argument parsing.
    UnexpectedEndOfLine,

    /// A sequence was started but not properlly ended.
    UnterminatedSequence(char),

    /// An invalid escape sequence was found.
    InvalidEscape(char),
}

impl Display for ArgumentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgumentError::UnexpectedEndOfLine => write!(f, "received unexpected end of line"),
            ArgumentError::UnterminatedSequence(c) => write!(f, "received unterminated '{}' sequence", c),
            ArgumentError::InvalidEscape(c) => write!(f, "received invalid escpace character'{}'", c),
        }
    }
}

impl Error for ArgumentError { }