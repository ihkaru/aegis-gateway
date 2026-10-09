// SPDX-License-Identifier: MIT
#![deny(unsafe_code)]

use clap::Parser;
use aegis_gateway::cli::{self, Cli};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // In Stdio mode or HTTP mode, write all diagnostic logs exclusively to stderr
    // to strictly preserve zero-contamination stdout channel for JSON-RPC 2.0 framing.
    if cli.stdio || matches!(&cli.command, Some(cli::Command::Serve(args)) if args.stdio) {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter("warn")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter("info")
            .init();
    }

    if let Err(e) = cli::run(cli).await {
        eprintln!("[AEGIS] Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
