use clap::Parser;
use servpulse::cli::{handle_cli, Cli};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    handle_cli(&cli)?;
    Ok(())
}
