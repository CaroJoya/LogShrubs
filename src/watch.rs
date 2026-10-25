// src/watch.rs
//
// `logshrubs watch app.log`: follow a file and keep a live summary of
// level counts, recent errors and the current spike status.

use crate::analyzer::spikes::floor_to_bucket;
use crate::analyzer::{ErrorAccumulator, HttpAccumulator, StatsAccumulator};
use crate::error::{LogShrubsError, Result};
use crate::model::{ErrorSpike, Stats};
use crate::parser::{self, Format, FormatChoice};
use chrono::{DateTime, Utc};
use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, IsTerminal, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

const RECENT_ERRORS: usize = 5;

#[derive(Debug, Clone, Serialize)]
pub struct RecentError {
    pub timestamp: Option<DateTime<Utc>>,
    pub message: String,
}

/// Everything accumulated so far for the followed file.
#[derive(Default)]
struct Live {
    lines: u64,
    stats: StatsAccumulator,
    errors: ErrorAccumulator,
    http: HttpAccumulator,
    recent: VecDeque<RecentError>,
    last_error_ts: Option<DateTime<Utc>>,
}

impl Live {
    fn observe(&mut self, line: &str, format: Format) {
        self.lines += 1;
        let event = parser::parse_line(line, format);
        self.stats.observe(&event);
        self.errors.observe(&event);
        self.http.observe(&event);
        if event.level.is_some_and(|l| l.is_error()) {
            if event.timestamp.is_some() {
                self.last_error_ts = event.timestamp;
            }
            if self.recent.len() == RECENT_ERRORS {
                self.recent.pop_front();
            }
            self.recent.push_back(RecentError {
                timestamp: event.timestamp,
                message: event.message,
            });
        }
    }
}

/// Incremental reader for a growing file. Handles partial last lines,
/// truncation and rotation (by resetting the live state and re-reading).
pub struct Tailer {
    path: PathBuf,
    choice: FormatChoice,
    format: Option<Format>,
    offset: u64,
    #[cfg(unix)]
    inode: Option<u64>,
    resets: u32,
    /// The file vanished since the last poll (rotation in progress).
    was_missing: bool,
    live: Live,
}

#[derive(Debug, Serialize)]
pub struct SpikeStatus {
    pub active: bool,
    pub current: Option<ErrorSpike>,
    pub total_spikes: usize,
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub source: String,
    pub lines: u64,
    pub format: Option<&'static str>,
    pub stats: Stats,
    pub total_errors: u64,
    pub recent_errors: Vec<RecentError>,
    pub spike: SpikeStatus,
    pub http_requests: u64,
}

impl Tailer {
    pub fn new(path: PathBuf, choice: FormatChoice) -> Self {
        let format = match choice {
            FormatChoice::Fixed(f) => Some(f),
            FormatChoice::Auto => None,
        };
        Self {
            path,
            choice,
            format,
            offset: 0,
            #[cfg(unix)]
            inode: None,
            resets: 0,
            was_missing: false,
            live: Live::default(),
        }
    }

    #[cfg(test)]
    pub fn resets(&self) -> u32 {
        self.resets
    }

    fn reset(&mut self) {
        self.offset = 0;
        self.live = Live::default();
        self.resets += 1;
        if self.choice == FormatChoice::Auto {
            self.format = None;
        }
    }

    /// Read whatever was appended since the last call.
    /// Returns `Ok(true)` when the summary changed.
    pub fn poll(&mut self) -> std::io::Result<bool> {
        let meta = match fs::metadata(&self.path) {
            Ok(m) => m,
            // Mid-rotation the file may briefly not exist: just wait.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.was_missing = true;
                return Ok(false);
            }
            Err(e) => return Err(e),
        };
        let len = meta.len();
        // A file that disappeared and came back is a new file, even if the
        // OS reused its inode and it is already longer than our offset.
        let reappeared = std::mem::take(&mut self.was_missing);

        #[cfg(unix)]
        let rotated = {
            use std::os::unix::fs::MetadataExt;
            let ino = meta.ino();
            let r = self.inode.is_some_and(|old| old != ino);
            self.inode = Some(ino);
            r
        };
        #[cfg(not(unix))]
        let rotated = false;

        let mut changed = false;
        if rotated || reappeared || len < self.offset {
            self.reset();
            changed = true;
        }
        if len == self.offset {
            return Ok(changed);
        }

        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.offset))?;
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        let mut lines: Vec<String> = Vec::new();
        loop {
            buf.clear();
            let n = reader.read_until(b'\n', &mut buf)?;
            if n == 0 || buf.last() != Some(&b'\n') {
                // EOF, or a line the writer hasn't finished yet.
                break;
            }
            self.offset += n as u64;
            buf.pop();
            if buf.last() == Some(&b'\r') {
                buf.pop();
            }
            lines.push(String::from_utf8_lossy(&buf).into_owned());
        }
        if lines.is_empty() {
            return Ok(changed);
        }

        let format = match self.format {
            Some(f) => f,
            None => {
                let f = parser::detect(&lines);
                self.format = Some(f);
                f
            }
        };
        for line in &lines {
            self.live.observe(line, format);
        }
        Ok(true)
    }

    pub fn snapshot(&self, top_n: usize) -> Snapshot {
        let stats = self.live.stats.clone().finish();
        let errors = self.live.errors.clone().finish(top_n);
        let http = self.live.http.clone().finish(top_n);

        let current = self.live.last_error_ts.and_then(|ts| {
            let bucket = floor_to_bucket(ts);
            errors
                .spikes
                .iter()
                .find(|s| s.bucket_start == bucket)
                .cloned()
        });

        Snapshot {
            source: self.path.display().to_string(),
            lines: self.live.lines,
            format: self.format.map(|f| f.name()),
            stats,
            total_errors: errors.total,
            recent_errors: self.live.recent.iter().cloned().collect(),
            spike: SpikeStatus {
                active: current.is_some(),
                current,
                total_spikes: errors.spikes.len(),
            },
            http_requests: http.total_requests,
        }
    }
}

fn print_snapshot(s: &Snapshot) {
    println!("WATCH  {}", s.source);
    println!("─────────────────────────────────");
    println!(
        "Lines       {}   Format {}",
        s.lines,
        s.format.unwrap_or("(waiting for data)")
    );
    println!();
    println!(
        "INFO {:>6}   WARN {:>6}   ERROR {:>6}",
        s.stats.info, s.stats.warn, s.stats.error
    );
    println!(
        "FATAL {:>5}   DEBUG {:>5}   TRACE {:>6}",
        s.stats.fatal, s.stats.debug, s.stats.trace
    );
    if s.http_requests > 0 {
        println!("HTTP requests {}", s.http_requests);
    }
    println!();

    println!("SPIKE STATUS");
    println!("─────────────────────────────────");
    match &s.spike.current {
        Some(sp) => println!(
            "⚠ SPIKE  {} → {}   {} errors   {:.1}× normal",
            sp.bucket_start.format("%H:%M"),
            sp.bucket_end.format("%H:%M"),
            sp.count,
            sp.severity
        ),
        None => println!(
            "normal ({} earlier spike(s) in this log)",
            s.spike.total_spikes
        ),
    }
    println!();

    println!("RECENT ERRORS  ({} total)", s.total_errors);
    println!("─────────────────────────────────");
    if s.recent_errors.is_empty() {
        println!("(none)");
    }
    for e in s.recent_errors.iter().rev() {
        let ts = e
            .timestamp
            .map(|t| t.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "--:--:--".to_string());
        let msg: String = if e.message.chars().count() > 70 {
            let mut m: String = e.message.chars().take(69).collect();
            m.push('…');
            m
        } else {
            e.message.clone()
        };
        println!("{}  {}", ts, msg);
    }
}

fn emit(snapshot: &Snapshot, json: bool, clear: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string(snapshot)?);
    } else {
        if clear {
            print!("\x1b[2J\x1b[H");
        } else {
            println!();
        }
        print_snapshot(snapshot);
    }
    Ok(())
}

/// Run the watch loop. With `once`, analyze the current content, print
/// one summary and return.
pub fn run(
    file: &Path,
    choice: FormatChoice,
    top_n: usize,
    interval_ms: u64,
    once: bool,
    json: bool,
) -> Result<()> {
    fs::metadata(file).map_err(|e| LogShrubsError::from_io(file, e))?;
    let clear = !json && std::io::stdout().is_terminal();

    let mut tailer = Tailer::new(file.to_path_buf(), choice);
    tailer
        .poll()
        .map_err(|e| LogShrubsError::from_io(file, e))?;
    emit(&tailer.snapshot(top_n), json, clear)?;
    if once {
        return Ok(());
    }

    // Watch the parent directory so rotation (rename + recreate) is seen.
    let (tx, rx) = mpsc::channel();
    let watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    });
    let mut _watcher = match watcher {
        Ok(mut w) => {
            let dir = match file.parent() {
                Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                _ => PathBuf::from("."),
            };
            w.watch(&dir, RecursiveMode::NonRecursive)
                .map_err(|e| LogShrubsError::Watch(e.to_string()))?;
            Some(w)
        }
        // No file-system events available: fall back to polling on the tick.
        Err(_) => None,
    };

    let tick = Duration::from_millis(interval_ms.max(50));
    loop {
        if let Err(mpsc::RecvTimeoutError::Disconnected) = rx.recv_timeout(tick) {
            std::thread::sleep(tick);
        }
        while rx.try_recv().is_ok() {}

        match tailer.poll() {
            Ok(true) => emit(&tailer.snapshot(top_n), json, clear)?,
            Ok(false) => {}
            Err(e) => eprintln!("warning: {}: {}", file.display(), e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn append(path: &Path, text: &str) {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(text.as_bytes()).unwrap();
    }

    #[test]
    fn reads_incrementally_and_waits_for_complete_lines() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("app.log");
        append(&p, "2026-09-15 14:00:00 INFO a\n2026-09-15 14:00:01 ERR");
        let mut t = Tailer::new(p.clone(), FormatChoice::Auto);
        assert!(t.poll().unwrap());
        assert_eq!(t.snapshot(5).lines, 1);

        append(&p, "OR boom\n");
        assert!(t.poll().unwrap());
        let s = t.snapshot(5);
        assert_eq!(s.lines, 2);
        assert_eq!(s.stats.error, 1);
        assert_eq!(s.recent_errors[0].message, "boom");

        assert!(!t.poll().unwrap());
    }

    #[test]
    fn truncation_resets_without_crashing() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("app.log");
        append(
            &p,
            "2026-09-15 14:00:00 ERROR one\n2026-09-15 14:00:01 ERROR two\n",
        );
        let mut t = Tailer::new(p.clone(), FormatChoice::Auto);
        t.poll().unwrap();
        assert_eq!(t.snapshot(5).stats.error, 2);

        fs::write(&p, "2026-09-15 15:00:00 INFO fresh\n").unwrap();
        assert!(t.poll().unwrap());
        let s = t.snapshot(5);
        assert_eq!(t.resets(), 1);
        assert_eq!(s.lines, 1);
        assert_eq!(s.stats.error, 0);
        assert_eq!(s.stats.info, 1);
    }

    #[test]
    fn missing_file_mid_rotation_is_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("app.log");
        append(&p, "2026-09-15 14:00:00 INFO a\n");
        let mut t = Tailer::new(p.clone(), FormatChoice::Auto);
        t.poll().unwrap();
        fs::remove_file(&p).unwrap();
        assert!(!t.poll().unwrap());
        append(&p, "2026-09-15 14:05:00 ERROR b\n");
        assert!(t.poll().unwrap());
        assert_eq!(t.snapshot(5).stats.error, 1);
    }

    #[cfg(unix)]
    #[test]
    fn rotation_to_new_file_is_detected_even_if_bigger() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("app.log");
        append(&p, "2026-09-15 14:00:00 INFO a\n");
        let mut t = Tailer::new(p.clone(), FormatChoice::Auto);
        t.poll().unwrap();

        fs::rename(&p, dir.path().join("app.log.1")).unwrap();
        let mut body = String::new();
        for i in 0..10 {
            body.push_str(&format!("2026-09-15 14:1{}:00 ERROR new {}\n", i % 10, i));
        }
        append(&p, &body);
        assert!(t.poll().unwrap());
        let s = t.snapshot(5);
        assert_eq!(t.resets(), 1);
        assert_eq!(s.lines, 10);
        assert_eq!(s.stats.error, 10);
    }

    #[test]
    fn json_format_is_auto_detected_when_data_arrives() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("app.log");
        fs::write(&p, "").unwrap();
        let mut t = Tailer::new(p.clone(), FormatChoice::Auto);
        t.poll().unwrap();
        assert_eq!(t.snapshot(5).format, None);
        append(&p, "{\"level\":\"error\",\"msg\":\"x\"}\n");
        t.poll().unwrap();
        let s = t.snapshot(5);
        assert_eq!(s.format, Some("json"));
        assert_eq!(s.stats.error, 1);
    }
}
