// src/output/json.rs

use crate::error::Result;
use crate::model::{AnalysisResult, ErrorSummary, HttpSummary, Stats};

/// Render an AnalysisResult as pretty JSON.
/// AnalysisResult already carries `source` as a field.
pub fn render_analysis(result: &AnalysisResult) -> Result<String> {
    Ok(serde_json::to_string_pretty(result)?)
}

/// Render an ErrorSummary as pretty JSON.
pub fn render_errors(source: &str, summary: &ErrorSummary) -> Result<String> {
    Ok(serde_json::to_string_pretty(&Wrapper {
        source,
        data: summary,
    })?)
}

/// Render a Stats as pretty JSON.
pub fn render_stats(source: &str, stats: &Stats) -> Result<String> {
    Ok(serde_json::to_string_pretty(&Wrapper {
        source,
        data: stats,
    })?)
}

/// Render an HttpSummary as pretty JSON.
pub fn render_http(source: &str, summary: &HttpSummary) -> Result<String> {
    Ok(serde_json::to_string_pretty(&Wrapper {
        source,
        data: summary,
    })?)
}

/// Small wrapper so partial outputs (stats/errors/http) include
/// the source file. AnalysisResult already has `source` as a field.
#[derive(serde::Serialize)]
struct Wrapper<'a, T: serde::Serialize> {
    source: &'a str,
    data: &'a T,
}