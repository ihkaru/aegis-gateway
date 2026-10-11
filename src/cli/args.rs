// SPDX-License-Identifier: MIT

use clap::{Args, Parser, Subcommand};

/// Propylon Gateway: Enterprise Zero-Trust MCP & Skill Control Plane
#[derive(Debug, Parser)]
#[command(
    name = "propylon",
    author = "Propylon Gateway Contributors",
    version = "0.9.0",
    about = "Enterprise Zero-Trust MCP & Skill Gateway with Distributed State, Granular ABAC, Real-Time DLP, and SIEM Audit",
    long_about = None
)]
pub struct Cli {
    /// Global path to topology configuration file (YAML or JSON)
    #[arg(short, long, global = true, env = "PROPYLON_CONFIG")]
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

    /// Initialize a starter propylon.yaml topology configuration file
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

    /// Start the embedded administrative Web UI control plane
    Ui(UiArgs),

    /// Authenticate via SAML SSO and cache enterprise session token
    Login(LoginArgs),

    /// Run local stdio bridge to remote Propylon Gateway for Claude Code & Antigravity IDE
    Bridge(BridgeArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ServeArgs {
    /// Run over stdin/stdout for local AI agent desktop clients (Claude, Cursor)
    #[arg(long, default_value_t = false)]
    pub stdio: bool,

    /// Bind host for Streamable HTTP server
    #[arg(long, default_value = "0.0.0.0", env = "PROPYLON_HOST")]
    pub host: String,

    /// Port for Streamable HTTP server
    #[arg(short, long, default_value_t = 39400, env = "PROPYLON_PORT")]
    pub port: u16,

    /// Path to topology configuration file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Disable embedded Admin Web UI (headless mode for minimal memory footprint)
    #[arg(long, default_value_t = false, env = "PROPYLON_NO_UI")]
    pub no_ui: bool,

    /// Enforce GitOps immutable mode (prohibit dynamic REST API mutations)
    #[arg(long, default_value_t = false, env = "PROPYLON_IMMUTABLE_CONFIG")]
    pub immutable_config: bool,

    /// Enable lean memory profile (skip UI event history buffers)
    #[arg(long, default_value_t = false, env = "PROPYLON_LEAN_PROFILE")]
    pub lean: bool,
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    /// Target path for generated configuration file
    #[arg(short, long, default_value = "propylon.yaml")]
    pub output: String,

    /// Overwrite existing configuration file if present
    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ValidateArgs {
    /// Path to configuration file to validate
    #[arg(short, long, default_value = "propylon.yaml")]
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
    #[arg(short, long, default_value = "propylon.yaml")]
    pub config: String,
}

#[derive(Debug, Args, Clone)]
pub struct RemoveArgs {
    /// Name of the backend server to remove
    pub name: String,

    /// Target topology configuration file to update
    #[arg(short, long, default_value = "propylon.yaml")]
    pub config: String,
}

#[derive(Debug, Args, Clone, Default)]
pub struct ListArgs {
    /// Path to configuration file to inspect
    #[arg(short, long, default_value = "propylon.yaml")]
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

#[derive(Debug, Args, Clone)]
pub struct UiArgs {
    /// Bind host for Admin Web UI server
    #[arg(long, default_value = "0.0.0.0", env = "PROPYLON_UI_HOST")]
    pub host: String,

    /// Port for Admin Web UI server
    #[arg(short, long, default_value_t = 8485, env = "PROPYLON_UI_PORT")]
    pub port: u16,

    /// Path to topology configuration file (YAML/JSON)
    #[arg(short, long, env = "PROPYLON_CONFIG")]
    pub config: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct LoginArgs {
    /// Remote Propylon Gateway endpoint URL (e.g. https://mcp.dvlpid.my.id)
    #[arg(short, long, default_value = "https://mcp.dvlpid.my.id")]
    pub remote: String,

    /// Direct manual token input (skips browser SSO redirect)
    #[arg(short, long)]
    pub token: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct BridgeArgs {
    /// Remote Propylon Gateway endpoint URL (e.g. https://mcp.dvlpid.my.id)
    #[arg(short, long, default_value = "https://mcp.dvlpid.my.id")]
    pub remote: String,

    /// Optional explicit Bearer token override
    #[arg(short, long, env = "PROPYLON_TOKEN")]
    pub token: Option<String>,
}

