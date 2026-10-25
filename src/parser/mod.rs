// src/parser/mod.rs

pub mod apache;
pub mod http;
pub mod json;
pub mod nginx;
pub mod text;

use crate::model::LogEvent;

/// A concrete log format (what `--format auto` resolves to per file).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
    Apache,
    Nginx,
}

impl Format {
    pub fn name(&self) -> &'static str {
        match self {
            Format::Text => "text",
            Format::Json => "json",
            Format::Apache => "apache",
            Format::Nginx => "nginx",
        }
    }
}

/// What the user asked for with `--format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatChoice {
    Auto,
    Fixed(Format),
}

/// How many leading lines of each input are used to detect its format.
pub const DETECT_SAMPLE_LINES: usize = 20;

/// Parse a single log line in the given format.
///
/// Lines that don't fit the chosen structured format fall back to the
/// plain-text parser, so a mixed file never loses lines.
pub fn parse_line(line: &str, format: Format) -> LogEvent {
    match format {
        Format::Text => parse_text(line),
        Format::Json => {
            let mut event = json::parse(line);
            if event.http.is_none() {
                event.http = http::parse(&event.message);
            }
            event
        }
        Format::Apache => apache::parse(line).unwrap_or_else(|| parse_text(line)),
        Format::Nginx => nginx::parse(line).unwrap_or_else(|| parse_text(line)),
    }
}

/// Text form first (timestamp, level, message), then HTTP extraction on
/// the resulting message, so "2026-09-15 14:21:04 GET /api/products 200
/// 124ms" gets both the timestamp AND the HTTP info.
fn parse_text(line: &str) -> LogEvent {
    let mut event = text::parse(line);
    event.http = http::parse(&event.message);
    event
}

/// Decide the format of one input from its first lines.
///
/// This is per file, not per line: a format wins when more than half of
/// the non-empty sample lines match it.
pub fn detect(sample: &[String]) -> Format {
    let lines: Vec<&str> = sample
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .take(DETECT_SAMPLE_LINES)
        .collect();
    if lines.is_empty() {
        return Format::Text;
    }
    let n = lines.len();
    let majority = |count: usize| count * 2 > n;

    if majority(lines.iter().filter(|l| json::looks_like_json(l)).count()) {
        return Format::Json;
    }
    if majority(lines.iter().filter(|l| nginx::looks_like_error(l)).count()) {
        return Format::Nginx;
    }
    let access = lines
        .iter()
        .filter(|l| apache::looks_like_access(l))
        .count();
    if majority(access) {
        let with_rt = lines.iter().filter(|l| apache::has_request_time(l)).count();
        return if with_rt * 2 > access {
            Format::Nginx
        } else {
            Format::Apache
        };
    }
    Format::Text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    const APACHE: &str =
        r#"1.2.3.4 - - [15/Sep/2026:14:21:04 +0000] "GET /a HTTP/1.1" 200 1 "-" "ua""#;
    const NGINX: &str =
        r#"1.2.3.4 - - [15/Sep/2026:14:21:04 +0000] "GET /a HTTP/1.1" 200 1 "-" "ua" 0.123"#;

    #[test]
    fn detection_matrix() {
        assert_eq!(detect(&v(&[r#"{"level":"info","msg":"x"}"#])), Format::Json);
        assert_eq!(detect(&v(&[APACHE, APACHE])), Format::Apache);
        assert_eq!(detect(&v(&[NGINX, NGINX])), Format::Nginx);
        assert_eq!(
            detect(&v(&["2026/09/15 14:21:04 [error] 1#0: *1 boom"])),
            Format::Nginx
        );
        assert_eq!(
            detect(&v(&["2026-09-15 14:21:04 INFO hello", "plain line"])),
            Format::Text
        );
        assert_eq!(detect(&v(&[])), Format::Text);
        assert_eq!(detect(&v(&["", "  "])), Format::Text);
    }

    #[test]
    fn minority_json_is_still_text() {
        let sample = v(&[
            "2026-09-15 14:21:04 INFO a",
            "2026-09-15 14:21:05 INFO b",
            r#"{"level":"info"}"#,
        ]);
        assert_eq!(detect(&sample), Format::Text);
    }

    #[test]
    fn forced_text_ignores_json_structure() {
        let e = parse_line(
            r#"{"ts":"2026-09-15T14:21:04Z","level":"ERROR","msg":"boom"}"#,
            Format::Text,
        );
        assert_eq!(e.level, None);
        assert_eq!(e.timestamp, None);
    }

    #[test]
    fn structured_format_falls_back_to_text_for_odd_lines() {
        let e = parse_line("2026-09-15 14:21:04 ERROR odd one", Format::Apache);
        assert_eq!(e.level, Some(crate::model::Level::Error));
    }
}
