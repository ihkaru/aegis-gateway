// SPDX-License-Identifier: MIT

use std::path::Path;
use serde_json::json;

use crate::backend::TopologyConfigLoader;
use crate::cli::args::{
    AddArgs, AuditArgs, BridgeArgs, Cli, Command, DoctorArgs, InitArgs, ListArgs, LoginArgs,
    RemoveArgs, ServeArgs, UiArgs, ValidateArgs,
};
use crate::core::error::{PropylonError, PropylonResult};
use crate::daemon::DaemonSupervisor;

/// Main CLI command dispatcher
pub async fn run(cli: Cli) -> PropylonResult<()> {
    if cli.stdio {
        let args = ServeArgs {
            stdio: true,
            host: "0.0.0.0".to_string(),
            port: 39400,
            config: cli.config,
            no_ui: false,
            immutable_config: false,
            lean: false,
        };
        return run_serve(args).await;
    }

    match cli.command {
        Some(Command::Serve(mut args)) => {
            if args.config.is_none() && cli.config.is_some() {
                args.config = cli.config;
            }
            run_serve(args).await
        }
        Some(Command::Init(args)) => run_init(args),
        Some(Command::Validate(args)) => run_validate(args),
        Some(Command::Add(args)) => run_add(args),
        Some(Command::Remove(args)) => run_remove(args),
        Some(Command::List(args)) => run_list(args),
        Some(Command::Doctor(args)) => run_doctor(args),
        Some(Command::Audit(args)) => run_audit(args).await,
        Some(Command::Ui(args)) => run_ui(args).await,
        Some(Command::Login(args)) => run_login(args),
        Some(Command::Bridge(args)) => run_bridge(args).await,
        None => {
            let args = ServeArgs {
                stdio: false,
                host: "0.0.0.0".to_string(),
                port: 39400,
                config: cli.config,
                no_ui: false,
                immutable_config: false,
                lean: false,
            };
            run_serve(args).await
        }
    }
}

pub async fn run_serve(args: ServeArgs) -> PropylonResult<()> {
    let (gateway, handler, _registry) = DaemonSupervisor::bootstrap(args.config.as_deref()).await?;

    if args.stdio {
        DaemonSupervisor::run_stdio(handler).await
    } else {
        DaemonSupervisor::run_http(gateway, handler, &args.host, args.port).await
    }
}

pub fn run_init(args: InitArgs) -> PropylonResult<()> {
    let path = Path::new(&args.output);
    if path.exists() && !args.force {
        return Err(PropylonError::Internal(format!(
            "File '{}' already exists. Use --force to overwrite.",
            args.output
        )));
    }

    let template = TopologyConfigLoader::template();
    let yaml = serde_yaml::to_string(&template)
        .map_err(|e| PropylonError::Internal(format!("Failed to serialize template: {e}")))?;

    std::fs::write(path, yaml).map_err(|e| {
        PropylonError::Internal(format!("Failed to write configuration file '{}': {e}", args.output))
    })?;

    println!("[PROPYLON] Successfully generated template configuration at '{}'", args.output);
    println!("Edit this file to configure your local or remote MCP servers, then run:");
    println!("  propylon serve --config {}", args.output);
    Ok(())
}

pub fn run_validate(args: ValidateArgs) -> PropylonResult<()> {
    let content = std::fs::read_to_string(&args.config).map_err(|e| {
        PropylonError::Internal(format!("Failed to read configuration file '{}': {e}", args.config))
    })?;

    let cfg = TopologyConfigLoader::load_from_str(&content)?;
    TopologyConfigLoader::validate(&cfg)?;

    println!("[PROPYLON] Configuration '{}' is VALID.", args.config);
    println!("Configured MCP Servers ({}):", cfg.mcp_servers.len());
    for (name, srv) in &cfg.mcp_servers {
        if let Some(cmd) = &srv.command {
            println!("  - {}: command='{}', args={:?}, timeout={}s", name, cmd, srv.args, srv.timeout_secs);
        } else if let Some(url) = &srv.url {
            println!("  - {}: remote_url='{}', timeout={}s", name, url, srv.timeout_secs);
        }
    }

    Ok(())
}

pub fn run_doctor(args: DoctorArgs) -> PropylonResult<()> {
    let checks = vec![
        ("Rust Runtime", true, "Available (edition 2024)"),
        ("Node.js / Bun", check_command_exists("bun") || check_command_exists("node"), "Required for npx MCP tools"),
        ("Python 3", check_command_exists("python3"), "Required for uv/pip MCP tools"),
        ("Docker CLI", check_command_exists("docker"), "Optional for containerized MCP servers"),
        ("Git CLI", check_command_exists("git"), "Required for GitOps skill synchronization"),
    ];

    if args.json {
        let results: Vec<_> = checks
            .into_iter()
            .map(|(name, ok, note)| json!({ "check": name, "available": ok, "notes": note }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        println!("============================================================");
        println!("         PROPYLON GATEWAY: SYSTEM PREFLIGHT DOCTOR             ");
        println!("============================================================");
        for (name, ok, note) in checks {
            let status = if ok { "[PASS]" } else { "[WARN]" };
            println!("{:<8} {:<18}: {}", status, name, note);
        }
        println!("------------------------------------------------------------");
        println!("System is ready to run Propylon Gateway.");
    }

    Ok(())
}

pub async fn run_audit(args: AuditArgs) -> PropylonResult<()> {
    let sequencer = crate::audit::HashChainSequencer::new();
    let report = sequencer.export_soc2_report().await;

    if let Some(out_path) = args.output {
        std::fs::write(&out_path, serde_json::to_string_pretty(&report)?).map_err(|e| {
            PropylonError::Internal(format!("Failed to write audit report to '{out_path}': {e}"))
        })?;
        println!("[PROPYLON] Exported SOC 2 Type II audit report to '{}'", out_path);
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    Ok(())
}

pub fn run_add(args: AddArgs) -> PropylonResult<()> {
    let path = Path::new(&args.config);
    let mut cfg = if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PropylonError::Internal(format!("Failed to read '{}': {e}", args.config)))?;
        TopologyConfigLoader::load_from_str(&content)?
    } else {
        TopologyConfigLoader::template()
    };

    let mut env_map = std::collections::HashMap::new();
    for item in args.env {
        if let Some((k, v)) = item.split_once('=') {
            env_map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }

    let backend = crate::core::backend::BackendConfig {
        name: args.name.clone(),
        command: args.command,
        args: args.args,
        url: args.url,
        env: env_map,
        timeout_secs: 30,
        enabled: true,
    };

    cfg.mcp_servers.insert(args.name.clone(), backend);
    let yaml = serde_yaml::to_string(&cfg)
        .map_err(|e| PropylonError::Internal(format!("Failed to serialize config: {e}")))?;
    std::fs::write(path, yaml)
        .map_err(|e| PropylonError::Internal(format!("Failed to write '{}': {e}", args.config)))?;

    println!("[PROPYLON] Successfully added backend '{}' to '{}'", args.name, args.config);
    Ok(())
}

pub fn run_remove(args: RemoveArgs) -> PropylonResult<()> {
    let path = Path::new(&args.config);
    if !path.exists() {
        return Err(PropylonError::Internal(format!("Configuration file '{}' not found", args.config)));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| PropylonError::Internal(format!("Failed to read '{}': {e}", args.config)))?;
    let mut cfg = TopologyConfigLoader::load_from_str(&content)?;

    if cfg.mcp_servers.remove(&args.name).is_none() {
        return Err(PropylonError::Internal(format!("Backend '{}' not found in '{}'", args.name, args.config)));
    }

    let yaml = serde_yaml::to_string(&cfg)
        .map_err(|e| PropylonError::Internal(format!("Failed to serialize config: {e}")))?;
    std::fs::write(path, yaml)
        .map_err(|e| PropylonError::Internal(format!("Failed to write '{}': {e}", args.config)))?;

    println!("[PROPYLON] Successfully removed backend '{}' from '{}'", args.name, args.config);
    Ok(())
}

pub fn run_list(args: ListArgs) -> PropylonResult<()> {
    let path = Path::new(&args.config);
    if !path.exists() {
        return Err(PropylonError::Internal(format!("Configuration file '{}' not found", args.config)));
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| PropylonError::Internal(format!("Failed to read '{}': {e}", args.config)))?;
    let cfg = TopologyConfigLoader::load_from_str(&content)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&cfg.mcp_servers)?);
    } else {
        println!("============================================================");
        println!("         PROPYLON GATEWAY: CONFIGURED MCP SERVERS              ");
        println!("============================================================");
        println!("Configuration: {}", args.config);
        println!("Total Servers: {}", cfg.mcp_servers.len());
        println!("------------------------------------------------------------");
        for (name, srv) in &cfg.mcp_servers {
            let status = if srv.enabled { "ACTIVE" } else { "DISABLED" };
            if let Some(cmd) = &srv.command {
                println!("[{status}] {name} (stdio: command='{cmd}', args={:?})", srv.args);
            } else if let Some(url) = &srv.url {
                println!("[{status}] {name} (remote: url='{url}')");
            }
        }
        println!("------------------------------------------------------------");
    }
    Ok(())
}

fn check_command_exists(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            if Path::new(dir).join(cmd).is_file() {
                return true;
            }
        }
    }
    false
}

pub async fn run_ui(args: UiArgs) -> PropylonResult<()> {
    let (gateway, _handler, registry) = DaemonSupervisor::bootstrap(args.config.as_deref()).await?;
    let server = crate::control::web_ui::EmbeddedAdminServer::new(args.port, true)
        .with_gateway(gateway)
        .with_registry(registry);

    println!("============================================================");
    println!("      PROPYLON GATEWAY: EMBEDDED ADMIN WEB UI CONTROL PLANE    ");
    println!("============================================================");
    println!("Serving Svelte 5 Dashboard at http://{}:{}", args.host, args.port);
    println!("Open http://localhost:{} in your browser", args.port);
    println!("------------------------------------------------------------");
    server.run_server(&args.host).await
}

pub fn run_login(args: LoginArgs) -> PropylonResult<()> {
    crate::cli::bridge_cmd::run_login(args)
}

pub async fn run_bridge(args: BridgeArgs) -> PropylonResult<()> {
    crate::cli::bridge_cmd::run_bridge(args).await
}

