# servpulse 💓⚡

> **Real-Time Service Heartbeat, Multi-Protocol Probing (TCP/HTTP), Latency Percentiles & SVG Status Badge Generator.**  
> *Bridging Computer & Network Engineering (TKJ) and Software Engineering (RPL).*

[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-orange?style=flat-square)](https://crates.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust%201.85-black?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/Tests-Passed-brightgreen?style=flat-square)]()

---

## 🌟 Overview

`servpulse` is a high-performance network service health monitor and SLA telemetry CLI tool written in pure Rust. Built for NetDevOps engineers, sysadmins, and microservice developers, it delivers real-time latency probing across TCP sockets and HTTP endpoints without cumbersome external daemon setups.

Beyond simple up/down checks, `servpulse` calculates true production SLA statistics: **percentiles ($p_{50}, p_{90}, p_{99}$)**, **consecutive jitter variance**, **SLA tier classifications (Four Nines 99.99%, Three Nines 99.9%, Two Nines 99.0%)**, and dynamically generates embeddable SVG status badges ready for documentation or web dashboards.

```
  ____                  ____        _          
 / ___|  ___ _ ____   _|  _ \ _   _| |___  ___ 
 \___ \ / _ \ '__\ \ / / |_) | | | | / __|/ _ \
  ___) |  __/ |   \ V /|  __/| |_| | \__ \  __/
 |____/ \___|_|    \_/ |_|    \__,_|_|___/\___|
    Real-Time SLA Heartbeat & SVG Badge Generator
```

---

## ✨ Features

- **Multi-Protocol Service Probing**:
  - **TCP Socket Probing**: Fast handshake latency measurement and socket validation for DNS, databases (Postgres, MySQL), Redis, and SSH.
  - **HTTP/HTTPS Probing**: Direct HTTP request dispatching, status code classification (2xx/3xx/4xx/5xx), and time-to-first-byte evaluation.
- **Enterprise SLA Analytics**:
  - Computes uptime percentage and maps directly to industry SLA tiers.
  - Latency distribution: min, max, mean, $p_{50}$, $p_{90}$, and $p_{99}$ percentiles.
  - Packet-to-packet jitter calculation according to RFC-standard delta differences.
  - Visual terminal ASCII sparklines (` ▂▃▅▆▇█`).
- **Dynamic SVG Status Badge Generator**:
  - Generates Shields.io compatible, vector SVG badges with custom labels, status values, and hex color codes.
  - Perfect for embedding directly in GitHub READMEs, internal portals, or CI/CD pipelines.
- **Flexible CLI & Automation**:
  - One-shot check mode for quick verification.
  - Loop monitor mode with configurable intervals and sample counts.
  - JSON output stream (`--format json`) for machine ingestion into Prometheus, Grafana, or SIEM pipelines.

---

## 🚀 Installation & Usage

### 1. Build from Source
```bash
git clone https://github.com/raihan748/servpulse.git
cd servpulse
cargo build --release
```

### 2. Service Probing & Monitoring
Check a TCP service (e.g. Cloudflare DNS on port 53):
```bash
servpulse --target 1.1.1.1 --port 53
```

Check an HTTP service:
```bash
servpulse --target example.com --protocol http --path /
```

Continuous SLA monitoring over 10 iterations:
```bash
servpulse --target 127.0.0.1 --port 8080 -c 10 -i 200
```

**Output Display:**
```
================================================================================
  SERVPULSE REAL-TIME SLA MONITOR: 127.0.0.1
  Protocol: Tcp | Port: 8080 | Probes: 10 | Timeout: 2000ms
================================================================================
[01/10] 127.0.0.1:8080           - SUCCESS -   0.42ms [Status: 200]
[02/10] 127.0.0.1:8080           - SUCCESS -   0.38ms [Status: 200]
...
================================================================================
  SERVPULSE REAL-TIME SLA REPORT: 127.0.0.1
================================================================================
  Target                 : 127.0.0.1
  Total Probes           : 10
  Successful / Failed    : 10 / 0
  Uptime Percentage      : 100.00%
  SLA Classification     : Four Nines (99.99% Enterprise HA)
  Latency (Min/Mean/Max) : 0.35ms / 0.41ms / 0.58ms
  Percentiles (p50/90/99): 0.40ms / 0.51ms / 0.58ms
  Jitter Variance        : 0.04ms
  Sparkline Trend        :  ▃▂▅▃ █▂ 
================================================================================
```

### 3. Dynamic SVG Badge Export
Generate a status badge for your repository or service dashboard:
```bash
servpulse --target 127.0.0.1 --port 80 --badge status.svg
```

---

## 🧪 Testing

Run the comprehensive test suite (26 real network loopback and engine tests):
```bash
cargo test
```

---

## 📄 License

Licensed under the [MIT License](LICENSE).  
Created with pride by [Raihan](https://github.com/raihan748) (SMK Telkom Makassar).
