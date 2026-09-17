// src/main.rs

mod analyzer;
mod cli;
mod error;
mod input;
mod model;
mod output;
mod parser;

use analyzer::{analyze_file, ErrorAccumulator, HttpAccumulator, StatsAccumulator};
use clap::Parser;
use cli::{Cli, Command};
use error::{LogLensError, Result};
use input::LogReader;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Analyze { file, top } => {
            let result = analyze_file(&file, top)
                .map_err(|e| LogLensError::from_io(&file, e))?;

            if cli.json {
                println!("{}", output::json::render_analysis(&result)?);
            } else {
                output::terminal::print_analyze(&result, top);
            }
        }

        Command::Errors { file, top } => {
            let reader = LogReader::open(&file)
                .map_err(|e| LogLensError::from_io(&file, e))?;
            let source = reader.path().display().to_string();

            let mut acc = ErrorAccumulator::new();
            for line in reader {
                let event = parser::parse_line(&line);
                acc.observe(&event);
            }
            let summary = acc.finish(top);

            if cli.json {
                println!("{}", output::json::render_errors(&source, &summary)?);
            } else {
                output::terminal::print_errors(&source, &summary, top);
            }
        }

        Command::Stats { file } => {
            let reader = LogReader::open(&file)
                .map_err(|e| LogLensError::from_io(&file, e))?;
            let source = reader.path().display().to_string();

            let mut acc = StatsAccumulator::new();
            for line in reader {
                let event = parser::parse_line(&line);
                acc.observe(&event);
            }
            let stats = acc.finish();

            if cli.json {
                println!("{}", output::json::render_stats(&source, &stats)?);
            } else {
                output::terminal::print_stats(&source, &stats);
            }
        }

        Command::Http { file, top } => {
            let reader = LogReader::open(&file)
                .map_err(|e| LogLensError::from_io(&file, e))?;
            let source = reader.path().display().to_string();

            let mut acc = HttpAccumulator::new();
            for line in reader {
                let event = parser::parse_line(&line);
                acc.observe(&event);
            }
            let summary = acc.finish(top);

            if cli.json {
                println!("{}", output::json::render_http(&source, &summary)?);
            } else {
                output::terminal::print_http(&source, &summary, top);
            }
        }

        Command::Version => {
            if cli.json {
                let v = serde_json::json!({
                    "name": "loglens",
                    "version": env!("CARGO_PKG_VERSION"),
                });
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("loglens {}", env!("CARGO_PKG_VERSION"));
            }
        }
    }

    Ok(())
}