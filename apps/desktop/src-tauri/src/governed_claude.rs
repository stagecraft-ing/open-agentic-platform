//! Feature 035 — governed Claude Code launch via bundled axiomregent MCP (`--mcp-config`) + `OPC_GOVERNANCE_GRANTS`.

use serde_json::json;
use std::path::PathBuf;

use crate::commands::agents::Agent;

#[derive(Debug, Clone)]
pub enum GovernedPlan {
    Governed {
        mcp_config_json: String,
    },
    Bypass,
}

/// Resolve bundled `axiomregent-*` binary next to `src-tauri/binaries/` (dev) or sidecar name at runtime.
pub fn bundled_axiomregent_binary_path() -> Result<PathBuf, String> {
    let suffix = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "x86_64-pc-windows-msvc"
    } else {
        return Err("unsupported host triple for bundled axiomregent".to_string());
    };
    let mut filename = format!("axiomregent-{suffix}");
    if cfg!(target_os = "windows") {
        filename.push_str(".exe");
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(filename);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("bundled axiomregent not found at {}", path.display()))
    }
}

pub fn grants_json_for_agent(agent: &Agent) -> String {
    json!({
        "enable_file_read": agent.enable_file_read,
        "enable_file_write": agent.enable_file_write,
        "enable_network": agent.enable_network,
        "max_tier": 3
    })
    .to_string()
}

pub fn grants_json_claude_default() -> String {
    json!({
        "enable_file_read": true,
        "enable_file_write": true,
        "enable_network": true,
        "max_tier": 2
    })
    .to_string()
}

pub fn axiomregent_mcp_config_json(axiom_exe: &std::path::Path, grants_json: &str) -> Result<String, String> {
    let empty_args: Vec<String> = vec![];
    let cfg = json!({
        "mcpServers": {
            "opc-axiomregent": {
                "command": axiom_exe.to_string_lossy(),
                "args": empty_args,
                "env": {
                    "OPC_GOVERNANCE_GRANTS": grants_json
                }
            }
        }
    });
    serde_json::to_string(&cfg).map_err(|e| e.to_string())
}

/// `announce_port`: `SidecarState` probe port when Some (axiomregent announced readiness).
pub fn plan_governed(announce_port: Option<u16>, grants_json: String) -> GovernedPlan {
    if announce_port.is_none() {
        return GovernedPlan::Bypass;
    }
    let Ok(binary) = bundled_axiomregent_binary_path() else {
        return GovernedPlan::Bypass;
    };
    match axiomregent_mcp_config_json(&binary, &grants_json) {
        Ok(mcp_config_json) => GovernedPlan::Governed { mcp_config_json },
        Err(_) => GovernedPlan::Bypass,
    }
}

pub fn append_claude_governance_args(args: &mut Vec<String>, plan: &GovernedPlan) {
    match plan {
        GovernedPlan::Governed { mcp_config_json } => {
            args.push("--mcp-config".to_string());
            args.push(mcp_config_json.clone());
            args.push("--permission-mode".to_string());
            args.push("default".to_string());
        }
        GovernedPlan::Bypass => {
            args.push("--dangerously-skip-permissions".to_string());
        }
    }
}
