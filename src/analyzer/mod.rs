// src/analyzer/mod.rs

pub mod errors;
pub mod spikes;
pub mod stats;

pub use errors::ErrorAccumulator;
pub use stats::StatsAccumulator;