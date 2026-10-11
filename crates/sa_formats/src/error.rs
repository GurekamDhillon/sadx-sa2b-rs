use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("unexpected end of data at offset {0:#x}")]
    Eof(usize),
    #[error("bad magic: expected {expected}, found {found:02x?}")]
    BadMagic { expected: &'static str, found: Vec<u8> },
    #[error("invalid data at offset {offset:#x}: {what}")]
    Invalid { offset: usize, what: String },
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn invalid(offset: usize, what: impl Into<String>) -> Error {
    Error::Invalid { offset, what: what.into() }
}
