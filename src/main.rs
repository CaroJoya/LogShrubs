// src/main.rs

mod analyzer;
mod cli;
mod input;
mod model;
mod output;
mod parser;

use analyzer::{analyze_file, ErrorAccumulator, HttpAccumulator, StatsAccumulator};
use clap::Parser;
use cli::{Cli, Command};
use input::LogReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze { file, top } => {
            let result = analyze_file(&file, top)?;
            output::terminal::print_analyze(&result, top);
        }

        Command::Errors { file, top } => {
            let reader = LogReader::open(&file)?;
            let source = reader.path().display().to_string();

            let mut acc = ErrorAccumulator::new();
            for line in reader {
                let event = parser::parse_line(&line);
                acc.observe(&event);
            }
            let summary = acc.finish(top);

            output::terminal::print_errors(&source, &summary, top);
        }

        Command::Stats { file } => {
            let reader = LogReader::open(&file)?;
            let source = reader.path().display().to_string();

            let mut acc = StatsAccumulator::new();
            for line in reader {
                let event = parser::parse_line(&line);
                acc.observe(&event);
            }
            let stats = acc.finish();

            output::terminal::print_stats(&source, &stats);
        }

        Command::Http { file, top } => {
            let reader = LogReader::open(&file)?;
            let source = reader.path().display().to_string();

            let mut acc = HttpAccumulator::new();
            for line in reader {
                let event = parser::parse_line(&line);
                acc.observe(&event);
            }
            let summary = acc.finish(top);

            output::terminal::print_http(&source, &summary, top);
        }

        Command::Version => {
            println!("loglens {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
