// Feature 035 — governed tool dispatch

use agent::safety::{ToolTier, get_tool_tier};
use serde_json::json;
use std::io::Write;

use crate::router::AxiomRegentError;
use crate::snapshot::lease::{Lease, PermissionGrants};

fn tier_rank(t: ToolTier) -> u8 {
    match t {
        ToolTier::Tier1 => 1,
        ToolTier::Tier2 => 2,
        ToolTier::Tier3 => 3,
    }
}

fn requires_file_read(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "snapshot.list"
            | "snapshot.read"
            | "snapshot.grep"
            | "snapshot.diff"
            | "snapshot.changes"
            | "snapshot.export"
            | "snapshot.info"
            | "gov.preflight"
            | "gov.drift"
            | "features.impact"
            | "xray.scan"
            | "agent.verify"
    )
}

fn requires_file_write(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "workspace.write_file"
            | "workspace.delete"
            | "workspace.apply_patch"
            | "snapshot.create"
            | "agent.propose"
            | "agent.execute"
    )
}

fn requires_network(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "run.execute" | "run.status" | "run.logs" | "agent.execute"
    )
}

/// Enforces tier ceiling and coarse permission flags against a set of grants.
pub fn check_grants(tool_name: &str, grants: &PermissionGrants) -> Result<(), AxiomRegentError> {
    let tool_tier = get_tool_tier(tool_name);
    let max_allowed = grants.max_tier.clamp(1, 3);
    if tier_rank(tool_tier) > max_allowed {
        return Err(AxiomRegentError::PermissionDenied(format!(
            "tool {tool_name} exceeds session max tier ({max_allowed})"
        )));
    }
    if requires_file_read(tool_name) && !grants.enable_file_read {
        return Err(AxiomRegentError::PermissionDenied(format!(
            "file read disabled for {tool_name}"
        )));
    }
    if requires_file_write(tool_name) && !grants.enable_file_write {
        return Err(AxiomRegentError::PermissionDenied(format!(
            "file write disabled for {tool_name}"
        )));
    }
    if requires_network(tool_name) && !grants.enable_network {
        return Err(AxiomRegentError::PermissionDenied(format!(
            "network disabled for {tool_name}"
        )));
    }
    Ok(())
}

/// Enforces tier ceiling and coarse permission flags for a tool call when a lease is present.
pub fn check_tool_permission(tool_name: &str, lease: &Lease) -> Result<(), AxiomRegentError> {
    check_grants(tool_name, &lease.grants)
}

/// Structured audit line on stderr (Feature 035 / T010).
pub fn audit_tool_dispatch(
    tool_name: &str,
    tier: &str,
    decision: &str,
    lease_id: Option<&str>,
) {
    let line = json!({
        "op": "axiomregent.tool_audit",
        "tool": tool_name,
        "tier": tier,
        "decision": decision,
        "lease_id": lease_id,
        "ts": chrono::Utc::now().to_rfc3339(),
    });
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "{line}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::lease::{Fingerprint, PermissionGrants};
    use std::collections::HashSet;

    fn lease_with(grants: PermissionGrants) -> Lease {
        Lease {
            id: "lid".into(),
            fingerprint: Fingerprint {
                head_oid: "".into(),
                index_oid: "".into(),
                status_hash: "".into(),
            },
            touched_files: HashSet::new(),
            grants,
        }
    }

    #[test]
    fn tier2_tool_denied_when_max_tier_1() {
        let lease = lease_with(PermissionGrants {
            enable_file_read: true,
            enable_file_write: true,
            enable_network: true,
            max_tier: 1,
        });
        assert!(check_tool_permission("workspace.write_file", &lease).is_err());
        assert!(check_tool_permission("gov.preflight", &lease).is_ok());
    }

    #[test]
    fn write_disabled_blocks_workspace_write() {
        let lease = lease_with(PermissionGrants {
            enable_file_read: true,
            enable_file_write: false,
            enable_network: true,
            max_tier: 3,
        });
        assert!(check_tool_permission("workspace.write_file", &lease).is_err());
    }

    #[test]
    fn read_disabled_blocks_snapshot_read() {
        let lease = lease_with(PermissionGrants {
            enable_file_read: false,
            enable_file_write: true,
            enable_network: true,
            max_tier: 3,
        });
        assert!(check_tool_permission("snapshot.read", &lease).is_err());
    }
}
