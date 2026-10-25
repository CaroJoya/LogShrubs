// src/parser/apache.rs
//
// Apache / generic "common" and "combined" access logs:
//   127.0.0.1 - - [15/Sep/2026:14:21:04 +0000] "GET /a HTTP/1.1" 200 124 "ref" "agent"
// nginx's default access format is identical, so nginx.rs reuses this
// parser and additionally understands a trailing request time.

use crate::model::{HttpInfo, LogEvent};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;

fn access_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"^(\S+) \S+ \S+ \[([^\]]+)\] "([A-Z]+) (\S+)(?: [^"]*)?" (\d{3}) (\S+)(?: "[^"]*" "[^"]*")?(.*)$"#,
        )
        .unwrap()
    })
}

/// Does this line have the shape of a common/combined access log line?
pub fn looks_like_access(line: &str) -> bool {
    access_re().is_match(line.trim())
}

/// Parse an access line. `with_request_time` enables reading a trailing
/// nginx `$request_time` (seconds, e.g. `0.123` or `rt=0.123`).
pub(crate) fn parse_access(line: &str, with_request_time: bool) -> Option<LogEvent> {
    let caps = access_re().captures(line.trim())?;
    let timestamp = parse_clf_timestamp(caps.get(2)?.as_str());
    let method = caps.get(3)?.as_str().to_string();
    let path = caps.get(4)?.as_str().to_string();
    let status: u16 = caps.get(5)?.as_str().parse().ok()?;
    let duration_ms = if with_request_time {
        request_time_ms(caps.get(7).map_or("", |m| m.as_str()))
    } else {
        None
    };

    Some(LogEvent {
        timestamp,
        level: None,
        message: format!("{} {} {}", method, path, status),
        http: Some(HttpInfo {
            method,
            path,
            status,
            duration_ms,
        }),
    })
}

/// `[15/Sep/2026:14:21:04 +0000]` contents → UTC timestamp.
pub(crate) fn parse_clf_timestamp(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_str(s, "%d/%b/%Y:%H:%M:%S %z")
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Last whitespace-separated token of `tail` as seconds → milliseconds.
pub(crate) fn request_time_ms(tail: &str) -> Option<u64> {
    let tok = tail.split_whitespace().last()?;
    let tok = tok
        .strip_prefix("rt=")
        .or_else(|| tok.strip_prefix("request_time="))
        .unwrap_or(tok);
    // Must look like a decimal number: "0.123" (plain integers are ambiguous).
    if !tok.contains('.') {
        return None;
    }
    let secs: f64 = tok.parse().ok()?;
    if secs < 0.0 {
        return None;
    }
    Some((secs * 1000.0).round() as u64)
}

/// True when the line has a trailing request time (used by auto-detect).
pub(crate) fn has_request_time(line: &str) -> bool {
    access_re()
        .captures(line.trim())
        .and_then(|c| c.get(7).map(|m| request_time_ms(m.as_str()).is_some()))
        .unwrap_or(false)
}

pub fn parse(line: &str) -> Option<LogEvent> {
    parse_access(line, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMBINED: &str = r#"192.168.1.1 - - [15/Sep/2026:14:21:04 +0000] "GET /api/products HTTP/1.1" 200 124 "-" "curl/8.0""#;

    #[test]
    fn parses_combined_line_fully() {
        let e = parse(COMBINED).unwrap();
        assert_eq!(
            e.timestamp.unwrap().to_rfc3339(),
            "2026-09-15T14:21:04+00:00"
        );
        assert_eq!(e.level, None);
        assert_eq!(e.message, "GET /api/products 200");
        let h = e.http.unwrap();
        assert_eq!(h.method, "GET");
        assert_eq!(h.status, 200);
    }

    #[test]
    fn parses_common_line_and_offsets() {
        let e = parse(r#"10.0.0.1 - bob [15/Sep/2026:16:21:04 +0200] "POST /x HTTP/1.0" 500 -"#)
            .unwrap();
        assert_eq!(
            e.timestamp.unwrap().to_rfc3339(),
            "2026-09-15T14:21:04+00:00"
        );
        assert_eq!(e.http.unwrap().status, 500);
    }

    #[test]
    fn rejects_non_access_lines() {
        assert!(parse("2026-09-15 14:21:04 INFO hello").is_none());
        assert!(!looks_like_access("{\"a\":1}"));
    }

    #[test]
    fn apache_ignores_request_time() {
        let line = format!("{} 0.250", COMBINED);
        assert_eq!(parse(&line).unwrap().http.unwrap().duration_ms, None);
    }
}
