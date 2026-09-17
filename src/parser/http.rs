// src/parser/http.rs

use crate::model::HttpInfo;
use regex::Regex;
use std::sync::OnceLock;

/// Try to extract HTTP request info from a line's message.
/// Returns None if the line doesn't look like an HTTP request log.
pub fn parse(message: &str) -> Option<HttpInfo> {
    // Pattern A: "GET /api/users 200 42ms"
    if let Some(caps) = simple_re().captures(message) {
        let method = caps.get(1)?.as_str().to_string();
        let path = caps.get(2)?.as_str().to_string();
        let status: u16 = caps.get(3)?.as_str().parse().ok()?;
        let duration_ms = caps
            .get(4)
            .and_then(|m| m.as_str().parse::<u64>().ok());
        return Some(HttpInfo { method, path, status, duration_ms });
    }

    // Pattern B: '"GET /api/products HTTP/1.1" 200'
    if let Some(caps) = combined_re().captures(message) {
        let method = caps.get(1)?.as_str().to_string();
        let path = caps.get(2)?.as_str().to_string();
        let status: u16 = caps.get(3)?.as_str().parse().ok()?;
        return Some(HttpInfo { method, path, status, duration_ms: None });
    }

    None
}

/// "GET /api/users 200 42ms" (duration optional)
fn simple_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\s+(\S+)\s+(\d{3})(?:\s+(\d+)ms)?")
            .unwrap()
    })
}

/// '"GET /path HTTP/1.1" 200'
fn combined_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#""(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\s+(\S+)\s+HTTP/[\d.]+"\s+(\d{3})"#)
            .unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_with_duration() {
        let info = parse("GET /api/users 200 42ms").unwrap();
        assert_eq!(info.method, "GET");
        assert_eq!(info.path, "/api/users");
        assert_eq!(info.status, 200);
        assert_eq!(info.duration_ms, Some(42));
    }

    #[test]
    fn parses_simple_without_duration() {
        let info = parse("DELETE /api/orders/5 204").unwrap();
        assert_eq!(info.method, "DELETE");
        assert_eq!(info.status, 204);
        assert_eq!(info.duration_ms, None);
    }

    #[test]
    fn parses_combined_apache_style() {
        let line = r#"192.168.1.1 - - [15/Sep/2026:14:21:04 +0000] "GET /api/products HTTP/1.1" 200 124"#;
        let info = parse(line).unwrap();
        assert_eq!(info.method, "GET");
        assert_eq!(info.path, "/api/products");
        assert_eq!(info.status, 200);
    }

    #[test]
    fn rejects_non_http() {
        assert!(parse("User authenticated id=4812").is_none());
        assert!(parse("Database connection timeout").is_none());
        assert!(parse("").is_none());
    }
}