// src/main.rs

mod cli;
mod model;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let result: Result<(), Box<dyn std::error::Error>> = match cli.command {
        Command::Analyze { file, top } => {
            println!(
                "[analyze] file={:?} top={} json={} quiet={} verbose={}",
                file, top, cli.json, cli.quiet, cli.verbose
            );
            Ok(())
        }
        Command::Errors { file, top } => {
            println!("[errors] file={:?} top={}", file, top);
            Ok(())
        }
        Command::Stats { file } => {
            println!("[stats] file={:?}", file);
            Ok(())
        }
        Command::Http { file, top } => {
            println!("[http] file={:?} top={}", file, top);
            Ok(())
        }
        Command::Version => {
            println!("loglens {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}