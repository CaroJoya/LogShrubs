// src/analyzer/http.rs

use crate::analyzer::performance::PerformanceAccumulator;
use crate::model::{EndpointCount, HttpSummary, LogEvent};
use std::collections::HashMap;

/// Folds HTTP request events into aggregate statistics.
#[derive(Debug, Clone, Default)]
pub struct HttpAccumulator {
    total_requests: u64,
    s2xx: u64,
    s3xx: u64,
    s4xx: u64,
    s5xx: u64,
    other: u64,
    endpoints: HashMap<String, u64>,
    perf: PerformanceAccumulator,
}

impl HttpAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Observe one event. Only events with `http: Some(...)` are recorded.
    pub fn observe(&mut self, event: &LogEvent) {
        let info = match &event.http {
            Some(i) => i,
            None => return,
        };

        self.total_requests += 1;

        match info.status {
            200..=299 => self.s2xx += 1,
            300..=399 => self.s3xx += 1,
            400..=499 => self.s4xx += 1,
            500..=599 => self.s5xx += 1,
            _ => self.other += 1,
        }

        *self.endpoints.entry(info.path.clone()).or_insert(0) += 1;

        if let Some(d) = info.duration_ms {
            self.perf.observe(&info.path, d);
        }
    }

    /// Fold another accumulator (e.g. from a different file) into this one.
    pub fn merge(&mut self, other: HttpAccumulator) {
        self.total_requests += other.total_requests;
        self.s2xx += other.s2xx;
        self.s3xx += other.s3xx;
        self.s4xx += other.s4xx;
        self.s5xx += other.s5xx;
        self.other += other.other;
        for (path, count) in other.endpoints {
            *self.endpoints.entry(path).or_insert(0) += count;
        }
        self.perf.merge(other.perf);
    }

    /// Consume the accumulator and produce the final HttpSummary.
    /// `top_n` limits how many distinct endpoints are kept.
    pub fn finish(self, top_n: usize) -> HttpSummary {
        let mut entries: Vec<(String, u64)> = self.endpoints.into_iter().collect();

        // Sort by count desc, then by path asc (stable tie-break).
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let top_endpoints = entries
            .into_iter()
            .take(top_n)
            .map(|(path, count)| EndpointCount { path, count })
            .collect();

        let perf = self.perf.finish(top_n);

        HttpSummary {
            total_requests: self.total_requests,
            s2xx: self.s2xx,
            s3xx: self.s3xx,
            s4xx: self.s4xx,
            s5xx: self.s5xx,
            other: self.other,
            top_endpoints,
            avg_ms: perf.avg_ms,
            min_ms: perf.min_ms,
            median_ms: perf.median_ms,
            p95_ms: perf.p95_ms,
            p99_ms: perf.p99_ms,
            max_ms: perf.max_ms,
            slowest_endpoints: perf.slowest_endpoints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::HttpInfo;

    fn ev(method: &str, path: &str, status: u16, dur: Option<u64>) -> LogEvent {
        LogEvent {
            timestamp: None,
            level: None,
            message: format!("{} {} {}", method, path, status),
            http: Some(HttpInfo {
                method: method.to_string(),
                path: path.to_string(),
                status,
                duration_ms: dur,
            }),
        }
    }

    #[test]
    fn counts_status_classes() {
        let mut acc = HttpAccumulator::new();
        acc.observe(&ev("GET", "/a", 200, None));
        acc.observe(&ev("GET", "/a", 201, None));
        acc.observe(&ev("POST", "/b", 301, None));
        acc.observe(&ev("GET", "/c", 404, None));
        acc.observe(&ev("GET", "/d", 500, None));
        acc.observe(&ev("GET", "/e", 0, None));
        let s = acc.finish(10);
        assert_eq!(s.total_requests, 6);
        assert_eq!(s.s2xx, 2);
        assert_eq!(s.s3xx, 1);
        assert_eq!(s.s4xx, 1);
        assert_eq!(s.s5xx, 1);
        assert_eq!(s.other, 1);
    }

    #[test]
    fn ignores_non_http_events() {
        let mut acc = HttpAccumulator::new();
        acc.observe(&LogEvent {
            timestamp: None,
            level: None,
            message: "not http".to_string(),
            http: None,
        });
        assert_eq!(acc.finish(10).total_requests, 0);
    }

    #[test]
    fn sorts_endpoints_by_count_desc() {
        let mut acc = HttpAccumulator::new();
        acc.observe(&ev("GET", "/users", 200, None));
        acc.observe(&ev("GET", "/users", 200, None));
        acc.observe(&ev("GET", "/products", 200, None));
        let s = acc.finish(10);
        assert_eq!(s.top_endpoints[0].path, "/users");
        assert_eq!(s.top_endpoints[0].count, 2);
    }

    #[test]
    fn latency_feeds_summary() {
        let mut acc = HttpAccumulator::new();
        acc.observe(&ev("GET", "/a", 200, Some(100)));
        acc.observe(&ev("GET", "/a", 200, Some(300)));
        acc.observe(&ev("GET", "/b", 200, None));
        let s = acc.finish(10);
        assert_eq!(s.avg_ms, Some(200.0));
        assert_eq!(s.max_ms, Some(300));
        assert_eq!(s.slowest_endpoints.len(), 1);
        assert_eq!(s.slowest_endpoints[0].path, "/a");
    }

    #[test]
    fn no_durations_no_latency() {
        let mut acc = HttpAccumulator::new();
        acc.observe(&ev("GET", "/a", 200, None));
        let s = acc.finish(10);
        assert!(s.avg_ms.is_none());
        assert!(s.slowest_endpoints.is_empty());
    }

    #[test]
    fn respects_top_n() {
        let mut acc = HttpAccumulator::new();
        acc.observe(&ev("GET", "/a", 200, None));
        acc.observe(&ev("GET", "/b", 200, None));
        acc.observe(&ev("GET", "/c", 200, None));
        let s = acc.finish(2);
        assert_eq!(s.top_endpoints.len(), 2);
    }
}
