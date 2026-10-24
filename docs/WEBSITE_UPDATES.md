# index.html updates needed for 1.0.0

`index.html` wasn't part of the files provided, so it hasn't been edited. These are the exact changes it needs so the site matches the binary.
Everything below is already implemented and tested — use the commands as the demo text.

## Version / install
- Version tab: `logshrubs 1.0.0` (JSON: `{ "name": "logshrubs", "version": "1.0.0" }`).
- Install section: add **Prebuilt binaries** beside crates.io, linking to
  `https://github.com/CaroJoya/LogShrubs/releases` — Linux x86_64 (`.tar.gz`), macOS Apple Silicon (`.tar.gz`), macOS Intel (`.tar.gz`), Windows x86_64 (`.zip`).
- Footer: add a **CHANGELOG** link → `https://github.com/CaroJoya/LogShrubs/blob/main/CHANGELOG.md`.

## New "Inputs" section
| You pass | What happens |
| --- | --- |
| `app.log` | Reads that file |
| `a.log b.log c.log` | Merged into one report |
| `logs/` | Every `.log` / `.txt`, recursive (`--no-recursive` for top level only) |
| `-` or nothing | Reads stdin: `cat app.log \| logshrubs`, `docker logs web \| logshrubs` |

## Commands demo
- Add a **stdin** tab: `cat app.log | logshrubs` (output starts `File        <stdin>`, no Size line).
- Add a **watch** tab: `logshrubs watch app.log` — shows `WATCH`, level counts, `SPIKE STATUS`, `RECENT ERRORS`.
- `http` and `analyze` tabs: add the `RESPONSE TIME` block (Average / Median / P95 / P99 / Min / Max) after the status counts and the `SLOW ENDPOINTS` block (`/api/orders   821.0 ms avg   64 req`) at the end. Add a `Size  218.4 KB` line under `File`.
- Options list: add `--format`, `--quiet`, `--verbose`, `--no-recursive`, `--jobs`.

## Supported formats
- Rewrite the "Supported Log Formats" section as the four-row table in README.md (text / json / apache / nginx).
- **"Not read" row:** remove "Apache timestamps". They are now read when the format is `apache`/`nginx` (explicit or auto-detected). Anything still unread: Apache `%D` durations, Apache *error* logs, non-UTC offsets in text/JSON logs without an explicit offset (treated as UTC).

## Parser inspector page
- Add a **format dropdown**: Auto / Text / JSON / Apache / nginx, so users can see how each parses the same line.
- Add a note: "`duration_ms` now feeds latency percentiles (average, median, p95, p99)."