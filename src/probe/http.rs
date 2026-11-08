use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use crate::probe::tcp::ProbeResult;

pub fn probe_http(host: &str, port: u16, path: &str, timeout: Duration) -> ProbeResult {
    let addr_str = format!("{}:{}", host, port);
    let start = Instant::now();

    let resolved = match addr_str.to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(e) => {
            return ProbeResult {
                target: format!("http://{}:{}{}", host, port, path),
                protocol: "HTTP".to_string(),
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
                target: format!("http://{}:{}{}", host, port, path),
                protocol: "HTTP".to_string(),
                success: false,
                latency_ms: 0.0,
                status_code: None,
                error_message: Some("Could not resolve address".to_string()),
            };
        }
    };

    let mut stream = match TcpStream::connect_timeout(&socket_addr, timeout) {
        Ok(s) => s,
        Err(e) => {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return ProbeResult {
                target: format!("http://{}:{}{}", host, port, path),
                protocol: "HTTP".to_string(),
                success: false,
                latency_ms: (latency_ms * 100.0).round() / 100.0,
                status_code: None,
                error_message: Some(format!("Connection refused: {}", e)),
            };
        }
    };

    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: ServPulse-SLA/0.1.0\r\nConnection: close\r\n\r\n",
        path, host
    );

    if let Err(e) = stream.write_all(request.as_bytes()) {
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        return ProbeResult {
            target: format!("http://{}:{}{}", host, port, path),
            protocol: "HTTP".to_string(),
            success: false,
            latency_ms: (latency_ms * 100.0).round() / 100.0,
            status_code: None,
            error_message: Some(format!("Write failed: {}", e)),
        };
    }

    let mut buf = [0u8; 512];
    let bytes_read = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            return ProbeResult {
                target: format!("http://{}:{}{}", host, port, path),
                protocol: "HTTP".to_string(),
                success: false,
                latency_ms: (latency_ms * 100.0).round() / 100.0,
                status_code: None,
                error_message: Some(format!("Read failed: {}", e)),
            };
        }
    };

    let elapsed = start.elapsed();
    let latency_ms = elapsed.as_secs_f64() * 1000.0;

    let response_text = String::from_utf8_lossy(&buf[..bytes_read]);
    let status_code = parse_http_status(&response_text);

    let success = match status_code {
        Some(code) => (200..=399).contains(&code),
        None => false,
    };

    ProbeResult {
        target: format!("http://{}:{}{}", host, port, path),
        protocol: "HTTP".to_string(),
        success,
        latency_ms: (latency_ms * 100.0).round() / 100.0,
        status_code,
        error_message: if success { None } else { Some(format!("HTTP Status: {:?}", status_code)) },
    }
}

fn parse_http_status(response: &str) -> Option<u16> {
    let first_line = response.lines().next()?;
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().ok()
    } else {
        None
    }
}
