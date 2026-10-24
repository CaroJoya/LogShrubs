// src/parser/json.rs

use crate::model::{HttpInfo, Level, LogEvent};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::{Map, Value};

const TIMESTAMP_KEYS: &[&str] = &["timestamp", "time", "ts", "@timestamp", "datetime"];
const LEVEL_KEYS: &[&str] = &["level", "severity", "lvl", "loglevel"];
const MESSAGE_KEYS: &[&str] = &["message", "msg", "log"];

/// Cheap shape check used by format auto-detection.
pub fn looks_like_json(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('{') && t.ends_with('}')
}

/// Parse one JSON log line. Malformed JSON never fails: the raw line
/// becomes the message of an unleveled event.
pub fn parse(line: &str) -> LogEvent {
    let trimmed = line.trim();
    match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Object(map)) => from_object(&map, trimmed),
        _ => LogEvent {
            timestamp: None,
            level: None,
            message: trimmed.to_string(),
            http: None,
        },
    }
}

fn first<'a>(map: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| map.get(*k))
}

fn from_object(map: &Map<String, Value>, raw: &str) -> LogEvent {
    let timestamp = first(map, TIMESTAMP_KEYS).and_then(parse_timestamp);
    let level = first(map, LEVEL_KEYS).and_then(parse_level);
    let message = match first(map, MESSAGE_KEYS) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => raw.to_string(),
        Some(other) => other.to_string(),
    };
    LogEvent {
        timestamp,
        level,
        message,
        http: http_info(map),
    }
}

fn parse_level(v: &Value) -> Option<Level> {
    match v {
        Value::String(s) => parse_level_str(s),
        // pino / bunyan numeric levels
        Value::Number(n) => match n.as_u64()? {
            0..=10 => Some(Level::Trace),
            11..=20 => Some(Level::Debug),
            21..=30 => Some(Level::Info),
            31..=40 => Some(Level::Warn),
            41..=50 => Some(Level::Error),
            _ => Some(Level::Fatal),
        },
        _ => None,
    }
}

fn parse_level_str(s: &str) -> Option<Level> {
    if let Some(l) = Level::parse(s.trim()) {
        return Some(l);
    }
    match s.trim().to_ascii_lowercase().as_str() {
        "notice" | "verbose" => Some(Level::Info),
        "emerg" | "emergency" | "alert" | "panic" => Some(Level::Fatal),
        _ => None,
    }
}

fn parse_timestamp(v: &Value) -> Option<DateTime<Utc>> {
    match v {
        Value::String(s) => parse_timestamp_str(s.trim()),
        Value::Number(n) => {
            let f = n.as_f64()?;
            if f > 1e11 {
                DateTime::from_timestamp_millis(f as i64)
            } else {
                DateTime::from_timestamp(f as i64, 0)
            }
        }
        _ => None,
    }
}

fn parse_timestamp_str(s: &str) -> Option<DateTime<Utc>> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S%.f"] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(Utc.from_utc_datetime(&n));
        }
    }
    None
}

/// Optional HTTP fields: needs method + path + status.
fn http_info(map: &Map<String, Value>) -> Option<HttpInfo> {
    let method = first(map, &["method", "http_method"])?
        .as_str()?
        .to_string();
    let path = first(map, &["path", "url", "uri", "endpoint"])?
        .as_str()?
        .to_string();
    let status = first(map, &["status", "status_code"])?.as_u64()?;
    let duration_ms = first(
        map,
        &[
            "duration_ms",
            "latency_ms",
            "response_time_ms",
            "elapsed_ms",
        ],
    )
    .and_then(|v| v.as_f64())
    .filter(|d| *d >= 0.0)
    .map(|d| d.round() as u64);
    Some(HttpInfo {
        method,
        path,
        status: u16::try_from(status).ok()?,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_field_aliases() {
        let e = parse(r#"{"ts":"2026-09-15T14:21:04Z","lvl":"error","msg":"boom"}"#);
        assert_eq!(e.level, Some(Level::Error));
        assert_eq!(e.message, "boom");
        assert_eq!(
            e.timestamp.unwrap().to_rfc3339(),
            "2026-09-15T14:21:04+00:00"
        );

        let e = parse(r#"{"time":"2026-09-15 14:21:04","severity":"WARNING","message":"hmm"}"#);
        assert_eq!(e.level, Some(Level::Warn));
        assert!(e.timestamp.is_some());
    }

    #[test]
    fn numeric_levels_and_epoch_timestamps() {
        let e = parse(r#"{"level":50,"time":1789482064000,"msg":"x"}"#);
        assert_eq!(e.level, Some(Level::Error));
        assert!(e.timestamp.is_some());
        let e = parse(r#"{"level":30,"ts":1789482064,"msg":"x"}"#);
        assert_eq!(e.level, Some(Level::Info));
    }

    #[test]
    fn malformed_json_does_not_crash() {
        let e = parse(r#"{"level":"ERROR","msg":"oops"#);
        assert_eq!(e.level, None);
        assert_eq!(e.timestamp, None);
        assert!(e.message.contains("oops"));
    }

    #[test]
    fn extracts_http_fields() {
        let e = parse(
            r#"{"level":"info","msg":"req","method":"GET","path":"/a","status":200,"duration_ms":42}"#,
        );
        let h = e.http.unwrap();
        assert_eq!(h.path, "/a");
        assert_eq!(h.duration_ms, Some(42));
    }
}