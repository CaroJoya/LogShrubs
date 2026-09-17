// src/parser/text.rs

use crate::model::{Level, LogEvent};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

pub fn parse(line: &str) -> LogEvent {
    let mut rest = line.trim();

    let (timestamp, remainder) = strip_timestamp(rest);
    rest = remainder;

    let (level, remainder) = strip_level(rest);
    rest = remainder;

    LogEvent {
        timestamp,
        level,
        message: rest.trim().to_string(),
        http: None,
    }
}

fn strip_timestamp(s: &str) -> (Option<DateTime<Utc>>, &str) {
    if s.len() >= 19 {
        let candidate = &s[..19];
        let normalized = candidate.replace('T', " ");
        if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%d %H:%M:%S") {
            let dt: DateTime<Utc> = Utc.from_utc_datetime(&naive);
            let mut tail = &s[19..];
            if tail.starts_with('Z') {
                tail = &tail[1..];
            }
            return (Some(dt), tail);
        }
    }
    (None, s)
}

fn strip_level(s: &str) -> (Option<Level>, &str) {
    let trimmed = s.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    if end == 0 {
        return (None, s);
    }
    let token = &trimmed[..end];
    if let Some(level) = Level::parse(token) {
        let looks_like_level = token.chars().all(|c| c.is_ascii_uppercase())
            || token.eq_ignore_ascii_case("info")
            || token.eq_ignore_ascii_case("warn");
        if looks_like_level {
            return (Some(level), &trimmed[end..]);
        }
    }
    (None, s)
}