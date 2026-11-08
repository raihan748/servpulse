use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub target: String,
    pub protocol: String,
    pub success: bool,
    pub latency_ms: f64,
    pub status_code: Option<u16>,
    pub error_message: Option<String>,
}

pub fn probe_tcp(target: &str, port: u16, timeout: Duration) -> ProbeResult {
    let addr_str = format!("{}:{}", target, port);
    let start = Instant::now();

    let resolved = match addr_str.to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(e) => {
            return ProbeResult {
                target: target.to_string(),
                protocol: "TCP".to_string(),
                success: false,
                latency_ms: 0.0,
                status_code: None,
                error_message: Some(format!("DNS resolution failed: {}", e)),
            };
        }
    };

    let socket_addr = match resolved {
        Some(addr) => addr,
        None => {
            return ProbeResult {
                target: target.to_string(),
                protocol: "TCP".to_string(),
                success: false,
                latency_ms: 0.0,
                status_code: None,
                error_message: Some("Could not resolve target address".to_string()),
            };
        }
    };

    match TcpStream::connect_timeout(&socket_addr, timeout) {
        Ok(_) => {
            let elapsed = start.elapsed();
            let latency_ms = elapsed.as_secs_f64() * 1000.0;
            ProbeResult {
                target: format!("{}:{}", target, port),
                protocol: "TCP".to_string(),
                success: true,
                latency_ms: (latency_ms * 100.0).round() / 100.0,
                status_code: Some(200),
                error_message: None,
            }
        }
        Err(e) => {
            let elapsed = start.elapsed();
            let latency_ms = elapsed.as_secs_f64() * 1000.0;
            ProbeResult {
                target: format!("{}:{}", target, port),
                protocol: "TCP".to_string(),
                success: false,
                latency_ms: (latency_ms * 100.0).round() / 100.0,
                status_code: None,
                error_message: Some(e.to_string()),
            }
        }
    }
}
