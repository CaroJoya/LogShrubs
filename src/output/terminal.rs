// src/output/terminal.rs

use crate::model::Stats;

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