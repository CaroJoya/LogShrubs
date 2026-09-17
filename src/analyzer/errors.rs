// src/analyzer/errors.rs

use crate::model::{ErrorCount, ErrorSummary, Level, LogEvent};
use std::collections::HashMap;

/// Folds ERROR and FATAL events into a summary of recurring messages.
///
/// Messages are normalized before counting so that variable values
/// don't fragment otherwise-identical errors. Example:
///
///     "User 18273 failed authentication"
///     "User 18291 failed authentication"
///     →  "User <NUM> failed authentication"  (count: 2)
#[derive(Debug, Default)]
pub struct ErrorAccumulator {
    counts: HashMap<String, u64>,
    total: u64,
    spikes: crate::analyzer::spikes::SpikeAccumulator,
}

impl ErrorAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe one event. Only ERROR and FATAL are recorded.
    pub fn observe(&mut self, event: &LogEvent) {
        let is_error = matches!(event.level, Some(Level::Error) | Some(Level::Fatal));
        if !is_error {
            return;
        }

        let normalized = normalize(&event.message);
        *self.counts.entry(normalized).or_insert(0) += 1;
        self.total += 1;

        if let Some(ts) = event.timestamp {
            self.spikes.observe(ts);
        }
    }

    /// Consume the accumulator and produce the final ErrorSummary.
    /// `top_n` limits how many distinct messages are kept.
    pub fn finish(self, top_n: usize) -> ErrorSummary {
        let mut entries: Vec<(String, u64)> = self.counts.into_iter().collect();

        // Sort by count desc, then by message asc (stable tie-break).
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let top = entries
            .into_iter()
            .take(top_n)
            .map(|(message, count)| ErrorCount { message, count })
            .collect();

        ErrorSummary {
            total: self.total,
            top,
            spikes: self.spikes.finish(),
        }
    }
}

/// Normalize an error message so that repeated-but-variable content
/// doesn't fragment counts.
///
/// Rules (deliberately simple for V1):
///   - Trim whitespace
///   - Collapse consecutive whitespace to a single space
///   - Replace runs of digits with `<NUM>`
fn normalize(msg: &str) -> String {
    let collapsed: String = msg.split_whitespace().collect::<Vec<_>>().join(" ");

    let mut out = String::with_capacity(collapsed.len());
    let mut prev_digit = false;

    for ch in collapsed.chars() {
        if ch.is_ascii_digit() {
            if !prev_digit {
                out.push_str("<NUM>");
            }
            prev_digit = true;
        } else {
            prev_digit = false;
            out.push(ch);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(level: Level, msg: &str) -> LogEvent {
        LogEvent {
            timestamp: None,
            level: Some(level),
            message: msg.to_string(),
            http: None,
        }
    }

    #[test]
    fn counts_repeated_errors() {
        let mut acc = ErrorAccumulator::new();
        acc.observe(&event(Level::Error, "Database connection timeout"));
        acc.observe(&event(Level::Error, "Database connection timeout"));
        acc.observe(&event(Level::Error, "Invalid token"));

        let summary = acc.finish(10);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.top[0].message, "Database connection timeout");
        assert_eq!(summary.top[0].count, 2);
    }

    #[test]
    fn ignores_non_errors() {
        let mut acc = ErrorAccumulator::new();
        acc.observe(&event(Level::Info, "everything is fine"));
        acc.observe(&event(Level::Warn, "just a warning"));
        assert_eq!(acc.finish(10).total, 0);
    }

    #[test]
    fn normalizes_numbers() {
        let mut acc = ErrorAccumulator::new();
        acc.observe(&event(Level::Error, "User 18273 failed authentication"));
        acc.observe(&event(Level::Error, "User 18291 failed authentication"));

        let summary = acc.finish(10);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.top.len(), 1);
        assert_eq!(summary.top[0].message, "User <NUM> failed authentication");
        assert_eq!(summary.top[0].count, 2);
    }

    #[test]
    fn includes_fatal() {
        let mut acc = ErrorAccumulator::new();
        acc.observe(&event(Level::Fatal, "shutdown"));
        assert_eq!(acc.finish(10).total, 1);
    }
}
