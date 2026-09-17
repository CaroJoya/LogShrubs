// src/parser/mod.rs

pub mod http;
pub mod text;

use crate::model::LogEvent;

/// Parse a single log line into a LogEvent.
///
/// We parse the text form first (timestamp, level, message), then
/// attempt HTTP extraction on the resulting message. This way
/// "2026-09-15 14:21:04 GET /api/products 200 124ms" gets both
/// the timestamp AND the HTTP info.
pub fn parse_line(line: &str) -> LogEvent {
    let mut event = text::parse(line);
    event.http = http::parse(&event.message);
    event
}