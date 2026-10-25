// src/input.rs

use crate::error::{LogShrubsError, Result};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, IsTerminal};
use std::path::{Path, PathBuf};

/// Label used for standard input in reports and error messages.
pub const STDIN_LABEL: &str = "<stdin>";

/// One place log lines can come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Stdin,
    File(PathBuf),
}

impl Source {
    /// Human-readable name used in reports.
    pub fn label(&self) -> String {
        match self {
            Source::Stdin => STDIN_LABEL.to_string(),
            Source::File(p) => p.display().to_string(),
        }
    }

    fn path_for_errors(&self) -> PathBuf {
        match self {
            Source::Stdin => PathBuf::from(STDIN_LABEL),
            Source::File(p) => p.clone(),
        }
    }
}

/// Turn the paths given on the command line into concrete sources.
///
/// - no paths        → stdin (error if stdin is an interactive terminal)
/// - `-`             → stdin
/// - a file          → that file (any extension)
/// - a directory     → every `.log` / `.txt` file inside (recursive unless
///   `recursive` is false); error if none are found
pub fn resolve_inputs(paths: &[PathBuf], recursive: bool) -> Result<Vec<Source>> {
    if paths.is_empty() {
        if std::io::stdin().is_terminal() {
            return Err(LogShrubsError::NoInput);
        }
        return Ok(vec![Source::Stdin]);
    }

    let mut out = Vec::new();
    for p in paths {
        if p.as_os_str() == "-" {
            out.push(Source::Stdin);
            continue;
        }

        let meta = fs::metadata(p).map_err(|e| LogShrubsError::from_io(p, e))?;
        if meta.is_dir() {
            let mut found = Vec::new();
            collect_dir(p, recursive, &mut found).map_err(|e| LogShrubsError::from_io(p, e))?;
            if found.is_empty() {
                return Err(LogShrubsError::EmptyDirectory(p.clone()));
            }
            found.sort();
            out.extend(found.into_iter().map(Source::File));
        } else {
            out.push(Source::File(p.clone()));
        }
    }
    Ok(out)
}

fn is_log_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("log") || e.eq_ignore_ascii_case("txt"))
        .unwrap_or(false)
}

fn collect_dir(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            // Symlinked directories are not followed (avoids loops).
            if recursive {
                collect_dir(&path, recursive, out)?;
            }
        } else if is_log_file(&path) && fs::metadata(&path).map(|m| m.is_file()).unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

/// Total size in bytes of the inputs, only when every input is a file.
pub fn total_size(sources: &[Source]) -> Option<u64> {
    let mut total = 0u64;
    for s in sources {
        match s {
            Source::Stdin => return None,
            Source::File(p) => total += fs::metadata(p).ok()?.len(),
        }
    }
    Some(total)
}

/// Streaming line reader. Yields `Err` (with the source path) if reading
/// fails mid-file instead of silently stopping. Invalid UTF-8 bytes are
/// replaced rather than treated as errors.
pub struct LogReader {
    path: PathBuf,
    reader: Box<dyn BufRead + Send>,
    buf: Vec<u8>,
    done: bool,
}

impl LogReader {
    pub fn open(source: &Source) -> Result<Self> {
        let path = source.path_for_errors();
        let reader: Box<dyn BufRead + Send> = match source {
            Source::Stdin => Box::new(BufReader::new(std::io::stdin())),
            Source::File(p) => {
                let f = File::open(p).map_err(|e| LogShrubsError::from_io(p, e))?;
                Box::new(BufReader::new(f))
            }
        };
        Ok(Self::from_reader(path, reader))
    }

    pub fn from_reader(path: PathBuf, reader: Box<dyn BufRead + Send>) -> Self {
        Self {
            path,
            reader,
            buf: Vec::new(),
            done: false,
        }
    }
}

impl Iterator for LogReader {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        self.buf.clear();
        match self.reader.read_until(b'\n', &mut self.buf) {
            Ok(0) => {
                self.done = true;
                None
            }
            Ok(_) => {
                if self.buf.last() == Some(&b'\n') {
                    self.buf.pop();
                }
                if self.buf.last() == Some(&b'\r') {
                    self.buf.pop();
                }
                Some(Ok(String::from_utf8_lossy(&self.buf).into_owned()))
            }
            Err(e) => {
                self.done = true;
                Some(Err(LogShrubsError::from_io(&self.path, e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    #[test]
    fn directory_is_recursive_by_default_and_filters_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.log"), "x\n").unwrap();
        fs::write(root.join("b.txt"), "x\n").unwrap();
        fs::write(root.join("c.dat"), "x\n").unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("d.LOG"), "x\n").unwrap();

        let all = resolve_inputs(&[root.to_path_buf()], true).unwrap();
        assert_eq!(all.len(), 3);

        let top = resolve_inputs(&[root.to_path_buf()], false).unwrap();
        assert_eq!(top.len(), 2);
    }

    #[test]
    fn empty_directory_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("x.dat"), "x").unwrap();
        let err = resolve_inputs(&[dir.path().to_path_buf()], true).unwrap_err();
        assert!(matches!(err, LogShrubsError::EmptyDirectory(_)));
    }

    #[test]
    fn dash_means_stdin() {
        let s = resolve_inputs(&[PathBuf::from("-")], true).unwrap();
        assert_eq!(s, vec![Source::Stdin]);
    }

    #[test]
    fn missing_path_is_file_not_found() {
        let err = resolve_inputs(&[PathBuf::from("nope.log")], true).unwrap_err();
        assert!(matches!(err, LogShrubsError::FileNotFound(_)));
    }

    #[test]
    fn reader_strips_newlines_and_survives_bad_utf8() {
        let data: Vec<u8> = b"one\r\ntwo\n\xff\xfe three\nlast".to_vec();
        let r = LogReader::from_reader(PathBuf::from("mem"), Box::new(Cursor::new(data)));
        let lines: Vec<String> = r.map(|l| l.unwrap()).collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "one");
        assert_eq!(lines[1], "two");
        assert!(lines[2].contains("three"));
        assert_eq!(lines[3], "last");
    }

    struct Failing(bool);
    impl Read for Failing {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if !self.0 {
                self.0 = true;
                buf[..4].copy_from_slice(b"ok\n\0");
                Ok(3)
            } else {
                Err(std::io::Error::other("disk gone"))
            }
        }
    }

    #[test]
    fn mid_file_errors_are_surfaced_with_path() {
        let r = LogReader::from_reader(
            PathBuf::from("big.log"),
            Box::new(BufReader::new(Failing(false))),
        );
        let items: Vec<_> = r.collect();
        assert_eq!(items.len(), 2);
        assert!(items[0].is_ok());
        let msg = items[1].as_ref().unwrap_err().to_string();
        assert!(msg.contains("big.log"), "{}", msg);
        assert!(msg.contains("disk gone"), "{}", msg);
    }
}
