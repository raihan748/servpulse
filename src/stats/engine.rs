use serde::{Deserialize, Serialize};
use crate::probe::tcp::ProbeResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaReport {
    pub target: String,
    pub total_probes: usize,
    pub successful_probes: usize,
    pub failed_probes: usize,
    pub uptime_percentage: f64,
    pub sla_tier: String,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub mean_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub jitter_ms: f64,
    pub sparkline: String,
}

pub fn calculate_sla_report(target: &str, results: &[ProbeResult]) -> SlaReport {
    if results.is_empty() {
        return SlaReport {
            target: target.to_string(),
            total_probes: 0,
            successful_probes: 0,
            failed_probes: 0,
            uptime_percentage: 0.0,
            sla_tier: "No Data".to_string(),
            min_latency_ms: 0.0,
            max_latency_ms: 0.0,
            mean_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p90_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            jitter_ms: 0.0,
            sparkline: "".to_string(),
        };
    }

    let total = results.len();
    let success_count = results.iter().filter(|r| r.success).count();
    let fail_count = total - success_count;
    let uptime = (success_count as f64 / total as f64) * 100.0;

    let sla_tier = if uptime >= 99.99 {
        "Four Nines (99.99% Enterprise HA)".to_string()
    } else if uptime >= 99.9 {
        "Three Nines (99.9% Production)".to_string()
    } else if uptime >= 99.0 {
        "Two Nines (99.0% Lab/Staging)".to_string()
    } else {
        "SLA Degraded (< 99.0%)".to_string()
    };

    let mut latencies: Vec<f64> = results.iter()
        .filter(|r| r.success)
        .map(|r| r.latency_ms)
        .collect();

    if latencies.is_empty() {
        return SlaReport {
            target: target.to_string(),
            total_probes: total,
            successful_probes: 0,
            failed_probes: fail_count,
            uptime_percentage: 0.0,
            sla_tier: "Offline / Failed".to_string(),
            min_latency_ms: 0.0,
            max_latency_ms: 0.0,
            mean_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p90_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            jitter_ms: 0.0,
            sparkline: "████████".to_string(),
        };
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min_lat = latencies[0];
    let max_lat = latencies[latencies.len() - 1];
    let sum_lat: f64 = latencies.iter().sum();
    let mean_lat = sum_lat / latencies.len() as f64;

    let p50 = percentile(&latencies, 50.0);
    let p90 = percentile(&latencies, 90.0);
    let p99 = percentile(&latencies, 99.0);

    // Jitter: average difference between consecutive latency measurements
    let mut jitter_acc = 0.0;
    if results.len() > 1 {
        for window in results.windows(2) {
            let diff = (window[1].latency_ms - window[0].latency_ms).abs();
            jitter_acc += diff;
        }
        jitter_acc /= (results.len() - 1) as f64;
    }

    let sparkline = generate_sparkline(&results.iter().map(|r| r.latency_ms).collect::<Vec<_>>());

    SlaReport {
        target: target.to_string(),
        total_probes: total,
        successful_probes: success_count,
        failed_probes: fail_count,
        uptime_percentage: (uptime * 100.0).round() / 100.0,
        sla_tier,
        min_latency_ms: (min_lat * 100.0).round() / 100.0,
        max_latency_ms: (max_lat * 100.0).round() / 100.0,
        mean_latency_ms: (mean_lat * 100.0).round() / 100.0,
        p50_latency_ms: (p50 * 100.0).round() / 100.0,
        p90_latency_ms: (p90 * 100.0).round() / 100.0,
        p99_latency_ms: (p99 * 100.0).round() / 100.0,
        jitter_ms: (jitter_acc * 100.0).round() / 100.0,
        sparkline,
    }
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[std::cmp::min(idx, sorted.len() - 1)]
}

pub fn generate_sparkline(values: &[f64]) -> String {
    if values.is_empty() { return "".to_string(); }
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let bars = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let range = max - min;

    values.iter().map(|&v| {
        if range <= 0.0001 {
            bars[0]
        } else {
            let normalized = (v - min) / range;
            let idx = (normalized * (bars.len() - 1) as f64).round() as usize;
            bars[std::cmp::min(idx, bars.len() - 1)]
        }
    }).collect()
}
