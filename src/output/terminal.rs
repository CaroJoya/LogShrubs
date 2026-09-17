// src/output/terminal.rs

use crate::model::{ErrorSpike, ErrorSummary, HttpSummary, Stats};

/// Print a Stats summary to stdout in human-readable form.
pub fn print_stats(source: &str, stats: &Stats) {
    let total = stats.info
        + stats.debug
        + stats.warn
        + stats.error
        + stats.fatal
        + stats.trace
        + stats.unleveled;

    println!("STATS");
    println!("─────────────────────────");
    println!("File        {}", source);
    println!("Lines       {}", total);
    println!();
    println!("INFO        {:>6}", stats.info);
    println!("DEBUG       {:>6}", stats.debug);
    println!("WARN        {:>6}", stats.warn);
    println!("ERROR       {:>6}", stats.error);
    println!("FATAL       {:>6}", stats.fatal);
    println!("TRACE       {:>6}", stats.trace);
    println!("UNLEVELED   {:>6}", stats.unleveled);
    println!();

    match (stats.first_timestamp, stats.last_timestamp) {
        (Some(first), Some(last)) => {
            println!("FIRST       {}", first.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("LAST        {}", last.format("%Y-%m-%d %H:%M:%S UTC"));
            let dur = last - first;
            println!("DURATION    {}s", dur.num_seconds().max(0));
        }
        _ => {
            println!("FIRST       (no timestamps found)");
            println!("LAST        (no timestamps found)");
        }
    }
}

/// Print an ErrorSummary in human-readable form.
pub fn print_errors(source: &str, summary: &ErrorSummary, top_n: usize) {
    println!("ERRORS");
    println!("─────────────────────────────────");
    println!("File        {}", source);
    println!("Total       {}", summary.total);
    println!();

    if summary.top.is_empty() {
        println!("(no errors found)");
        return;
    }

    for (i, entry) in summary.top.iter().take(top_n).enumerate() {
        println!(
            "{:>2}. {:<40} {:>6}",
            i + 1,
            truncate(&entry.message, 40),
            entry.count
        );
    }

    print_spikes(&summary.spikes);
}

/// Print a list of detected error spikes.
pub fn print_spikes(spikes: &[ErrorSpike]) {
    println!();
    println!("ERROR SPIKES");
    println!("─────────────────────────────────");

    if spikes.is_empty() {
        println!("(no significant spikes detected)");
        return;
    }

    for spike in spikes {
        println!(
            "{} → {}   {:>5} errors   ⚠ {:.1}× normal",
            spike.bucket_start.format("%H:%M"),
            spike.bucket_end.format("%H:%M"),
            spike.count,
            spike.severity,
        );
    }
}

/// Truncate a string to `max` chars, appending "…" if needed.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Print an HttpSummary in human-readable form.
pub fn print_http(source: &str, summary: &HttpSummary, top_n: usize) {
    println!("HTTP ANALYSIS");
    println!("─────────────────────────────────");
    println!("File        {}", source);
    println!("Requests    {}", summary.total_requests);
    println!();

    if summary.total_requests == 0 {
        println!("(no HTTP requests detected)");
        return;
    }

    println!("2xx         {:>6}", summary.s2xx);
    println!("3xx         {:>6}", summary.s3xx);
    println!("4xx         {:>6}", summary.s4xx);
    println!("5xx         {:>6}", summary.s5xx);
    println!("Other       {:>6}", summary.other);
    println!();

    if summary.top_endpoints.is_empty() {
        return;
    }

    println!("TOP ENDPOINTS");
    println!("─────────────────────────────────");
    for (i, entry) in summary.top_endpoints.iter().take(top_n).enumerate() {
        println!(
            "{:>2}. {:<40} {:>6}",
            i + 1,
            truncate(&entry.path, 40),
            entry.count
        );
    }
}