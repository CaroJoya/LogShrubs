// src/cli.rs

use crate::parser::{Format, FormatChoice};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "logshrubs",
    version,
    about = "A fast, local-first log analyzer",
    long_about = "LogShrubs reads application/server log files and turns raw logs \
                  into useful diagnostic information.\n\n\
                  Give it files, directories, or pipe logs into it \
                  (`docker logs web | logshrubs`). With no subcommand it runs \
                  `analyze` on stdin.\n\n\
                  Nothing is uploaded — all analysis runs locally."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Output as JSON (machine-readable)
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress empty sections and non-essential lines
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Show more detail: wider columns, extra rows, per-input info
    #[arg(long, short, global = true)]
    pub verbose: bool,

    /// Log format: auto-detected per file by default
    #[arg(long, global = true, value_enum, default_value_t = FormatArg::Auto)]
    pub format: FormatArg,

    /// Worker threads when reading several inputs (0 = automatic, 1 = sequential)
    #[arg(long, short = 'j', global = true, default_value_t = 0)]
    pub jobs: usize,
}

/// Value of `--format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FormatArg {
    Auto,
    Text,
    Json,
    Apache,
    Nginx,
}

impl From<FormatArg> for FormatChoice {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Auto => FormatChoice::Auto,
            FormatArg::Text => FormatChoice::Fixed(Format::Text),
            FormatArg::Json => FormatChoice::Fixed(Format::Json),
            FormatArg::Apache => FormatChoice::Fixed(Format::Apache),
            FormatArg::Nginx => FormatChoice::Fixed(Format::Nginx),
        }
    }
}

/// Where to read logs from.
#[derive(Args, Debug, Default, Clone)]
pub struct InputArgs {
    /// Files or directories to read; `-` (or nothing) reads stdin.
    /// Directories are searched recursively for .log and .txt files.
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Only read the top level of directories
    #[arg(long)]
    pub no_recursive: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Full log analysis (the main command)
    Analyze {
        #[command(flatten)]
        input: InputArgs,

        /// Show top N errors (default: 10)
        #[arg(long, default_value_t = 10)]
        top: usize,
    },

    /// Show recurring errors only
    Errors {
        #[command(flatten)]
        input: InputArgs,

        #[arg(long, default_value_t = 10)]
        top: usize,
    },

    /// Show overall statistics only
    Stats {
        #[command(flatten)]
        input: InputArgs,
    },

    /// Analyze HTTP requests only
    Http {
        #[command(flatten)]
        input: InputArgs,

        #[arg(long, default_value_t = 10)]
        top: usize,
    },

    /// Follow a log file and keep a live summary up to date
    Watch {
        /// The log file to follow
        file: PathBuf,

        /// Show top N recent/recurring items (default: 10)
        #[arg(long, default_value_t = 10)]
        top: usize,

        /// Refresh interval in milliseconds when no change event arrives
        #[arg(long, default_value_t = 2000)]
        interval: u64,

        /// Read the file once, print one summary and exit
        #[arg(long)]
        once: bool,
    },

    /// Show installed version
    Version,
}
