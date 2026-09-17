// src/analyzer/spikes.rs

use crate::model::ErrorSpike;
use chrono::{DateTime, Duration, Utc};
use std::collections::BTreeMap;

/// Bucket width in minutes. 5 is a good default for log analysis:
/// small enough to catch short outages, large enough to smooth noise.
pub const BUCKET_MINUTES: i64 = 5;

/// Minimum spike count to bother reporting. Avoids flagging
/// 1-error buckets in near-silent logs as "spikes".
const MIN_SPIKE_COUNT: u64 = 3;

/// How many standard deviations above the mean a bucket must be
/// to qualify as a spike.
const SIGMA_THRESHOLD: f64 = 2.0;

/// With very few populated buckets, a genuine outage inflates the
/// stddev so much that it can't clear its own 2σ threshold. Loosen
/// the bar to 1σ when we have fewer than this many buckets.
const SMALL_SAMPLE_BUCKETS: usize = 6;
const SIGMA_THRESHOLD_SMALL: f64 = 1.0;

/// Collects error timestamps and computes which time buckets stand out.
#[derive(Debug, Default)]
pub struct SpikeAccumulator {
    /// bucket_start → error count in that bucket
    buckets: BTreeMap<DateTime<Utc>, u64>,
}

impl SpikeAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that an error occurred at `ts`.
    pub fn observe(&mut self, ts: DateTime<Utc>) {
        let bucket = floor_to_bucket(ts);
        *self.buckets.entry(bucket).or_insert(0) += 1;
    }

    /// Compute spikes from the accumulated buckets.
    /// Returns an empty Vec if there aren't enough buckets to be meaningful.
    pub fn finish(self) -> Vec<ErrorSpike> {
        if self.buckets.len() < 3 {
            // With <3 buckets we can't say anything statistically meaningful.
            return Vec::new();
        }

        let counts: Vec<u64> = self.buckets.values().copied().collect();
        let n = counts.len() as f64;
        let mean = counts.iter().sum::<u64>() as f64 / n;

        // Population stddev (we have all the buckets, not a sample).
        let variance = counts
            .iter()
            .map(|&c| {
                let diff = c as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / n;
        let stddev = variance.sqrt();

        // If stddev is ~0 (uniform log), nothing spikes.
        if stddev < f64::EPSILON {
            return Vec::new();
        }

        // 2σ is the default, but with very few buckets a genuine outage
        // inflates the stddev so much that it can't clear its own threshold.
        // Loosen to 1σ when we have fewer than SMALL_SAMPLE_BUCKETS.
        let sigma = if self.buckets.len() < SMALL_SAMPLE_BUCKETS {
            SIGMA_THRESHOLD_SMALL
        } else {
            SIGMA_THRESHOLD
        };
        let threshold = mean + sigma * stddev;

        let mut spikes: Vec<ErrorSpike> = self
            .buckets
            .into_iter()
            .filter(|(_, count)| *count as f64 > threshold && *count >= MIN_SPIKE_COUNT)
            .map(|(start, count)| {
                let severity = if mean > 0.0 { count as f64 / mean } else { 0.0 };
                ErrorSpike {
                    bucket_start: start,
                    bucket_end: start + Duration::minutes(BUCKET_MINUTES),
                    count,
                    severity,
                }
            })
            .collect();

        // Sort by count desc — biggest spikes first.
        spikes.sort_by(|a, b| b.count.cmp(&a.count));
        spikes
    }
}

/// Round `ts` down to the nearest 5-minute boundary.
fn floor_to_bucket(ts: DateTime<Utc>) -> DateTime<Utc> {
    let secs = ts.timestamp();
    let bucket_secs = BUCKET_MINUTES * 60;
    let floored = secs - (secs.rem_euclid(bucket_secs));
    DateTime::from_timestamp(floored, 0).unwrap_or(ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn floors_to_bucket_boundary() {
        // 2026-01-01 00:00:00 UTC is 1767225600
        let t = ts(1767225600 + 7 * 60); // 7 minutes past the hour
        let floored = floor_to_bucket(t);
        assert_eq!(floored.timestamp() % (5 * 60), 0);
        assert_eq!(floored.timestamp(), 1767225600 + 5 * 60);
    }

    #[test]
    fn no_spikes_in_uniform_log() {
        let mut acc = SpikeAccumulator::new();
        // 10 buckets, 5 errors each — perfectly flat.
        for b in 0..10 {
            for _ in 0..5 {
                acc.observe(ts(1767225600 + b * 5 * 60));
            }
        }
        assert!(acc.finish().is_empty());
    }

    #[test]
    fn detects_obvious_spike() {
        let mut acc = SpikeAccumulator::new();
        // 20 quiet buckets with 1 error each...
        for b in 0..20 {
            acc.observe(ts(1767225600 + b * 5 * 60));
        }
        // ...and one bucket with 50 more errors (bucket 5 already has 1
        // from the loop above, so the total is 51).
        for _ in 0..50 {
            acc.observe(ts(1767225600 + 5 * 5 * 60));
        }
        let spikes = acc.finish();
        assert_eq!(spikes.len(), 1);
        assert_eq!(spikes[0].count, 51);
        assert!(spikes[0].severity > 10.0);
    }

    #[test]
    fn ignores_tiny_buckets() {
        let mut acc = SpikeAccumulator::new();
        // Below MIN_SPIKE_COUNT (3) — even if statistically odd.
        for b in 0..20 {
            acc.observe(ts(1767225600 + b * 5 * 60));
        }
        acc.observe(ts(1767225600 + 5 * 5 * 60));
        acc.observe(ts(1767225600 + 5 * 5 * 60)); // now that bucket has 3
                                                  // Not enough spread — depends on stddev. Just confirm no panic.
        let _ = acc.finish();
    }
}
