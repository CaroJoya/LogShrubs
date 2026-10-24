// src/parser/nginx.rs
//
// nginx access logs (default combined format, optionally followed by
// `$request_time` in seconds) and nginx error logs:
//   2026/09/15 14:21:04 [error] 1234#0: *5 connect() failed ...

use super::apache;
use crate::model::{Level, LogEvent};
use chrono::{NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use std::sync::OnceLock;

fn error_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) \[(\w+)\] (?:\d+#\d+: )?(?:\*\d+ )?(.*)$",
        )
        .unwrap()
    })
}

/// Does this line look like an nginx error-log line?
pub fn looks_like_error(line: &str) -> bool {
    error_re().is_match(line.trim())
}

fn map_level(s: &str) -> Option<Level> {
    match s.to_ascii_lowercase().as_str() {
        "emerg" | "alert" | "crit" => Some(Level::Fatal),
        "error" => Some(Level::Error),
        "warn" => Some(Level::Warn),
        "notice" | "info" => Some(Level::Info),
        "debug" => Some(Level::Debug),
        _ => None,
    }
}

fn parse_error_line(line: &str) -> Option<LogEvent> {
    let caps = error_re().captures(line.trim())?;
    let naive = NaiveDateTime::parse_from_str(caps.get(1)?.as_str(), "%Y/%m/%d %H:%M:%S").ok()?;
    Some(LogEvent {
        timestamp: Some(Utc.from_utc_datetime(&naive)),
        level: map_level(caps.get(2)?.as_str()),
        message: caps.get(3)?.as_str().trim().to_string(),
        http: None,
    })
}

pub fn parse(line: &str) -> Option<LogEvent> {
    apache::parse_access(line, true).or_else(|| parse_error_line(line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_line_with_request_time() {
        let line = r#"10.0.0.1 - - [15/Sep/2026:14:21:04 +0000] "GET /api/orders HTTP/1.1" 200 512 "-" "agent" 0.821"#;
        let e = parse(line).unwrap();
        assert_eq!(e.http.unwrap().duration_ms, Some(821));
    }

    #[test]
    fn access_line_without_request_time() {
        let line =
            r#"10.0.0.1 - - [15/Sep/2026:14:21:04 +0000] "GET /a HTTP/1.1" 200 512 "-" "agent""#;
        assert_eq!(parse(line).unwrap().http.unwrap().duration_ms, None);
    }

    #[test]
    fn error_log_line_has_real_level_and_time() {
        let line =
            "2026/09/15 14:21:04 [error] 1234#0: *5 connect() failed (111: Connection refused)";
        let e = parse(line).unwrap();
        assert_eq!(e.level, Some(Level::Error));
        assert_eq!(
            e.timestamp.unwrap().to_rfc3339(),
            "2026-09-15T14:21:04+00:00"
        );
        assert_eq!(e.message, "connect() failed (111: Connection refused)");
    }

    #[test]
    fn level_mapping() {
        assert_eq!(map_level("crit"), Some(Level::Fatal));
        assert_eq!(map_level("notice"), Some(Level::Info));
        assert_eq!(map_level("warn"), Some(Level::Warn));
        assert_eq!(map_level("nonsense"), None);
    }
}