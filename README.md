LogShrubs 🌿
![Crates.io](https://img.shields.io/crates/v/logshrubs.svg)
![CI](https://github.com/CaroJoya/LogShrubs/actions/workflows/ci.yml/badge.svg)
A fast, local-first CLI for analyzing application and server logs.
Supports plain text, JSON Lines, Apache, and nginx logs. Everything runs
locally---no uploads or telemetry.
Features
Flexible input: Files, multiple files, directories, or stdin.
Format detection: Text, JSON Lines, Apache, and nginx.
Log statistics: Counts for log levels and time ranges.
Error analysis: Groups recurring errors by normalizing variable
values and detects error spikes.
HTTP analysis: Status codes, popular endpoints, and
response-time statistics.
Live monitoring: Watch log files, including after truncation or
rotation.
Parallel processing: Analyze multiple inputs using multiple
workers.
JSON output: Machine-readable reports for scripts and tools such
as `jq`.
Installation
From crates.io
``` bash
cargo install logshrubs
```
Prebuilt binaries
Download the archive for your platform from GitHub
Releases.
From source
``` bash
git clone https://github.com/CaroJoya/LogShrubs.git
cd LogShrubs
cargo install --path .
```
Usage
``` bash
logshrubs analyze app.log
logshrubs stats app.log
logshrubs errors app.log
logshrubs http app.log
logshrubs watch app.log
```
Running `logshrubs` without a subcommand analyzes standard input.
Input examples
``` bash
logshrubs analyze a.log b.log c.log
logshrubs stats /var/log/myapp
logshrubs errors /var/log/myapp --no-recursive
cat app.log | logshrubs
docker logs web 2>&1 | logshrubs errors
```
A directory is scanned recursively by default for `.log` and `.txt`
files. Use `--no-recursive` to scan only the top level. Use `-` to read
standard input explicitly.
Common options
---
Option                                       Description
---
`--top N`                                    Show the top N errors or endpoints
(default: 10).
`--json`                                     Output machine-readable JSON.
`--format auto\|text\|json\|apache\|nginx`   Select the log format.
`--quiet`, `-q`                              Hide empty report sections.
`--verbose`, `-v`                            Show wider columns, more rows, and
input details.
`--no-recursive`                             Do not scan subdirectories.
`--jobs N`, `-j N`                           Set worker threads; `0` selects
automatically and `1` runs
sequentially.
Examples:
``` bash
logshrubs errors app.log --top 5
logshrubs analyze app.log --json
logshrubs analyze logs/ --quiet
```
Live watch
``` bash
logshrubs watch app.log
logshrubs watch app.log --interval 1000
logshrubs watch app.log --once
logshrubs watch app.log --json
```
Watch mode refreshes a live summary and handles partially written lines,
truncation, and log rotation.
Supported formats
Set a format with `--format`, or use `auto` (the default).
Auto-detection checks the first 20 lines of each input.
---
Format                              Supported data
---
`text`                              Timestamps, log levels, messages,
and supported HTTP patterns in text
logs.
`json`                              One JSON object per line. Supports
common timestamp (`timestamp`,
`time`, `ts`), level (`level`,
`severity`, `lvl`), and message
(`message`, `msg`) fields. Optional
HTTP fields include `method`,
`path`, `status`, and
`duration_ms`.
`apache`                            Common and combined access logs,
including timestamps, methods,
paths, and status codes.
`nginx`                             Access logs and error logs;
access-log request times can
provide latency data.
With a structured format, lines that do not match the selected parser
fall back to the text parser.
HTTP latency
When request durations are available, reports include average, median,
p95, p99, minimum, and maximum response times, plus slowest endpoints.
JSON output includes latency fields under `http`; these fields are
`null` when no request contains a duration.
Percentiles use a reservoir of up to 100,000 timed requests. They are
exact up to that limit and estimated from a reproducible sample for
larger inputs. Average, minimum, and maximum remain exact.
Development
``` bash
cargo test
cargo clippy --all-targets -- -D warnings
```
CI tests run on Linux, macOS, and Windows. Pushing a `v*` tag builds
release binaries and attaches them to a GitHub Release.
See CHANGELOG.md for version history.