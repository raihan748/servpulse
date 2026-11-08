use crate::stats::engine::SlaReport;

/// Escapes XML special characters to prevent SVG XML injection.
pub fn escape_xml(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Returns standard hex color for SLA uptime threshold.
pub fn badge_color_for_uptime(uptime: f64) -> &'static str {
    if uptime >= 99.90 {
        "#4c1" // High Availability / Production Green
    } else if uptime >= 99.00 {
        "#dfb317" // Warning / Degraded Yellow
    } else {
        "#e05d44" // Outage / Critical Red
    }
}

/// Returns standard hex color based on network latency in milliseconds.
pub fn badge_color_for_latency(latency_ms: f64) -> &'static str {
    if latency_ms < 50.0 {
        "#4c1" // Fast (< 50ms)
    } else if latency_ms < 200.0 {
        "#dfb317" // Moderate (50ms - 200ms)
    } else {
        "#e05d44" // Slow (>= 200ms)
    }
}

/// Generates a Shields.io compliant, standalone SVG status badge.
pub fn generate_badge(label: &str, value: &str, color: &str) -> String {
    let escaped_label = escape_xml(label);
    let escaped_value = escape_xml(value);

    let label_len = label.chars().count();
    let value_len = value.chars().count();

    let label_width = (label_len * 7 + 14).max(40);
    let value_width = (value_len * 7 + 14).max(40);
    let total_width = label_width + value_width;

    let label_center_x = (label_width * 10) / 2;
    let value_center_x = label_width * 10 + (value_width * 10) / 2;

    let label_text_len = (label_len * 70).max(300);
    let value_text_len = (value_len * 70).max(300);

    format!(
r##"<svg xmlns="http://www.w3.org/2000/svg" width="{total_width}" height="20" role="img" aria-label="{escaped_label}: {escaped_value}">
  <title>{escaped_label}: {escaped_value}</title>
  <linearGradient id="s" x2="0" y2="100%">
    <stop offset="0" stop-color="#bbb" stop-opacity=".1"/>
    <stop offset="1" stop-opacity=".1"/>
  </linearGradient>
  <clipPath id="r">
    <rect width="{total_width}" height="20" rx="3" fill="#fff"/>
  </clipPath>
  <g clip-path="url(#r)">
    <rect width="{label_width}" height="20" fill="#555"/>
    <rect x="{label_width}" width="{value_width}" height="20" fill="{color}"/>
    <rect width="{total_width}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="110">
    <text aria-hidden="true" x="{label_center_x}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{label_text_len}">{escaped_label}</text>
    <text x="{label_center_x}" y="140" transform="scale(.1)" fill="#fff" textLength="{label_text_len}">{escaped_label}</text>
    <text aria-hidden="true" x="{value_center_x}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{value_text_len}">{escaped_value}</text>
    <text x="{value_center_x}" y="140" transform="scale(.1)" fill="#fff" textLength="{value_text_len}">{escaped_value}</text>
  </g>
</svg>"##
    )
}

/// Generates an uptime-focused SVG badge directly from an SlaReport.
pub fn generate_uptime_badge(report: &SlaReport) -> String {
    let color = badge_color_for_uptime(report.uptime_percentage);
    let val_str = if report.uptime_percentage.fract() == 0.0 {
        format!("{:.0}%", report.uptime_percentage)
    } else {
        let formatted = format!("{:.2}%", report.uptime_percentage);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        format!("{}%", trimmed.trim_end_matches('%'))
    };
    generate_badge("uptime", &val_str, color)
}

/// Generates an uptime-focused SVG badge from a raw float percentage.
pub fn generate_uptime_badge_val(uptime: f64) -> String {
    let color = badge_color_for_uptime(uptime);
    let val_str = if uptime.fract() == 0.0 {
        format!("{:.0}%", uptime)
    } else {
        let formatted = format!("{:.2}%", uptime);
        let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
        format!("{}%", trimmed.trim_end_matches('%'))
    };
    generate_badge("uptime", &val_str, color)
}

/// Formats an SlaReport into a comprehensive SLA status badge.
pub fn generate_sla_badge(report: &SlaReport) -> String {
    let color = badge_color_for_uptime(report.uptime_percentage);
    let val_str = format!("{:.1}% ({:.1}ms)", report.uptime_percentage, report.mean_latency_ms);
    generate_badge("servpulse", &val_str, color)
}

/// Exports SVG content to a file.
pub fn export_badge_file(path: &str, svg_content: &str) -> anyhow::Result<()> {
    std::fs::write(path, svg_content)
        .map_err(|e| anyhow::anyhow!("Failed to export SVG badge to '{}': {}", path, e))
}
