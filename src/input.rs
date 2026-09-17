// src/input.rs

use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::{Path, PathBuf};

pub struct LogReader {
    path: PathBuf,
    lines: Lines<BufReader<File>>,
}

impl LogReader {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf)?;
        let reader = BufReader::new(file);
        Ok(Self {
            path: path_buf,
            lines: reader.lines(),
        })
    }

    /// The path this reader was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Iterator for LogReader {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        match self.lines.next() {
            Some(Ok(line)) => Some(line),
            Some(Err(_)) => None,
            None => None,
        }
    }
}