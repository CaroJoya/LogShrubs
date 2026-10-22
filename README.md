# LogShrubs 🌿

[![Crates.io](https://img.shields.io/crates/v/logshrubs.svg)](https://crates.io/crates/logshrubs)

A fast, local-first CLI that analyzes application and server log files.

LogShrubs reads raw text logs and turns them into useful diagnostic information right in your terminal. No data is ever uploaded, no telemetry, and no complex setup — just point it at a file.

## Features

- **Fast & Local:** Runs entirely on your machine.
- **Level Stats:** Instant counts of INFO, DEBUG, WARN, ERROR, and FATAL lines.
- **Smart Error Grouping:** Normalizes variable data (like `User 18273` → `User <NUM>`) so you can see the *actual* most frequent errors.
- **Error Spikes:** Automatically detects statistical anomalies (e.g., 50 errors in a 5-minute window) using standard deviation.
- **HTTP Analysis:** Extracts status codes (2xx, 4xx, 5xx) and finds your most hit endpoints.
- **JSON Output:** Pipe results into `jq` or other tools using the `--json` flag.

## Installation

### Via Cargo (Recommended)

If you have Rust installed, you can install LogShrubs directly from crates.io:

```bash
cargo install logshrubs
```

### From Source

```bash
git clone https://github.com/CaroJoya/LogShrubs.git
cd LogShrubs
cargo install --path .
```

## Usage

LogShrubs has a few focused commands.

### Full Analysis (The Main Command)

Get a complete breakdown of the file, including levels, time range, top errors, spikes, and HTTP stats.

```bash
logshrubs analyze app.log
```

### Specific Reports

If you only care about one thing, use a specific command:

```bash
# Just the level counts and time range
logshrubs stats app.log

# Just the top recurring errors and spikes
logshrubs errors app.log

# Just HTTP requests and top endpoints
logshrubs http app.log
```

### Command Options

You can limit the number of results shown (default is 10) and output machine-readable JSON.

```bash
# Show the top 5 errors
logshrubs errors app.log --top 5

# Output the full analysis as JSON
logshrubs analyze app.log --json
```

## Example Output

Running `logshrubs analyze server.log` will produce a clean terminal report:

```text
ANALYSIS
─────────────────────────────────
File        server.log
Lines       1420

LEVELS
─────────────────────────────────
INFO           980
WARN           210
ERROR          230
FATAL            0
DEBUG            0
TRACE            0
UNLEVELED        0

TIME
─────────────────────────────────
First       2026-09-15 14:00:00 UTC
Last        2026-09-15 15:45:00 UTC
Duration    1h 45m 0s

TOP ERRORS
─────────────────────────────────
 1. Database connection timeout              45
 2. Auth failed for user <NUM>               32
 3. Payment gateway unreachable              12

ERROR SPIKES
─────────────────────────────────
14:30 → 14:35     51 errors   ⚠ 35.0× normal

HTTP
─────────────────────────────────
Requests    850
2xx         720
3xx          20
4xx          85
5xx          25

TOP ENDPOINTS
─────────────────────────────────
 1. /api/products                           210
 2. /api/users                              180
 3. /api/checkout                            95
```

## Supported Log Formats

LogShrubs is designed to handle messy logs. It automatically detects:

- **Timestamps:** ISO 8601 (`2026-09-15 14:21:04` or `2026-09-15T14:21:04Z`).
- **Levels:** `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`, `FATAL` (and variations like `ERR`, `WARNING`).
- **HTTP Requests:** Simple format (`GET /api/users 200 42ms`) and Combined/Apache format (`"GET /api/users HTTP/1.1" 200`).
