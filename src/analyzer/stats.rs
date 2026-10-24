// src/analyzer/stats.rs

use crate::model::{Level, LogEvent, Stats};
use chrono::{DateTime, Utc};

/// Folds LogEvents into a Stats struct.
///
/// Usage:
///     let mut acc = StatsAccumulator::new();
///     for event in events { acc.observe(&event); }
///     let stats = acc.finish();
#[derive(Debug, Clone, Default)]
pub struct StatsAccumulator {
    info: u64,
    debug: u64,
    warn: u64,
    error: u64,
    fatal: u64,
    trace: u64,
    unleveled: u64,
    first_timestamp: Option<DateTime<Utc>>,
    last_timestamp: Option<DateTime<Utc>>,
}

impl StatsAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold one event into the running stats.
    pub fn observe(&mut self, event: &LogEvent) {
        // Count by level
        match event.level {
            Some(Level::Info) => self.info += 1,
            Some(Level::Debug) => self.debug += 1,
            Some(Level::Warn) => self.warn += 1,
            Some(Level::Error) => self.error += 1,
            Some(Level::Fatal) => self.fatal += 1,
            Some(Level::Trace) => self.trace += 1,
            None => self.unleveled += 1,
        }

        // Track first/last timestamp
        if let Some(ts) = event.timestamp {
            match self.first_timestamp {
                None => self.first_timestamp = Some(ts),
                Some(existing) if ts < existing => self.first_timestamp = Some(ts),
                _ => {}
            }
            match self.last_timestamp {
                None => self.last_timestamp = Some(ts),
                Some(existing) if ts > existing => self.last_timestamp = Some(ts),
                _ => {}
            }
        }
    }

    /// Fold another accumulator (e.g. from a different file) into this one.
    pub fn merge(&mut self, other: StatsAccumulator) {
        self.info += other.info;
        self.debug += other.debug;
        self.warn += other.warn;
        self.error += other.error;
        self.fatal += other.fatal;
        self.trace += other.trace;
        self.unleveled += other.unleveled;
        self.first_timestamp = match (self.first_timestamp, other.first_timestamp) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        self.last_timestamp = match (self.last_timestamp, other.last_timestamp) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
    }

    /// Consume the accumulator and produce the final Stats value.
    pub fn finish(self) -> Stats {
        Stats {
            info: self.info,
            debug: self.debug,
            warn: self.warn,
            error: self.error,
            fatal: self.fatal,
            trace: self.trace,
            unleveled: self.unleveled,
            first_timestamp: self.first_timestamp,
            last_timestamp: self.last_timestamp,
        }
    }
}