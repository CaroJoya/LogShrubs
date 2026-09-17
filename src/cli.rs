// src/cli.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "loglens",
    version,
    about = "A fast, local-first log analyzer",
    long_about = "LogLens reads application/server log files and turns raw logs \
                  into useful diagnostic information.\n\n\
                  Nothing is uploaded — all analysis runs locally."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress non-essential output
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Show more detail
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Full log analysis (the main command)
    Analyze {
        /// Path to the log file
        file: PathBuf,

        /// Show top N errors (default: 10)
        #[arg(long, default_value_t = 10)]
        top: usize,
    },

    /// Show recurring errors only
    Errors {
        file: PathBuf,

        #[arg(long, default_value_t = 10)]
        top: usize,
    },

    /// Show overall statistics only
    Stats { file: PathBuf },

    /// Analyze HTTP requests only
    Http {
        file: PathBuf,

        #[arg(long, default_value_t = 10)]
        top: usize,
    },

    /// Show installed version
    Version,
}
