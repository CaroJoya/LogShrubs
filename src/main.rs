// src/main.rs

mod analyzer;
mod cli;
mod input;
mod model;
mod output;
mod parser;

use analyzer::{ErrorAccumulator, HttpAccumulator, StatsAccumulator};
use clap::Parser;
use cli::{Cli, Command};
use input::LogReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze { file, top: _ } => {
            // Still temporary — full analyze comes in Step 7.
            // For now, just parse and print first 20 events.
            let reader = LogReader::open(&file)?;
            let mut count: u64 = 0;
            for line in reader {
                let event = parser::parse_line(&line);
                println!(
                    "ts={:?} level={:?} msg={:?}",
                    event.timestamp.map(|t| t.to_rfc3339()),
                    event.level.map(|l| l.as_str()),
                    event.message,
                );
                count += 1;
                if count >= 20 {
                    println!("... (showing first 20 lines only)");
                    break;
                }
            }
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