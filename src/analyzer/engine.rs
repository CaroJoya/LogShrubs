// src/analyzer/engine.rs

use crate::analyzer::{ErrorAccumulator, HttpAccumulator, StatsAccumulator};
use crate::input::LogReader;
use crate::model::{AnalysisResult, TimeSummary};
use crate::parser;

/// Run the full analysis engine over a log file.
///
/// Opens the file, parses every line into a LogEvent, feeds it to
/// three accumulators (stats, errors, http) in a single pass, and
/// returns a combined AnalysisResult.
pub fn analyze_file(path: &std::path::Path, top_n: usize) -> std::io::Result<AnalysisResult> {
    let source = path.display().to_string();
    let reader = LogReader::open(path)?;

    let mut stats_acc = StatsAccumulator::new();
    let mut errors_acc = ErrorAccumulator::new();
    let mut http_acc = HttpAccumulator::new();

    let mut total_lines: u64 = 0;
    let mut parsed_events: u64 = 0;

    for line in reader {
        total_lines += 1;
        let event = parser::parse_line(&line);

        stats_acc.observe(&event);
        errors_acc.observe(&event);
        http_acc.observe(&event);

        parsed_events += 1;
    }

    let stats = stats_acc.finish();
    let errors = errors_acc.finish(top_n);
    let http = http_acc.finish(top_n);

    // Derive TimeSummary from the stats we already have.
    let time = TimeSummary {
        first: stats.first_timestamp,
        last: stats.last_timestamp,
        duration_secs: match (stats.first_timestamp, stats.last_timestamp) {
            (Some(f), Some(l)) => Some((l - f).num_seconds().max(0)),
            _ => None,
        },
    };

    Ok(AnalysisResult {
        source,
        total_lines,
        parsed_events,
        stats,
        errors,
        time,
        http,
    })
}
