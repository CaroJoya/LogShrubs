// src/analyzer/mod.rs

pub mod engine;
pub mod errors;
pub mod http;
pub mod performance;
pub mod spikes;
pub mod stats;

pub use engine::{analyze_sources, Options};
pub use errors::ErrorAccumulator;
pub use http::HttpAccumulator;
pub use stats::StatsAccumulator;
