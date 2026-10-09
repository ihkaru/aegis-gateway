// SPDX-License-Identifier: MIT

use clap::{Args, Parser, Subcommand};

/// Aegis Gateway: Enterprise Zero-Trust MCP & Skill Control Plane
#[derive(Debug, Parser)]
#[command(
    name = "aegis-gateway",
    author = "Aegis Gateway Contributors",
    version = "0.9.0",
    about = "Enterprise Zero-Trust MCP & Skill Gateway with Distributed State, Granular ABAC, Real-Time DLP, and SIEM Audit",
    long_about = None
)]
pub struct Cli {
    /// Global path to topology configuration file (YAML or JSON)
    #[arg(short, long, global = true, env = "AEGIS_CONFIG")]
    pub config: Option<String>,

    /// Run directly in Stdio mode for Claude Desktop / Cursor (shorthand for `serve --stdio`)
    #[arg(long, global = true)]
    pub stdio: bool,

    /// Subcommand to execute (defaults to Serve if none specified)
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the gateway server daemon (default)
    Serve(ServeArgs),

    /// Initialize a starter aegis.yaml topology configuration file
    Init(InitArgs),

    /// Validate configuration file syntax and backend binary paths
    Validate(ValidateArgs),

    /// Add an MCP backend server to topology configuration
    Add(AddArgs),

    /// Remove an MCP backend server from topology configuration
    Remove(RemoveArgs),

    /// List configured MCP backend servers
    List(ListArgs),

    /// Run preflight diagnostics on host environment dependencies
    Doctor(DoctorArgs),

    /// Verify cryptographic hash-chain audit log integrity (SOC 2 Type II report)
    Audit(AuditArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ServeArgs {
    /// Run over stdin/stdout for local AI agent desktop clients (Claude, Cursor)
    #[arg(long, default_value_t = false)]
    pub stdio: bool,

    /// Bind host for Streamable HTTP server
    #[arg(long, default_value = "0.0.0.0", env = "AEGIS_HOST")]
    pub host: String,

    /// Port for Streamable HTTP server
    #[arg(short, long, default_value_t = 39400, env = "AEGIS_PORT")]
    pub port: u16,

    /// Path to topology configuration file
    #[arg(short, long)]
    pub config: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    /// Target path for generated configuration file
    #[arg(short, long, default_value = "aegis.yaml")]
    pub output: String,

    /// Overwrite existing configuration file if present
    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ValidateArgs {
    /// Path to configuration file to validate
    #[arg(short, long, default_value = "aegis.yaml")]
    pub config: String,
}

#[derive(Debug, Args, Clone)]
pub struct AddArgs {
    /// Unique name of the backend server
    pub name: String,

    /// Executable command (e.g. "npx", "uvx", "node")
    #[arg(short, long)]
    pub command: Option<String>,

    /// Command arguments (e.g. "-y", "@modelcontextprotocol/server-postgres")
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,

    /// Remote Streamable HTTP / SSE endpoint URL
    #[arg(short, long)]
    pub url: Option<String>,

    /// Environment variables in KEY=VALUE format
    #[arg(short, long)]
    pub env: Vec<String>,

    /// Target topology configuration file to update
    #[arg(short, long, default_value = "aegis.yaml")]
    pub config: String,
}

#[derive(Debug, Args, Clone)]
pub struct RemoveArgs {
    /// Name of the backend server to remove
    pub name: String,

    /// Target topology configuration file to update
    #[arg(short, long, default_value = "aegis.yaml")]
    pub config: String,
}

#[derive(Debug, Args, Clone, Default)]
pub struct ListArgs {
    /// Path to configuration file to inspect
    #[arg(short, long, default_value = "aegis.yaml")]
    pub config: String,

    /// Format output as JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Args, Clone, Default)]
pub struct DoctorArgs {
    /// Output results in JSON format
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Args, Clone, Default)]
pub struct AuditArgs {
    /// Export report as JSON file
    #[arg(short, long)]
    pub output: Option<String>,
}
