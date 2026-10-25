// src/error.rs

use std::path::PathBuf;
use thiserror::Error;

/// All errors LogShrubs can produce, with user-friendly messages.
#[derive(Debug, Error)]
pub enum LogShrubsError {
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

    #[error("no .log or .txt files found in directory: {0}")]
    EmptyDirectory(PathBuf),

    #[error("no input: pass a file or directory, use '-' for stdin, or pipe data into logshrubs")]
    NoInput,

    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("watch failed: {0}")]
    Watch(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl LogShrubsError {
    /// Convert an std::io::Error into a domain-specific LogShrubsError,
    /// using the given path for context.
    pub fn from_io(path: &std::path::Path, err: std::io::Error) -> Self {
        use std::io::ErrorKind;
        match err.kind() {
            ErrorKind::NotFound => LogShrubsError::FileNotFound(path.to_path_buf()),
            ErrorKind::PermissionDenied => LogShrubsError::PermissionDenied(path.to_path_buf()),
            ErrorKind::InvalidData => LogShrubsError::InvalidUtf8(path.to_path_buf()),
            _ => LogShrubsError::Io {
                path: path.to_path_buf(),
                source: err,
            },
        }
    }
}

pub type Result<T> = std::result::Result<T, LogShrubsError>;
