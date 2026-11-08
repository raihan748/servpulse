pub mod probe;
pub mod stats;
pub mod badge;
pub mod cli;

pub use probe::tcp::{probe_tcp, ProbeResult};
pub use probe::http::probe_http;
pub use stats::engine::{calculate_sla_report, generate_sparkline, SlaReport};
pub use badge::svg::{
    badge_color_for_latency,
    badge_color_for_uptime,
    escape_xml,
    export_badge_file,
    generate_badge,
    generate_sla_badge,
    generate_uptime_badge,
    generate_uptime_badge_val,
};
pub use cli::{Cli, Commands, OutputFormat, ProbeProtocol, ProtocolArg};
