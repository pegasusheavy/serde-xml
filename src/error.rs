//! Error types for XML serialization and deserialization.

use std::fmt::{self, Display};
use std::io;

/// Result type alias for serde_xml operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for XML serialization and deserialization.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    position: Option<Position>,
}

/// Position information for error reporting.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    /// Line number (1-indexed).
    pub line: usize,
    /// Column number (1-indexed).
    pub column: usize,
    /// Byte offset from start.
    pub offset: usize,
}

/// The kind of error that occurred.
#[derive(Debug)]
pub enum ErrorKind {
    /// An I/O error occurred.
    Io(io::Error),
    /// Unexpected end of input.
    UnexpectedEof,
    /// Invalid XML syntax.
    Syntax(String),
    /// Invalid XML name.
    InvalidName(String),
    /// Missing required attribute.
    MissingAttribute(String),
    /// Unexpected element.
    UnexpectedElement(String),
    /// Unexpected attribute.
    UnexpectedAttribute(String),
    /// Invalid value for type.
    InvalidValue(String),
    /// Unclosed tag.
    UnclosedTag(String),
    /// Mismatched closing tag.
    MismatchedTag {
        /// The expected tag name.
        expected: String,
        /// The actual tag name found.
        found: String,
    },
    /// Invalid escape sequence.
    InvalidEscape(String),
    /// Invalid UTF-8.
    InvalidUtf8,
    /// Custom error message.
    Custom(String),
    /// Unsupported operation.
    Unsupported(String),
}

impl Error {
    /// Creates a new error with the given kind.
    #[inline]
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, position: None }
    }

    /// Creates a new error with position information.
    #[inline]
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    /// Returns the error kind.
    #[inline]
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Returns the position where the error occurred.
    #[inline]
    pub fn position(&self) -> Option<Position> {
        self.position
    }

    /// Creates an unexpected EOF error.
    #[inline]
    pub fn unexpected_eof() -> Self {
        Self::new(ErrorKind::UnexpectedEof)
    }

    /// Creates a syntax error.
    #[inline]
    pub fn syntax<S: Into<String>>(msg: S) -> Self {
        Self::new(ErrorKind::Syntax(msg.into()))
    }

    /// Creates an invalid name error.
    #[inline]
    pub fn invalid_name<S: Into<String>>(name: S) -> Self {
        Self::new(ErrorKind::InvalidName(name.into()))
    }

    /// Creates an invalid value error.
    #[inline]
    pub fn invalid_value<S: Into<String>>(msg: S) -> Self {
        Self::new(ErrorKind::InvalidValue(msg.into()))
    }

    /// Creates an unclosed tag error.
    #[inline]
    pub fn unclosed_tag<S: Into<String>>(tag: S) -> Self {
        Self::new(ErrorKind::UnclosedTag(tag.into()))
    }

    /// Creates a mismatched tag error.
    #[inline]
    pub fn mismatched_tag<S: Into<String>>(expected: S, found: S) -> Self {
        Self::new(ErrorKind::MismatchedTag {
            expected: expected.into(),
            found: found.into(),
        })
    }

    /// Creates an invalid escape error.
    #[inline]
    pub fn invalid_escape<S: Into<String>>(seq: S) -> Self {
        Self::new(ErrorKind::InvalidEscape(seq.into()))
    }

    /// Creates a custom error.
    #[inline]
    pub fn custom<S: Into<String>>(msg: S) -> Self {
        Self::new(ErrorKind::Custom(msg.into()))
    }

    /// Creates an unsupported operation error.
    #[inline]
    pub fn unsupported<S: Into<String>>(msg: S) -> Self {
        Self::new(ErrorKind::Unsupported(msg.into()))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Io(e) => write!(f, "I/O error: {}", e),
            ErrorKind::UnexpectedEof => write!(f, "unexpected end of input"),
            ErrorKind::Syntax(msg) => write!(f, "syntax error: {}", msg),
            ErrorKind::InvalidName(name) => write!(f, "invalid XML name: {}", name),
            ErrorKind::MissingAttribute(name) => write!(f, "missing required attribute: {}", name),
            ErrorKind::UnexpectedElement(name) => write!(f, "unexpected element: {}", name),
            ErrorKind::UnexpectedAttribute(name) => write!(f, "unexpected attribute: {}", name),
            ErrorKind::InvalidValue(msg) => write!(f, "invalid value: {}", msg),
            ErrorKind::UnclosedTag(tag) => write!(f, "unclosed tag: <{}>", tag),
            ErrorKind::MismatchedTag { expected, found } => {
                write!(f, "mismatched closing tag: expected </{}>, found </{}>", expected, found)
            }
            ErrorKind::InvalidEscape(seq) => write!(f, "invalid escape sequence: {}", seq),
            ErrorKind::InvalidUtf8 => write!(f, "invalid UTF-8"),
            ErrorKind::Custom(msg) => write!(f, "{}", msg),
            ErrorKind::Unsupported(msg) => write!(f, "unsupported: {}", msg),
        }?;

        if let Some(pos) = self.position {
            write!(f, " at line {}, column {} (offset {})", pos.line, pos.column, pos.offset)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::new(ErrorKind::Io(e))
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::custom(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::custom(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::syntax("expected '>'");
        assert_eq!(err.to_string(), "syntax error: expected '>'");
    }

    #[test]
    fn test_error_with_position() {
        let err = Error::syntax("expected '>'")
            .with_position(Position { line: 5, column: 10, offset: 42 });
        assert_eq!(
            err.to_string(),
            "syntax error: expected '>' at line 5, column 10 (offset 42)"
        );
    }

    #[test]
    fn test_mismatched_tag_error() {
        let err = Error::mismatched_tag("foo", "bar");
        assert_eq!(
            err.to_string(),
            "mismatched closing tag: expected </foo>, found </bar>"
        );
    }

    #[test]
    fn test_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(err.to_string().contains("I/O error"));
    }

    #[test]
    fn test_custom_error() {
        let err = Error::custom("something went wrong");
        assert_eq!(err.to_string(), "something went wrong");
    }
}
