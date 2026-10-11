use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Format(#[from] sa_formats::Error),
    #[error("{path}: {source}")]
    Io { path: String, source: std::io::Error },
    #[error("unsupported game build: {0}")]
    Unsupported(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("not available for this game: {0}")]
    NotAvailable(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn read(path: &std::path::Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|source| Error::Io { path: path.display().to_string(), source })
}
