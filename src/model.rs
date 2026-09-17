// src/model.rs

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Log severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl Level {
    pub fn parse(s: &str) -> Option<Level> {
        match s.to_ascii_uppercase().as_str() {
            "TRACE" => Some(Level::Trace),
            "DEBUG" => Some(Level::Debug),
            "INFO" | "INFORMATION" => Some(Level::Info),
            "WARN" | "WARNING" => Some(Level::Warn),
            "ERROR" | "ERR" => Some(Level::Error),
            "FATAL" | "CRITICAL" | "CRIT" => Some(Level::Fatal),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
            Level::Fatal => "FATAL",
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Level::Error | Level::Fatal)
    }
}

/// A single parsed log line.
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub timestamp: Option<DateTime<Utc>>,
    pub level: Option<Level>,
    pub message: String,
    pub http: Option<HttpInfo>,
}

/// HTTP request info extracted from a line, if present.
#[derive(Debug, Clone, Serialize)]
pub struct HttpInfo {
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: Option<u64>,
}

/// Result of running the analysis engine over a log input.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisResult {
    pub source: String,
    pub total_lines: u64,
    pub parsed_events: u64,
    pub stats: Stats,
    pub errors: ErrorSummary,
    pub time: TimeSummary,
    pub http: HttpSummary,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub info: u64,
    pub debug: u64,
    pub warn: u64,
    pub error: u64,
    pub fatal: u64,
    pub trace: u64,
    pub unleveled: u64,
    pub first_timestamp: Option<DateTime<Utc>>,
    pub last_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ErrorSummary {
    pub total: u64,
    /// Normalized message → count, sorted desc at output time.
    pub top: Vec<ErrorCount>,
    /// 5-minute buckets with elevated error counts.
    pub spikes: Vec<ErrorSpike>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorCount {
    pub message: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorSpike {
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub count: u64,
    /// Multiplier vs the mean of all buckets (e.g. 35.0 = 35× normal).
    pub severity: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TimeSummary {
    pub first: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct HttpSummary {
    pub total_requests: u64,
    pub s2xx: u64,
    pub s3xx: u64,
    pub s4xx: u64,
    pub s5xx: u64,
    pub other: u64,
    pub top_endpoints: Vec<EndpointCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointCount {
    pub path: String,
    pub count: u64,
}
