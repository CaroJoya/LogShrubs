// src/analyzer/performance.rs
//
// Latency statistics from `HttpInfo.duration_ms`.
//
// Memory stays flat regardless of input size: durations go into a bounded
// reservoir (exact while fewer than RESERVOIR_CAP samples have been seen,
// uniform random sample afterwards, using a fixed seed so runs are
// reproducible). Average, min and max are tracked exactly on the side.

use crate::model::EndpointLatency;
use std::collections::HashMap;

/// Maximum number of duration samples kept for percentile estimation.
pub const RESERVOIR_CAP: usize = 100_000;

/// Fixed-capacity, deterministic reservoir sampler (Algorithm R).
#[derive(Debug, Clone)]
pub struct Reservoir {
    cap: usize,
    samples: Vec<u64>,
    seen: u64,
    rng: u64,
}

impl Reservoir {
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            samples: Vec::new(),
            seen: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    fn next_rand(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn observe(&mut self, v: u64) {
        self.seen += 1;
        if self.samples.len() < self.cap {
            self.samples.push(v);
        } else {
            let j = self.next_rand() % self.seen;
            if (j as usize) < self.cap {
                self.samples[j as usize] = v;
            }
        }
    }

    /// Combine two reservoirs. Exact when everything still fits; otherwise
    /// each side contributes in proportion to how many values it saw.
    pub fn merge(&mut self, other: Reservoir) {
        if other.seen == 0 {
            return;
        }
        let total = self.seen + other.seen;
        if self.samples.len() + other.samples.len() <= self.cap {
            self.samples.extend(other.samples);
            self.seen = total;
            return;
        }
        let share = (self.cap as u128 * self.seen as u128 / total as u128) as usize;
        let k_self = share.min(self.samples.len());
        let k_other = (self.cap - k_self).min(other.samples.len());
        let mut merged = stride_pick(&self.samples, k_self);
        merged.extend(stride_pick(&other.samples, k_other));
        self.samples = merged;
        self.seen = total;
    }

    fn sorted(&self) -> Vec<u64> {
        let mut v = self.samples.clone();
        v.sort_unstable();
        v
    }
}

fn stride_pick(src: &[u64], k: usize) -> Vec<u64> {
    if k == 0 || src.is_empty() {
        return Vec::new();
    }
    if k >= src.len() {
        return src.to_vec();
    }
    (0..k).map(|i| src[i * src.len() / k]).collect()
}

/// Nearest-rank percentile on an ascending slice (`p` in 0..=100).
fn percentile(sorted: &[u64], p: f64) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    Some(sorted[rank.min(sorted.len()) - 1])
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Finished latency numbers (copied into `HttpSummary`).
#[derive(Debug, Clone, Default)]
pub struct PerfSummary {
    pub avg_ms: Option<f64>,
    pub min_ms: Option<u64>,
    pub median_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub p99_ms: Option<u64>,
    pub max_ms: Option<u64>,
    pub slowest_endpoints: Vec<EndpointLatency>,
}

#[derive(Debug, Clone)]
pub struct PerformanceAccumulator {
    reservoir: Reservoir,
    count: u64,
    sum: u128,
    min: Option<u64>,
    max: Option<u64>,
    /// path → (requests with a duration, total duration)
    per_endpoint: HashMap<String, (u64, u128)>,
}

impl Default for PerformanceAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceAccumulator {
    pub fn new() -> Self {
        Self::with_capacity(RESERVOIR_CAP)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            reservoir: Reservoir::new(cap),
            count: 0,
            sum: 0,
            min: None,
            max: None,
            per_endpoint: HashMap::new(),
        }
    }

    pub fn observe(&mut self, path: &str, duration_ms: u64) {
        self.reservoir.observe(duration_ms);
        self.count += 1;
        self.sum += duration_ms as u128;
        self.min = Some(self.min.map_or(duration_ms, |m| m.min(duration_ms)));
        self.max = Some(self.max.map_or(duration_ms, |m| m.max(duration_ms)));

        let entry = match self.per_endpoint.get_mut(path) {
            Some(e) => e,
            None => self.per_endpoint.entry(path.to_string()).or_insert((0, 0)),
        };
        entry.0 += 1;
        entry.1 += duration_ms as u128;
    }

    pub fn merge(&mut self, other: PerformanceAccumulator) {
        self.reservoir.merge(other.reservoir);
        self.count += other.count;
        self.sum += other.sum;
        self.min = match (self.min, other.min) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        self.max = match (self.max, other.max) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        for (path, (n, sum)) in other.per_endpoint {
            let e = self.per_endpoint.entry(path).or_insert((0, 0));
            e.0 += n;
            e.1 += sum;
        }
    }

    /// Produce the summary. `top_n` limits the slowest-endpoint list.
    pub fn finish(self, top_n: usize) -> PerfSummary {
        if self.count == 0 {
            return PerfSummary::default();
        }
        let sorted = self.reservoir.sorted();

        let mut endpoints: Vec<EndpointLatency> = self
            .per_endpoint
            .into_iter()
            .map(|(path, (count, sum))| EndpointLatency {
                path,
                avg_ms: round2(sum as f64 / count as f64),
                count,
            })
            .collect();
        endpoints.sort_by(|a, b| {
            b.avg_ms
                .partial_cmp(&a.avg_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
        });
        endpoints.truncate(top_n);

        PerfSummary {
            avg_ms: Some(round2(self.sum as f64 / self.count as f64)),
            min_ms: self.min,
            median_ms: percentile(&sorted, 50.0),
            p95_ms: percentile(&sorted, 95.0),
            p99_ms: percentile(&sorted, 99.0),
            max_ms: self.max,
            slowest_endpoints: endpoints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_distribution_has_known_percentiles() {
        let mut acc = PerformanceAccumulator::new();
        for d in 1..=100u64 {
            acc.observe("/x", d);
        }
        let s = acc.finish(10);
        assert_eq!(s.median_ms, Some(50));
        assert_eq!(s.p95_ms, Some(95));
        assert_eq!(s.p99_ms, Some(99));
        assert_eq!(s.min_ms, Some(1));
        assert_eq!(s.max_ms, Some(100));
        assert_eq!(s.avg_ms, Some(50.5));
    }

    #[test]
    fn no_durations_means_no_stats() {
        let s = PerformanceAccumulator::new().finish(10);
        assert!(s.avg_ms.is_none());
        assert!(s.median_ms.is_none());
        assert!(s.slowest_endpoints.is_empty());
    }

    #[test]
    fn slowest_endpoints_sorted_by_average() {
        let mut acc = PerformanceAccumulator::new();
        acc.observe("/fast", 10);
        acc.observe("/fast", 20);
        acc.observe("/slow", 800);
        acc.observe("/slow", 842);
        acc.observe("/mid", 100);
        let s = acc.finish(2);
        assert_eq!(s.slowest_endpoints.len(), 2);
        assert_eq!(s.slowest_endpoints[0].path, "/slow");
        assert_eq!(s.slowest_endpoints[0].avg_ms, 821.0);
        assert_eq!(s.slowest_endpoints[1].path, "/mid");
    }

    #[test]
    fn memory_is_bounded_but_exact_stats_stay_exact() {
        let mut acc = PerformanceAccumulator::with_capacity(1000);
        for d in 1..=50_000u64 {
            acc.observe("/x", d);
        }
        assert_eq!(acc.reservoir.samples.len(), 1000);
        let s = acc.finish(1);
        assert_eq!(s.min_ms, Some(1));
        assert_eq!(s.max_ms, Some(50_000));
        assert_eq!(s.avg_ms, Some(25_000.5));
        // Sampled percentile should be close to the truth (25_000).
        let med = s.median_ms.unwrap() as i64;
        assert!((med - 25_000).abs() < 5_000, "median {}", med);
    }

    #[test]
    fn merge_matches_single_accumulator() {
        let mut whole = PerformanceAccumulator::new();
        let mut a = PerformanceAccumulator::new();
        let mut b = PerformanceAccumulator::new();
        for d in 1..=60u64 {
            whole.observe("/x", d);
            a.observe("/x", d);
        }
        for d in 61..=100u64 {
            whole.observe("/y", d);
            b.observe("/y", d);
        }
        a.merge(b);
        let (w, m) = (whole.finish(5), a.finish(5));
        assert_eq!(w.median_ms, m.median_ms);
        assert_eq!(w.p95_ms, m.p95_ms);
        assert_eq!(w.p99_ms, m.p99_ms);
        assert_eq!(w.avg_ms, m.avg_ms);
        assert_eq!(w.slowest_endpoints.len(), m.slowest_endpoints.len());
    }
}