// src/output/terminal.rs

use crate::model::{AnalysisResult, ErrorSpike, HttpSummary};

/// How the terminal report should be rendered.
#[derive(Debug, Clone, Copy)]
pub struct RenderOpts {
    /// The `--top` value the user asked for.
    pub top_n: usize,
    /// `--quiet`: leave out empty sections and "nothing found" notes.
    pub quiet: bool,
    /// `--verbose`: wider columns, twice the rows, per-input details.
    pub verbose: bool,
}

impl RenderOpts {
    /// Rows to keep/show. Verbose shows twice as many.
    pub fn rows(&self) -> usize {
        if self.verbose {
            self.top_n.saturating_mul(2)
        } else {
            self.top_n
        }
    }

    /// Column width for messages and paths.
    pub fn width(&self) -> usize {
        if self.verbose {
            80
        } else {
            40
        }
    }
}

const RULE: &str = "─────────────────────────────────";

/// Format a byte count as B / KB / MB / GB.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.1} GB", b / (KB * KB * KB))
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

/// Format seconds as "1h 39m 3s", "5m 12s", or "42s".
fn format_duration(secs: i64) -> String {
    if secs < 0 {
        return "0s".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// `File` / `Files` / `Size` / `Format` lines shared by every report.
fn print_source(result: &AnalysisResult, opts: &RenderOpts) {
    if result.sources.len() == 1 {
        println!("File        {}", result.sources[0]);
    } else {
        println!("Files       {}", result.sources.len());
        if opts.verbose {
            for s in &result.sources {
                println!("            {}", s);
            }
        }
    }
    if let Some(size) = result.size_bytes {
        println!("Size        {}", format_size(size));
    }
    if opts.verbose {
        let mut formats: Vec<&str> = Vec::new();
        for f in &result.formats {
            if !formats.contains(&f.as_str()) {
                formats.push(f);
            }
        }
        println!("Format      {}", formats.join(", "));
    }
}

fn print_level_row(label: &str, count: u64, opts: &RenderOpts) {
    if opts.quiet && count == 0 {
        return;
    }
    println!("{:<11} {:>6}", label, count);
}

fn print_ranked(rows: &[(String, u64)], opts: &RenderOpts) {
    let w = opts.width();
    for (i, (name, count)) in rows.iter().enumerate() {
        println!(
            "{:>2}. {:<w$} {:>6}",
            i + 1,
            truncate(name, w),
            count,
            w = w
        );
    }
}

fn spike_line(spike: &ErrorSpike) -> String {
    format!(
        "{} → {}   {:>5} errors   ⚠ {:.1}× normal",
        spike.bucket_start.format("%H:%M"),
        spike.bucket_end.format("%H:%M"),
        spike.count,
        spike.severity,
    )
}

/// RESPONSE TIME block. Returns false (prints nothing) without duration data.
fn print_response_time(http: &HttpSummary) -> bool {
    let Some(avg) = http.avg_ms else {
        return false;
    };
    let ms = |v: Option<u64>| v.map_or("-".to_string(), |v| format!("{} ms", v));
    println!("RESPONSE TIME");
    println!("{}", RULE);
    println!("Average     {:.1} ms", avg);
    println!("Median      {}", ms(http.median_ms));
    println!("P95         {}", ms(http.p95_ms));
    println!("P99         {}", ms(http.p99_ms));
    println!("Min         {}", ms(http.min_ms));
    println!("Max         {}", ms(http.max_ms));
    true
}

/// SLOW ENDPOINTS block. Prints nothing without duration data.
fn print_slow_endpoints(http: &HttpSummary, opts: &RenderOpts) {
    if http.slowest_endpoints.is_empty() {
        return;
    }
    let w = opts.width();
    println!("SLOW ENDPOINTS");
    println!("{}", RULE);
    for (i, e) in http.slowest_endpoints.iter().take(opts.rows()).enumerate() {
        println!(
            "{:>2}. {:<w$} {:>9} ms avg {:>6} req",
            i + 1,
            truncate(&e.path, w),
            format!("{:.1}", e.avg_ms),
            e.count,
            w = w
        );
    }
}

fn http_status_rows(http: &HttpSummary, opts: &RenderOpts, always_other: bool) {
    println!("2xx         {:>6}", http.s2xx);
    println!("3xx         {:>6}", http.s3xx);
    println!("4xx         {:>6}", http.s4xx);
    println!("5xx         {:>6}", http.s5xx);
    if always_other || opts.verbose || http.other > 0 {
        println!("Other       {:>6}", http.other);
    }
}

fn endpoint_rows(http: &HttpSummary, opts: &RenderOpts) -> Vec<(String, u64)> {
    http.top_endpoints
        .iter()
        .take(opts.rows())
        .map(|e| (e.path.clone(), e.count))
        .collect()
}

/// Print a Stats summary to stdout in human-readable form.
pub fn print_stats(result: &AnalysisResult, opts: &RenderOpts) {
    let stats = &result.stats;
    println!("STATS");
    println!("─────────────────────────");
    print_source(result, opts);
    println!("Lines       {}", result.total_lines);
    println!();
    print_level_row("INFO", stats.info, opts);
    print_level_row("DEBUG", stats.debug, opts);
    print_level_row("WARN", stats.warn, opts);
    print_level_row("ERROR", stats.error, opts);
    print_level_row("FATAL", stats.fatal, opts);
    print_level_row("TRACE", stats.trace, opts);
    print_level_row("UNLEVELED", stats.unleveled, opts);
    println!();

    match (stats.first_timestamp, stats.last_timestamp) {
        (Some(first), Some(last)) => {
            println!("FIRST       {}", first.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("LAST        {}", last.format("%Y-%m-%d %H:%M:%S UTC"));
            let dur = last - first;
            println!("DURATION    {}s", dur.num_seconds().max(0));
        }
        _ if opts.quiet => {}
        _ => {
            println!("FIRST       (no timestamps found)");
            println!("LAST        (no timestamps found)");
        }
    }
}

/// Print the error report (recurring errors + spikes).
pub fn print_errors(result: &AnalysisResult, opts: &RenderOpts) {
    let summary = &result.errors;
    println!("ERRORS");
    println!("{}", RULE);
    print_source(result, opts);
    println!("Total       {}", summary.total);
    println!();

    if summary.top.is_empty() {
        if !opts.quiet {
            println!("(no errors found)");
        }
        return;
    }

    let rows: Vec<(String, u64)> = summary
        .top
        .iter()
        .take(opts.rows())
        .map(|e| (e.message.clone(), e.count))
        .collect();
    print_ranked(&rows, opts);

    if summary.spikes.is_empty() && opts.quiet {
        return;
    }
    println!();
    println!("ERROR SPIKES");
    println!("{}", RULE);
    if summary.spikes.is_empty() {
        println!("(no significant spikes detected)");
        return;
    }
    for spike in &summary.spikes {
        println!("{}", spike_line(spike));
    }
}

/// Print the HTTP report.
pub fn print_http(result: &AnalysisResult, opts: &RenderOpts) {
    let summary = &result.http;
    println!("HTTP ANALYSIS");
    println!("{}", RULE);
    print_source(result, opts);
    println!("Requests    {}", summary.total_requests);
    println!();

    if summary.total_requests == 0 {
        if !opts.quiet {
            println!("(no HTTP requests detected)");
        }
        return;
    }

    http_status_rows(summary, opts, true);
    println!();

    if print_response_time(summary) {
        println!();
    }

    if !summary.top_endpoints.is_empty() {
        println!("TOP ENDPOINTS");
        println!("{}", RULE);
        print_ranked(&endpoint_rows(summary, opts), opts);
    }

    if !summary.slowest_endpoints.is_empty() {
        println!();
        print_slow_endpoints(summary, opts);
    }
}

/// Print a complete AnalysisResult in human-readable form.
/// This is the "full picture" output for `logshrubs analyze`.
pub fn print_analyze(result: &AnalysisResult, opts: &RenderOpts) {
    println!("ANALYSIS");
    println!("{}", RULE);
    print_source(result, opts);
    println!("Lines       {}", result.total_lines);
    if opts.verbose {
        println!("Parsed      {}", result.parsed_events);
    }
    println!();

    // LEVELS
    println!("LEVELS");
    println!("{}", RULE);
    print_level_row("INFO", result.stats.info, opts);
    print_level_row("WARN", result.stats.warn, opts);
    print_level_row("ERROR", result.stats.error, opts);
    print_level_row("FATAL", result.stats.fatal, opts);
    print_level_row("DEBUG", result.stats.debug, opts);
    print_level_row("TRACE", result.stats.trace, opts);
    print_level_row("UNLEVELED", result.stats.unleveled, opts);
    println!();

    // TIME
    match (result.time.first, result.time.last) {
        (Some(first), Some(last)) => {
            println!("TIME");
            println!("{}", RULE);
            println!("First       {}", first.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("Last        {}", last.format("%Y-%m-%d %H:%M:%S UTC"));
            let secs = result.time.duration_secs.unwrap_or(0);
            println!("Duration    {}", format_duration(secs));
            println!();
        }
        _ if opts.quiet => {}
        _ => {
            println!("TIME");
            println!("{}", RULE);
            println!("First       (no timestamps found)");
            println!("Last        (no timestamps found)");
            println!();
        }
    }

    // TOP ERRORS + SPIKES
    if result.errors.total > 0 {
        println!("TOP ERRORS");
        println!("{}", RULE);
        let rows: Vec<(String, u64)> = result
            .errors
            .top
            .iter()
            .take(opts.rows())
            .map(|e| (e.message.clone(), e.count))
            .collect();
        print_ranked(&rows, opts);
        println!();

        if !result.errors.spikes.is_empty() {
            println!("ERROR SPIKES");
            println!("{}", RULE);
            for spike in &result.errors.spikes {
                println!("{}", spike_line(spike));
            }
            println!();
        }
    }

    // HTTP
    let http = &result.http;
    if http.total_requests > 0 {
        println!("HTTP");
        println!("{}", RULE);
        println!("Requests    {}", http.total_requests);
        http_status_rows(http, opts, false);
        println!();

        if print_response_time(http) {
            println!();
        }

        if !http.top_endpoints.is_empty() {
            println!("TOP ENDPOINTS");
            println!("{}", RULE);
            print_ranked(&endpoint_rows(http, opts), opts);
            println!();
        }

        print_slow_endpoints(http, opts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn verbose_widens_and_doubles() {
        let q = RenderOpts {
            top_n: 5,
            quiet: false,
            verbose: false,
        };
        let v = RenderOpts {
            top_n: 5,
            quiet: false,
            verbose: true,
        };
        assert_eq!((q.rows(), q.width()), (5, 40));
        assert_eq!((v.rows(), v.width()), (10, 80));
    }

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
    }
}