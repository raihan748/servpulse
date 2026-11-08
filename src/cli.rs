use std::thread;
use std::time::Duration;
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;
use serde::{Deserialize, Serialize};

use crate::badge::svg::{export_badge_file, generate_badge, generate_uptime_badge};
use crate::probe::http::probe_http;
use crate::probe::tcp::{probe_tcp, ProbeResult};
use crate::stats::engine::{calculate_sla_report, SlaReport};

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeProtocol {
    Tcp,
    Http,
}

pub type ProtocolArg = ProbeProtocol;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "servpulse",
    author = "SMK Telkom Developer <dev@smktelkom-mks.sch.id>",
    version = "0.1.0",
    about = "Multi-Protocol ICMP/TCP/HTTP Uptime SLA Monitor & Terminal Dashboard"
)]
pub struct Cli {
    /// Target host or IP address (e.g. 127.0.0.1 or api.example.com)
    #[arg(long)]
    pub target: String,

    /// Target port to probe
    #[arg(long, default_value_t = 80)]
    pub port: u16,

    /// Probe protocol (tcp or http)
    #[arg(long, value_enum, default_value_t = ProbeProtocol::Tcp)]
    pub protocol: ProbeProtocol,

    /// HTTP request path (for HTTP protocol)
    #[arg(long, default_value = "/")]
    pub path: String,

    /// Number of probes to send
    #[arg(short = 'c', long, default_value_t = 5)]
    pub count: usize,

    /// Interval between probes in milliseconds
    #[arg(short = 'i', long, default_value_t = 500)]
    pub interval: u64,

    /// Timeout for each probe in milliseconds
    #[arg(short = 't', long, default_value_t = 2000)]
    pub timeout: u64,

    /// Output format (text or json)
    #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,

    /// Generate an SVG uptime status badge and save to file
    #[arg(short = 'b', long)]
    pub badge: Option<String>,

    /// Run single probe check instead of multi-probe monitor
    #[arg(long, default_value_t = false)]
    pub single: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// One-shot probe for TCP or HTTP target
    Check {
        target: String,
        #[arg(long, default_value_t = 80)]
        port: u16,
        #[arg(long, value_enum, default_value_t = ProbeProtocol::Tcp)]
        protocol: ProbeProtocol,
        #[arg(long, default_value = "/")]
        path: String,
        #[arg(short = 't', long, default_value_t = 2000)]
        timeout: u64,
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Dedicated TCP port latency probe
    Tcp {
        target: String,
        #[arg(long, default_value_t = 80)]
        port: u16,
        #[arg(short = 'c', long, default_value_t = 5)]
        count: usize,
        #[arg(short = 'i', long, default_value_t = 500)]
        interval: u64,
        #[arg(short = 't', long, default_value_t = 2000)]
        timeout: u64,
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        #[arg(short = 'b', long)]
        badge: Option<String>,
    },
    /// Dedicated HTTP endpoint latency probe
    Http {
        target: String,
        #[arg(long, default_value_t = 80)]
        port: u16,
        #[arg(long, default_value = "/")]
        path: String,
        #[arg(short = 'c', long, default_value_t = 5)]
        count: usize,
        #[arg(short = 'i', long, default_value_t = 500)]
        interval: u64,
        #[arg(short = 't', long, default_value_t = 2000)]
        timeout: u64,
        #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        #[arg(short = 'b', long)]
        badge: Option<String>,
    },
    /// Generate standalone SVG badge
    Badge {
        #[arg(long, default_value = "uptime")]
        label: String,
        #[arg(long, default_value = "99.9%")]
        status: String,
        #[arg(long, default_value = "#4c1")]
        color: String,
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
}

pub fn handle_cli(cli: &Cli) -> anyhow::Result<SlaReport> {
    let count = if cli.single { 1 } else { cli.count };
    let timeout = Duration::from_millis(cli.timeout);
    let interval = Duration::from_millis(cli.interval);

    if cli.format == OutputFormat::Text {
        println!("\n{}", "================================================================================".cyan().bold());
        println!("  SERVPULSE REAL-TIME SLA MONITOR: {}", cli.target.yellow().bold());
        println!("  Protocol: {:?} | Port: {} | Probes: {} | Timeout: {}ms", cli.protocol, cli.port, count, cli.timeout);
        println!("{}\n", "================================================================================".cyan().bold());
    }

    let mut results: Vec<ProbeResult> = Vec::with_capacity(count);

    for i in 0..count {
        let res = match cli.protocol {
            ProbeProtocol::Tcp => probe_tcp(&cli.target, cli.port, timeout),
            ProbeProtocol::Http => probe_http(&cli.target, cli.port, &cli.path, timeout),
        };

        if cli.format == OutputFormat::Text {
            let status_badge = if res.success {
                "SUCCESS".green().bold()
            } else {
                "FAILED".red().bold()
            };
            let code_str = match res.status_code {
                Some(code) => format!("[Status: {}]", code),
                None => "[Status: None]".to_string(),
            };
            println!(
                "[{:02}/{:02}] {:<24} - {} - {:>6.2}ms {}",
                i + 1,
                count,
                res.target,
                status_badge,
                res.latency_ms,
                code_str.dimmed()
            );
        }

        results.push(res);

        if i + 1 < count && interval.as_millis() > 0 {
            thread::sleep(interval);
        }
    }

    let report = calculate_sla_report(&cli.target, &results);

    if let Some(ref badge_path) = cli.badge {
        let svg = generate_uptime_badge(&report);
        export_badge_file(badge_path, &svg)?;
        if cli.format == OutputFormat::Text {
            println!("\n  {} Status badge exported to: {}", "✓".green().bold(), badge_path.cyan().bold());
        }
    }

    if cli.format == OutputFormat::Json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_sla_terminal_report(&report);
    }

    Ok(report)
}

pub fn handle_check(
    target: &str,
    protocol: ProbeProtocol,
    port: u16,
    path: &str,
    count: usize,
    interval: u64,
    timeout: u64,
    json: bool,
    badge: Option<&str>,
) -> anyhow::Result<SlaReport> {
    let cli = Cli {
        target: target.to_string(),
        port,
        protocol,
        path: path.to_string(),
        count,
        interval,
        timeout,
        format: if json { OutputFormat::Json } else { OutputFormat::Text },
        badge: badge.map(|s| s.to_string()),
        single: count == 1,
    };
    handle_cli(&cli)
}

pub fn handle_tcp(
    target: &str,
    port: u16,
    count: usize,
    interval: u64,
    timeout: u64,
    json: bool,
    badge: Option<&str>,
) -> anyhow::Result<SlaReport> {
    handle_check(target, ProbeProtocol::Tcp, port, "/", count, interval, timeout, json, badge)
}

pub fn handle_http(
    target: &str,
    port: u16,
    path: &str,
    count: usize,
    interval: u64,
    timeout: u64,
    json: bool,
    badge: Option<&str>,
) -> anyhow::Result<SlaReport> {
    handle_check(target, ProbeProtocol::Http, port, path, count, interval, timeout, json, badge)
}

pub fn handle_badge(
    label: &str,
    uptime: Option<f64>,
    latency: Option<f64>,
    status: Option<&str>,
    color: Option<&str>,
    out: Option<&str>,
) -> anyhow::Result<String> {
    let (val_str, default_color) = if let Some(up) = uptime {
        (format!("{:.1}%", up), crate::badge::svg::badge_color_for_uptime(up))
    } else if let Some(lat) = latency {
        (format!("{:.1}ms", lat), crate::badge::svg::badge_color_for_latency(lat))
    } else if let Some(st) = status {
        (st.to_string(), "#4c1")
    } else {
        ("up".to_string(), "#4c1")
    };

    let chosen_color = color.unwrap_or(default_color);
    let svg = generate_badge(label, &val_str, chosen_color);

    if let Some(path) = out {
        export_badge_file(path, &svg)?;
        println!("{} Badge saved to: {}", "✓".green().bold(), path.cyan().bold());
    } else {
        println!("{}", svg);
    }

    Ok(svg)
}

pub fn parse_target_url(target: &str) -> (String, Option<u16>, String) {
    let mut s = target.trim();
    if let Some(stripped) = s.strip_prefix("http://") {
        s = stripped;
    } else if let Some(stripped) = s.strip_prefix("https://") {
        s = stripped;
    }

    let (host_port, path) = match s.find('/') {
        Some(idx) => (&s[..idx], s[idx..].to_string()),
        None => (s, "/".to_string()),
    };

    let (host, port) = match host_port.find(':') {
        Some(idx) => {
            let host = host_port[..idx].to_string();
            let port = host_port[idx + 1..].parse::<u16>().ok();
            (host, port)
        }
        None => (host_port.to_string(), None),
    };

    (host, port, path)
}

pub fn print_sla_terminal_report(report: &SlaReport) {
    println!("\n{}", "================================================================================".cyan().bold());
    println!("  SERVPULSE REAL-TIME SLA REPORT: {}", report.target.yellow().bold());
    println!("{}\n", "================================================================================".cyan().bold());

    let tier_colored = if report.uptime_percentage >= 99.9 {
        report.sla_tier.green().bold()
    } else if report.uptime_percentage >= 99.0 {
        report.sla_tier.yellow().bold()
    } else {
        report.sla_tier.red().bold()
    };

    let uptime_colored = if report.uptime_percentage >= 99.9 {
        format!("{:.2}%", report.uptime_percentage).green().bold()
    } else if report.uptime_percentage >= 99.0 {
        format!("{:.2}%", report.uptime_percentage).yellow().bold()
    } else {
        format!("{:.2}%", report.uptime_percentage).red().bold()
    };

    println!("  Target                 : {}", report.target.white().bold());
    println!("  Total Probes           : {}", report.total_probes);
    println!("  Successful / Failed    : {} / {}", report.successful_probes.to_string().green().bold(), report.failed_probes.to_string().red().bold());
    println!("  Uptime Percentage      : {}", uptime_colored);
    println!("  SLA Classification     : {}", tier_colored);
    println!("  Latency (Min/Mean/Max) : {:.2}ms / {:.2}ms / {:.2}ms", report.min_latency_ms, report.mean_latency_ms, report.max_latency_ms);
    println!("  Percentiles (p50/90/99): {:.2}ms / {:.2}ms / {:.2}ms", report.p50_latency_ms, report.p90_latency_ms, report.p99_latency_ms);
    println!("  Jitter Variance        : {:.2}ms", report.jitter_ms);
    println!("  Sparkline Trend        : {}", report.sparkline.cyan().bold());
    println!("\n{}\n", "================================================================================".cyan().bold());
}
