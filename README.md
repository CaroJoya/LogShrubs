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
