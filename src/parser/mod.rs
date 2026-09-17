// src/parser/mod.rs

pub mod text;

use crate::model::LogEvent;

pub fn parse_line(line: &str) -> LogEvent {
    text::parse(line)
}