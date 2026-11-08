//! ServPulse Comprehensive Test Suite
//! Covers Stats Engine, Real TCP Loopback, Real HTTP Loopback, SVG Badge Generator, and CLI.
//! ZERO MOCK FACADES - All network probes exercise real OS socket loopbacks.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use servpulse::badge::svg::{badge_color_for_uptime, escape_xml, generate_badge, generate_uptime_badge};
use servpulse::cli::{Cli, OutputFormat, ProbeProtocol};
use servpulse::probe::http::probe_http;
use servpulse::probe::tcp::{probe_tcp, ProbeResult};
use servpulse::stats::engine::{calculate_sla_report, generate_sparkline, SlaReport};
use clap::Parser;

// ============================================================================
// 1. STATS ENGINE & SLA ACCURACY TESTS
// ============================================================================

#[test]
fn test_stats_empty_sample_slice() {
    let report = calculate_sla_report("127.0.0.1:8080", &[]);
    assert_eq!(report.target, "127.0.0.1:8080");
    assert_eq!(report.total_probes, 0);
    assert_eq!(report.successful_probes, 0);
    assert_eq!(report.failed_probes, 0);
    assert_eq!(report.uptime_percentage, 0.0);
    assert_eq!(report.sla_tier, "No Data");
    assert_eq!(report.min_latency_ms, 0.0);
    assert_eq!(report.max_latency_ms, 0.0);
    assert_eq!(report.mean_latency_ms, 0.0);
    assert_eq!(report.p50_latency_ms, 0.0);
    assert_eq!(report.p90_latency_ms, 0.0);
    assert_eq!(report.p99_latency_ms, 0.0);
    assert_eq!(report.jitter_ms, 0.0);
    assert_eq!(report.sparkline, "");
}

#[test]
fn test_stats_all_failed_probes() {
    let results = vec![
        ProbeResult {
            target: "127.0.0.1:9000".to_string(),
            protocol: "TCP".to_string(),
            success: false,
            latency_ms: 0.0,
            status_code: None,
            error_message: Some("Connection refused".to_string()),
        },
        ProbeResult {
            target: "127.0.0.1:9000".to_string(),
            protocol: "TCP".to_string(),
            success: false,
            latency_ms: 0.0,
            status_code: None,
            error_message: Some("Connection refused".to_string()),
        },
    ];

    let report = calculate_sla_report("127.0.0.1:9000", &results);
    assert_eq!(report.total_probes, 2);
    assert_eq!(report.successful_probes, 0);
    assert_eq!(report.failed_probes, 2);
    assert_eq!(report.uptime_percentage, 0.0);
    assert_eq!(report.sla_tier, "Offline / Failed");
    assert_eq!(report.sparkline, "████████");
}

#[test]
fn test_stats_single_successful_probe() {
    let results = vec![ProbeResult {
        target: "127.0.0.1:80".to_string(),
        protocol: "TCP".to_string(),
        success: true,
        latency_ms: 12.34,
        status_code: Some(200),
        error_message: None,
    }];

    let report = calculate_sla_report("127.0.0.1:80", &results);
    assert_eq!(report.total_probes, 1);
    assert_eq!(report.successful_probes, 1);
    assert_eq!(report.failed_probes, 0);
    assert_eq!(report.uptime_percentage, 100.0);
    assert_eq!(report.min_latency_ms, 12.34);
    assert_eq!(report.max_latency_ms, 12.34);
    assert_eq!(report.mean_latency_ms, 12.34);
    assert_eq!(report.p50_latency_ms, 12.34);
    assert_eq!(report.jitter_ms, 0.0);
}

#[test]
fn test_stats_percentiles_accuracy() {
    // 10 ordered probes from 10.0ms to 100.0ms
    let results: Vec<ProbeResult> = (1..=10)
        .map(|i| ProbeResult {
            target: "127.0.0.1".to_string(),
            protocol: "TCP".to_string(),
            success: true,
            latency_ms: i as f64 * 10.0,
            status_code: Some(200),
            error_message: None,
        })
        .collect();

    let report = calculate_sla_report("127.0.0.1", &results);
    assert_eq!(report.min_latency_ms, 10.0);
    assert_eq!(report.max_latency_ms, 100.0);
    assert_eq!(report.mean_latency_ms, 55.0);
    assert_eq!(report.p50_latency_ms, 60.0);
    assert_eq!(report.p90_latency_ms, 90.0);
    assert_eq!(report.p99_latency_ms, 100.0);
}

#[test]
fn test_stats_jitter_window_calculation() {
    let latencies = [10.0, 15.0, 12.0, 20.0];
    let results: Vec<ProbeResult> = latencies
        .iter()
        .map(|&lat| ProbeResult {
            target: "127.0.0.1".to_string(),
            protocol: "TCP".to_string(),
            success: true,
            latency_ms: lat,
            status_code: Some(200),
            error_message: None,
        })
        .collect();

    let report = calculate_sla_report("127.0.0.1", &results);
    // Differences: |15-10|=5, |12-15|=3, |20-12|=8. Sum = 16. Jitter = 16 / 3 = 5.33
    assert!((report.jitter_ms - 5.33).abs() < 0.01);
}

#[test]
fn test_stats_sla_tier_thresholds() {
    // Helper to generate N total with M success
    let make_probes = |total: usize, success: usize| -> Vec<ProbeResult> {
        (0..total)
            .map(|i| ProbeResult {
                target: "srv".to_string(),
                protocol: "TCP".to_string(),
                success: i < success,
                latency_ms: 10.0,
                status_code: if i < success { Some(200) } else { None },
                error_message: None,
            })
            .collect()
    };

    let p100 = make_probes(1000, 1000);
    assert_eq!(calculate_sla_report("s", &p100).sla_tier, "Four Nines (99.99% Enterprise HA)");

    let p999 = make_probes(1000, 999);
    assert_eq!(calculate_sla_report("s", &p999).sla_tier, "Three Nines (99.9% Production)");

    let p992 = make_probes(1000, 992);
    assert_eq!(calculate_sla_report("s", &p992).sla_tier, "Two Nines (99.0% Lab/Staging)");

    let p950 = make_probes(1000, 950);
    assert_eq!(calculate_sla_report("s", &p950).sla_tier, "SLA Degraded (< 99.0%)");
}

#[test]
fn test_stats_sparkline_distribution() {
    assert_eq!(generate_sparkline(&[]), "");
    assert_eq!(generate_sparkline(&[10.0, 10.0, 10.0]), "   ");

    let ascending = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    let spark = generate_sparkline(&ascending);
    assert_eq!(spark, " ▂▃▄▅▆▇█");
}

#[test]
fn test_stats_sla_report_serde_json() {
    let report = SlaReport {
        target: "10.0.0.1:80".to_string(),
        total_probes: 10,
        successful_probes: 10,
        failed_probes: 0,
        uptime_percentage: 100.0,
        sla_tier: "Four Nines".to_string(),
        min_latency_ms: 1.2,
        max_latency_ms: 4.5,
        mean_latency_ms: 2.8,
        p50_latency_ms: 2.5,
        p90_latency_ms: 4.1,
        p99_latency_ms: 4.5,
        jitter_ms: 0.3,
        sparkline: " ▄█".to_string(),
    };

    let json_str = serde_json::to_string(&report).expect("Serialize SlaReport");
    assert!(json_str.contains("\"uptime_percentage\":100.0"));
    assert!(json_str.contains("\"target\":\"10.0.0.1:80\""));

    let deserialized: SlaReport = serde_json::from_str(&json_str).expect("Deserialize SlaReport");
    assert_eq!(deserialized.total_probes, 10);
    assert_eq!(deserialized.mean_latency_ms, 2.8);
}

// ============================================================================
// 2. REAL TCP LOOPBACK PROBE TESTS (ZERO MOCK FACADES)
// ============================================================================

#[test]
fn test_tcp_probe_loopback_success() {
    // Bind to ephemeral port chosen by OS
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind TCP loopback");
    let port = listener.local_addr().expect("Local addr").port();

    // Spawn thread to accept single connection
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("Accept TCP connection");
        drop(stream);
    });

    let res = probe_tcp("127.0.0.1", port, Duration::from_secs(2));
    assert!(res.success);
    assert_eq!(res.protocol, "TCP");
    assert_eq!(res.target, format!("127.0.0.1:{}", port));
    assert_eq!(res.status_code, Some(200));
    assert!(res.error_message.is_none());
    assert!(res.latency_ms >= 0.0);

    let _ = handle.join();
}

#[test]
fn test_tcp_probe_connection_refused() {
    // Bind and immediately drop listener to guarantee closed port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind ephemeral port");
    let closed_port = listener.local_addr().unwrap().port();
    drop(listener);

    let res = probe_tcp("127.0.0.1", closed_port, Duration::from_millis(500));
    assert!(!res.success);
    assert_eq!(res.protocol, "TCP");
    assert_eq!(res.status_code, None);
    assert!(res.error_message.is_some());
}

#[test]
fn test_tcp_probe_unresolvable_hostname() {
    let res = probe_tcp("invalid.strictly.local.nonexistent.domain", 80, Duration::from_millis(500));
    assert!(!res.success);
    assert_eq!(res.status_code, None);
    let err = res.error_message.expect("Error message expected");
    assert!(err.contains("DNS resolution failed"));
}

#[test]
fn test_tcp_probe_result_serialization() {
    let result = ProbeResult {
        target: "127.0.0.1:8080".to_string(),
        protocol: "TCP".to_string(),
        success: true,
        latency_ms: 15.42,
        status_code: Some(200),
        error_message: None,
    };

    let json = serde_json::to_string(&result).expect("Serialize ProbeResult");
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"protocol\":\"TCP\""));
    assert!(json.contains("\"latency_ms\":15.42"));
}

// ============================================================================
// 3. REAL HTTP RAW SOCKET LOOPBACK PROBE TESTS
// ============================================================================

#[test]
fn test_http_probe_200_ok_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind HTTP loopback");
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept HTTP client");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
        let _ = stream.write_all(resp.as_bytes());
    });

    let res = probe_http("127.0.0.1", port, "/healthz", Duration::from_secs(2));
    assert!(res.success);
    assert_eq!(res.protocol, "HTTP");
    assert_eq!(res.status_code, Some(200));
    assert!(res.error_message.is_none());
    assert!(res.target.contains("/healthz"));

    let _ = handle.join();
}

#[test]
fn test_http_probe_404_not_found_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind HTTP loopback");
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept client");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found";
        let _ = stream.write_all(resp.as_bytes());
    });

    let res = probe_http("127.0.0.1", port, "/missing", Duration::from_secs(2));
    assert!(!res.success);
    assert_eq!(res.status_code, Some(404));
    assert!(res.error_message.expect("Error msg").contains("404"));

    let _ = handle.join();
}

#[test]
fn test_http_probe_500_server_error_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind HTTP loopback");
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept client");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\nConnection: close\r\n\r\nError";
        let _ = stream.write_all(resp.as_bytes());
    });

    let res = probe_http("127.0.0.1", port, "/crash", Duration::from_secs(2));
    assert!(!res.success);
    assert_eq!(res.status_code, Some(500));
    assert!(res.error_message.expect("Error msg").contains("500"));

    let _ = handle.join();
}

#[test]
fn test_http_probe_302_redirect_loopback() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind HTTP loopback");
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept client");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = "HTTP/1.1 302 Found\r\nLocation: /new-path\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
    });

    let res = probe_http("127.0.0.1", port, "/old-path", Duration::from_secs(2));
    assert!(res.success); // 200..=399 is considered success in probe_http
    assert_eq!(res.status_code, Some(302));
    assert!(res.error_message.is_none());

    let _ = handle.join();
}

#[test]
fn test_http_probe_malformed_response() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind HTTP loopback");
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept client");
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = "RAW_GARBAGE_WITHOUT_HTTP_PREFIX\r\n";
        let _ = stream.write_all(resp.as_bytes());
    });

    let res = probe_http("127.0.0.1", port, "/corrupt", Duration::from_secs(2));
    assert!(!res.success);
    assert_eq!(res.status_code, None);
    assert_eq!(res.error_message, Some("HTTP Status: None".to_string()));

    let _ = handle.join();
}

#[test]
fn test_http_probe_request_headers_sent() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Bind HTTP loopback");
    let port = listener.local_addr().unwrap().port();

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("Accept client");
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).unwrap();
        let received = String::from_utf8_lossy(&buf[..n]).to_string();
        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        received
    });

    let _ = probe_http("127.0.0.1", port, "/api/v1/ping", Duration::from_secs(2));
    let raw_request = handle.join().expect("Join thread");

    assert!(raw_request.contains("GET /api/v1/ping HTTP/1.1"));
    assert!(raw_request.contains("Host: 127.0.0.1"));
    assert!(raw_request.contains("User-Agent: ServPulse-SLA/0.1.0"));
    assert!(raw_request.contains("Connection: close"));
}

// ============================================================================
// 4. SVG BADGE GENERATOR TESTS
// ============================================================================

#[test]
fn test_svg_badge_valid_xml() {
    let svg = generate_badge("uptime", "99.95%", "#4c1");
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("uptime"));
    assert!(svg.contains("99.95%"));
    assert!(svg.contains("#4c1"));
}

#[test]
fn test_svg_badge_color_mapping() {
    assert_eq!(badge_color_for_uptime(100.0), "#4c1");
    assert_eq!(badge_color_for_uptime(99.99), "#4c1");
    assert_eq!(badge_color_for_uptime(99.90), "#4c1");
    assert_eq!(badge_color_for_uptime(99.50), "#dfb317");
    assert_eq!(badge_color_for_uptime(99.00), "#dfb317");
    assert_eq!(badge_color_for_uptime(98.99), "#e05d44");
    assert_eq!(badge_color_for_uptime(0.0), "#e05d44");
}

#[test]
fn test_svg_xml_escaping() {
    let raw = "Malicious <script>alert(\"XSS & test's\")</script>";
    let escaped = escape_xml(raw);
    assert_eq!(
        escaped,
        "Malicious &lt;script&gt;alert(&quot;XSS &amp; test&apos;s&quot;)&lt;/script&gt;"
    );

    let badge = generate_badge("tag<1>", "val&2", "#4c1");
    assert!(!badge.contains("tag<1>"));
    assert!(badge.contains("tag&lt;1&gt;"));
    assert!(badge.contains("val&amp;2"));
}

#[test]
fn test_svg_badge_from_sla_report() {
    let report = SlaReport {
        target: "srv.local".to_string(),
        total_probes: 100,
        successful_probes: 99,
        failed_probes: 1,
        uptime_percentage: 99.0,
        sla_tier: "Two Nines".to_string(),
        min_latency_ms: 10.0,
        max_latency_ms: 50.0,
        mean_latency_ms: 20.0,
        p50_latency_ms: 18.0,
        p90_latency_ms: 35.0,
        p99_latency_ms: 48.0,
        jitter_ms: 2.1,
        sparkline: " ▄█".to_string(),
    };

    let badge = generate_uptime_badge(&report);
    assert!(badge.contains("<svg"));
    assert!(badge.contains("uptime"));
    assert!(badge.contains("99%"));
    assert!(badge.contains("#dfb317"));
}

// ============================================================================
// 5. CLI PARSING & INTEGRATION TESTS
// ============================================================================

#[test]
fn test_cli_default_parsing() {
    let args = vec!["servpulse", "--target", "127.0.0.1"];
    let cli = Cli::try_parse_from(args).expect("Default parse");
    assert_eq!(cli.target, "127.0.0.1");
    assert_eq!(cli.port, 80);
    assert_eq!(cli.protocol, ProbeProtocol::Tcp);
    assert_eq!(cli.path, "/");
    assert_eq!(cli.count, 5);
    assert_eq!(cli.interval, 500);
    assert_eq!(cli.timeout, 2000);
    assert_eq!(cli.format, OutputFormat::Text);
    assert_eq!(cli.badge, None);
    assert_eq!(cli.single, false);
}

#[test]
fn test_cli_all_custom_options() {
    let args = vec![
        "servpulse",
        "--target", "api.internal.net",
        "--port", "8443",
        "--protocol", "http",
        "--path", "/status",
        "-c", "20",
        "-i", "100",
        "-t", "1500",
        "-f", "json",
        "-b", "status_badge.svg",
        "--single",
    ];
    let cli = Cli::try_parse_from(args).expect("Custom flags parse");
    assert_eq!(cli.target, "api.internal.net");
    assert_eq!(cli.port, 8443);
    assert_eq!(cli.protocol, ProbeProtocol::Http);
    assert_eq!(cli.path, "/status");
    assert_eq!(cli.count, 20);
    assert_eq!(cli.interval, 100);
    assert_eq!(cli.timeout, 1500);
    assert_eq!(cli.format, OutputFormat::Json);
    assert_eq!(cli.badge, Some("status_badge.svg".to_string()));
    assert_eq!(cli.single, true);
}

#[test]
fn test_cli_missing_target_error() {
    let args = vec!["servpulse"];
    let result = Cli::try_parse_from(args);
    assert!(result.is_err());
}

#[test]
fn test_cli_invalid_port_error() {
    let args = vec!["servpulse", "--target", "127.0.0.1", "--port", "99999"];
    let result = Cli::try_parse_from(args);
    assert!(result.is_err());
}
