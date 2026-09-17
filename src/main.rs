// src/main.rs
#![allow(dead_code)]   
mod cli;
mod input;
mod model;
mod parser;

use clap::Parser;
use cli::{Cli, Command};
use input::LogReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze { file, top: _ } => {
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
            println!("[errors] file={:?} top={}", file, top);
        }
        Command::Stats { file } => {
            println!("[stats] file={:?}", file);
        }
        Command::Http { file, top } => {
            println!("[http] file={:?} top={}", file, top);
        }
        Command::Version => {
            println!("loglens {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}