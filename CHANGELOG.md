# Changelog

All notable changes to LogShrubs are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/) and the project uses [Semantic Versioning](https://semver.org/).

## [1.0.0] — Scale & Ecosystem

First stable release.

### Added
- **`watch` subcommand** (`logshrubs watch app.log`): follows a file using the `notify` crate and prints a live summary — level counts, recent errors and current spike status. Handles partial lines, truncation and log rotation (including a file that disappears and comes back). `--interval <ms>` sets the fallback refresh tick, `--once` prints a single summary and exits, `--json` emits one JSON object per update.
- **Parallel processing** with `rayon`: when more than one input is given, files are analyzed on worker threads and merged in input order, so output is identical to a sequential run. A single input stays single-threaded. New `--jobs/-j <N>` flag (`0` = automatic, `1` = force sequential).
- **GitHub Actions**: `ci.yml` (tests on Linux/macOS/Windows, plus `rustfmt` and `clippy`) and `release.yml` (cross-builds Linux, macOS (Apple Silicon + Intel) and Windows binaries on `v*` tags and attaches them to the GitHub Release).
- Prebuilt binaries documented in the README install section.

### Changed
- Every accumulator can now be merged (`merge`) and cloned, which is what powers both parallel runs and `watch`.
- Version bumped to `1.0.0`.

## [0.4.0] — Format Support

### Added
- **`--format auto|text|json|apache|nginx`** (default `auto`).
- **Per-file auto-detection** from the first 20 lines (a format must match more than half of the non-empty sample lines) — never per line.
- **JSON logs** (`parser/json.rs`): `timestamp`/`time`/`ts`, `level`/`severity`/`lvl`, `message`/`msg` aliases; numeric (pino/bunyan) levels; RFC 3339, `YYYY-MM-DD HH:MM:SS` and epoch (seconds or milliseconds) timestamps; optional `method`/`path`/`status`/`duration_ms` fields feed HTTP statistics. A malformed JSON line never crashes — it becomes an unleveled event.
- **Apache** common/combined access logs (`parser/apache.rs`) with real timestamps (`[15/Sep/2026:14:21:04 +0000]`).
- **nginx** access logs (`parser/nginx.rs`) including a trailing `$request_time` (feeds latency stats), and nginx **error logs** with real levels (`emerg`/`alert`/`crit` → FATAL, `error`, `warn`, `notice`/`info` → INFO, `debug`).
- `--verbose` shows the detected format per input; JSON output gains a `formats` array.

### Changed
- Lines that don't fit the chosen structured format fall back to the text parser, so mixed files never lose lines.
- The plain-text parser is unchanged and remains the default fallback.

## [0.3.0] — Latency Statistics

### Added
- `PerformanceAccumulator`: average, median, p95, p99, min and max response time from the durations that were already being parsed.
- Per-endpoint latency, shown as `SLOW ENDPOINTS` (top N by average latency).
- New terminal blocks `RESPONSE TIME` and `SLOW ENDPOINTS` in `analyze` and `http`; shown only when durations exist.
- JSON `http` object gains `avg_ms`, `min_ms`, `median_ms`, `p95_ms`, `p99_ms`, `max_ms` and `slowest_endpoints`.
- Percentiles use a **bounded reservoir** (100,000 samples): exact up to that many requests, a reproducible uniform sample beyond it. Average, min and max are always exact, and memory stays flat regardless of file size.

## [0.2.0] — Input Flexibility

### Added
- **Multiple files** in one run (`logshrubs analyze a.log b.log c.log`), merged into a single report.
- **Directories**: searched recursively for `.log` / `.txt` files; `--no-recursive` stays at the top level. An empty directory is a clear error.
- **stdin**: `-` as a path, or no path at all (`cat app.log | logshrubs`, `docker logs web | logshrubs`). Bare `logshrubs` with no subcommand runs `analyze`.
- Human-readable `Size` line (B / KB / MB / GB) for file-based input.
- `--quiet` now suppresses empty sections (zero-count levels, missing TIME, "no HTTP requests", "no spikes").
- `--verbose` now widens columns (40 → 80), shows twice as many rows, and lists inputs, detected format and parsed-event count.

### Fixed
- Mid-file read errors are no longer swallowed: they are reported with the source path. Invalid UTF-8 bytes are replaced instead of aborting the read.
- `version` and `version --json` now print `logshrubs` instead of the stale `loglens`.
- `--quiet` / `--verbose` were declared but never read — both are now wired up.

## [0.1.0]

- Initial release: `analyze`, `stats`, `errors`, `http`, `version`; level counts, error grouping, spike detection, HTTP status and endpoint analysis, JSON output.