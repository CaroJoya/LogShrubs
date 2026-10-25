// src/analyzer/engine.rs

use crate::analyzer::{ErrorAccumulator, HttpAccumulator, StatsAccumulator};
use crate::error::{LogShrubsError, Result};
use crate::input::{total_size, LogReader, Source};
use crate::model::{AnalysisResult, TimeSummary};
use crate::parser::{self, Format, FormatChoice};
use rayon::prelude::*;

/// Knobs for one analysis run.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// How many distinct errors / endpoints to keep.
    pub top_n: usize,
    pub format: FormatChoice,
    /// Worker threads for multi-source runs: 0 = automatic, 1 = sequential.
    pub jobs: usize,
}

/// Everything learned from one input source. Partials are merged in
/// source order, so the result does not depend on thread scheduling.
struct Partial {
    label: String,
    format: Format,
    lines: u64,
    stats: StatsAccumulator,
    errors: ErrorAccumulator,
    http: HttpAccumulator,
}

impl Partial {
    fn new(label: String) -> Self {
        Self {
            label,
            format: Format::Text,
            lines: 0,
            stats: StatsAccumulator::new(),
            errors: ErrorAccumulator::new(),
            http: HttpAccumulator::new(),
        }
    }

    fn observe_line(&mut self, line: &str, format: Format) {
        self.lines += 1;
        let event = parser::parse_line(line, format);
        self.stats.observe(&event);
        self.errors.observe(&event);
        self.http.observe(&event);
    }

    fn merge(&mut self, other: Partial) {
        self.lines += other.lines;
        self.stats.merge(other.stats);
        self.errors.merge(other.errors);
        self.http.merge(other.http);
    }
}

/// Stream one source line by line. With `--format auto`, only the first
/// few lines are buffered to pick the format; the rest streams through.
fn process_source(source: &Source, choice: FormatChoice) -> Result<Partial> {
    let mut reader = LogReader::open(source)?;
    let mut partial = Partial::new(source.label());

    let mut head: Vec<String> = Vec::new();
    let format = match choice {
        FormatChoice::Fixed(f) => f,
        FormatChoice::Auto => {
            while head.len() < parser::DETECT_SAMPLE_LINES {
                match reader.next() {
                    Some(line) => head.push(line?),
                    None => break,
                }
            }
            parser::detect(&head)
        }
    };
    partial.format = format;

    for line in &head {
        partial.observe_line(line, format);
    }
    for line in reader {
        partial.observe_line(&line?, format);
    }
    Ok(partial)
}

/// Run the full analysis over one or more sources in a single streaming
/// pass each, and merge them into one report.
///
/// With more than one source (and `jobs != 1`), sources are processed in
/// parallel with rayon; a single source always stays on the calling thread.
pub fn analyze_sources(sources: &[Source], opts: &Options) -> Result<AnalysisResult> {
    let partials: Vec<Partial> = if sources.len() > 1 && opts.jobs != 1 {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(opts.jobs)
            .build()
            .map_err(|e| LogShrubsError::Internal(e.to_string()))?;
        pool.install(|| {
            sources
                .par_iter()
                .map(|s| process_source(s, opts.format))
                .collect::<Result<Vec<_>>>()
        })?
    } else {
        sources
            .iter()
            .map(|s| process_source(s, opts.format))
            .collect::<Result<Vec<_>>>()?
    };

    let labels: Vec<String> = partials.iter().map(|p| p.label.clone()).collect();
    let formats: Vec<String> = partials
        .iter()
        .map(|p| p.format.name().to_string())
        .collect();

    let mut iter = partials.into_iter();
    let mut total = iter
        .next()
        .ok_or_else(|| LogShrubsError::Internal("no input sources".to_string()))?;
    for p in iter {
        total.merge(p);
    }

    let total_lines = total.lines;
    let stats = total.stats.finish();
    let errors = total.errors.finish(opts.top_n);
    let http = total.http.finish(opts.top_n);

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
        source: labels.join(", "),
        sources: labels,
        formats,
        size_bytes: total_size(sources),
        total_lines,
        parsed_events: total_lines,
        stats,
        errors,
        time,
        http,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &std::path::Path, name: &str, body: &str) -> Source {
        let p = dir.join(name);
        fs::write(&p, body).unwrap();
        Source::File(p)
    }

    fn opts(jobs: usize) -> Options {
        Options {
            top_n: 10,
            format: FormatChoice::Auto,
            jobs,
        }
    }

    #[test]
    fn parallel_output_equals_sequential() {
        let dir = tempfile::tempdir().unwrap();
        let mut sources = Vec::new();
        for f in 0..6 {
            let mut body = String::new();
            for i in 0..200 {
                body.push_str(&format!(
                    "2026-09-15 14:{:02}:{:02} ERROR Failure {} in worker\n",
                    (f * 7 + i / 60) % 60,
                    i % 60,
                    i
                ));
                body.push_str(&format!(
                    "2026-09-15 14:{:02}:{:02} GET /api/r{} 200 {}ms\n",
                    (f * 5 + i / 60) % 60,
                    i % 60,
                    i % 7,
                    (i * 13 + f) % 900
                ));
            }
            sources.push(write(dir.path(), &format!("f{}.log", f), &body));
        }
        let seq = analyze_sources(&sources, &opts(1)).unwrap();
        let par = analyze_sources(&sources, &opts(4)).unwrap();
        assert_eq!(
            serde_json::to_string(&seq).unwrap(),
            serde_json::to_string(&par).unwrap()
        );
        assert_eq!(seq.total_lines, 6 * 400);
    }

    #[test]
    fn multi_file_merge_sums_counts() {
        let dir = tempfile::tempdir().unwrap();
        let a = write(dir.path(), "a.log", "2026-09-15 14:00:00 ERROR boom\n");
        let b = write(
            dir.path(),
            "b.log",
            "2026-09-15 15:00:00 ERROR boom\n2026-09-15 15:00:01 INFO ok\n",
        );
        let r = analyze_sources(&[a, b], &opts(0)).unwrap();
        assert_eq!(r.total_lines, 3);
        assert_eq!(r.stats.error, 2);
        assert_eq!(r.errors.top[0].count, 2);
        assert_eq!(r.sources.len(), 2);
        assert_eq!(r.time.duration_secs, Some(3601));
    }

    #[test]
    fn format_is_detected_per_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = write(dir.path(), "a.log", "{\"level\":\"error\",\"msg\":\"x\"}\n");
        let b = write(dir.path(), "b.log", "2026-09-15 14:00:00 ERROR y\n");
        let r = analyze_sources(&[a, b], &opts(1)).unwrap();
        assert_eq!(r.formats, vec!["json", "text"]);
        assert_eq!(r.stats.error, 2);
    }
}
