// src/input.rs

use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;

pub struct LogReader {
    lines: Lines<BufReader<File>>,
}

impl LogReader {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self { lines: reader.lines() })
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