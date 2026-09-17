// src/error.rs

use std::path::PathBuf;
use thiserror::Error;

/// All errors LogLens can produce, with user-friendly messages.
#[derive(Debug, Error)]
pub enum LogLensError {
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    #[error("permission denied reading: {0}")]
    PermissionDenied(PathBuf),

    #[error("input is not valid UTF-8: {0}")]
    InvalidUtf8(PathBuf),

    #[error("I/O error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

impl LogLensError {
    /// Convert an std::io::Error into a domain-specific LogLensError,
    /// using the given path for context.
    pub fn from_io(path: &std::path::Path, err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => LogLensError::FileNotFound(path.to_path_buf()),
            ErrorKind::PermissionDenied => LogLensError::PermissionDenied(path.to_path_buf()),
            ErrorKind::InvalidData => LogLensError::InvalidUtf8(path.to_path_buf()),
            _ => LogLensError::Io {
                path: path.to_path_buf(),
                source: err,
            },
        }
    }
}

pub type Result<T> = std::result::Result<T, LogLensError>;