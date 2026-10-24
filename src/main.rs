// src/main.rs

mod analyzer;
mod cli;
mod error;
mod input;
mod model;
mod output;
mod parser;
mod watch;

use analyzer::{analyze_sources, Options};
use clap::Parser;
use cli::{Cli, Command, InputArgs};
use error::Result;
use model::AnalysisResult;
use output::terminal::RenderOpts;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

/// Resolve inputs and run the analysis engine for one command.
fn analyze(cli: &Cli, input: &InputArgs, top: usize) -> Result<(AnalysisResult, RenderOpts)> {
    let render = RenderOpts {
        top_n: top,
        quiet: cli.quiet,
        verbose: cli.verbose,
    };
    let sources = input::resolve_inputs(&input.paths, !input.no_recursive)?;
    let opts = Options {
        top_n: render.rows(),
        format: cli.format.into(),
        jobs: cli.jobs,
    };
    let result = analyze_sources(&sources, &opts)?;
    Ok((result, render))
}

fn run(cli: Cli) -> Result<()> {
    // `logshrubs` with no subcommand analyzes stdin.
    let command = match &cli.command {
        Some(c) => c,
        None => &Command::Analyze {
            input: InputArgs::default(),
            top: 10,
        },
    };

    match command {
        Command::Analyze { input, top } => {
            let (result, render) = analyze(&cli, input, *top)?;
            if cli.json {
                println!("{}", output::json::render_analysis(&result)?);
            } else {
                output::terminal::print_analyze(&result, &render);
            }
        }

        Command::Errors { input, top } => {
            let (result, render) = analyze(&cli, input, *top)?;
            if cli.json {
                println!(
                    "{}",
                    output::json::render_errors(&result.source, &result.errors)?
                );
            } else {
                output::terminal::print_errors(&result, &render);
            }
        }

        Command::Stats { input } => {
            let (result, render) = analyze(&cli, input, 10)?;
            if cli.json {
                println!(
                    "{}",
                    output::json::render_stats(&result.source, &result.stats)?
                );
            } else {
                output::terminal::print_stats(&result, &render);
            }
        }

        Command::Http { input, top } => {
            let (result, render) = analyze(&cli, input, *top)?;
            if cli.json {
                println!(
                    "{}",
                    output::json::render_http(&result.source, &result.http)?
                );
            } else {
                output::terminal::print_http(&result, &render);
            }
        }

        Command::Watch {
            file,
            top,
            interval,
            once,
        } => {
            watch::run(file, cli.format.into(), *top, *interval, *once, cli.json)?;
        }

        Command::Version => {
            if cli.json {
                let v = serde_json::json!({
                    "name": "logshrubs",
                    "version": env!("CARGO_PKG_VERSION"),
                });
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("logshrubs {}", env!("CARGO_PKG_VERSION"));
            }
        }
    }

    Ok(())
}