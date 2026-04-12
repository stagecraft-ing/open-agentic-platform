// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: PROMOTION_GRADE_MIRROR
// Spec: specs/097-promotion-grade-mirror/spec.md

use crate::state::{StepExecutionStatus, WorkflowState};
use serde::{Deserialize, Serialize};

/// Whether a completed run is eligible for platform promotion (097 Slice 1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PromotionEligibility {
    Eligible,
    Ineligible { reasons: Vec<String> },
}

/// Status of platform event/artifact synchronization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStatus {
    /// Whether all workflow events have been acknowledged by the platform.
    pub events_synced: bool,
    /// Whether all step artifacts have been recorded to the platform.
    pub artifacts_recorded: bool,
    /// Last sync error, if any.
    pub last_sync_error: Option<String>,
}

/// Full promotion check result with all individual criteria (097 Slice 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionCheck {
    pub workflow_id: String,
    pub workspace_id: Option<String>,
    pub governance_active: bool,
    pub events_synced: bool,
    pub artifacts_recorded: bool,
    pub all_steps_completed: bool,
    pub eligibility: PromotionEligibility,
}

/// Check whether a completed workflow run is eligible for platform promotion.
///
/// A run is eligible when all five criteria are met:
/// 1. `workspace_id` is present in workflow metadata
/// 2. Governance was active during the run (not bypassed)
/// 3. All events were acknowledged by platform
/// 4. All artifacts were recorded to platform
/// 5. All steps completed successfully (no Failed or Skipped)
pub fn check_promotion_eligibility(
    state: &WorkflowState,
    sync_status: &SyncStatus,
) -> PromotionCheck {
    let workspace_id = state
        .metadata
        .get("workspace_id")
        .and_then(|v| v.as_str())
        .map(String::from);

    let governance_mode = state
        .metadata
        .get("governance_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let governance_active = governance_mode != "bypass";

    let all_steps_completed = state
        .steps
        .iter()
        .all(|s| s.status == StepExecutionStatus::Completed);

    let mut reasons: Vec<String> = Vec::new();

    if workspace_id.is_none() {
        reasons.push("workspace_id not present in workflow metadata".into());
    }
    if !governance_active {
        reasons.push("governance was bypassed during this run".into());
    }
    if !sync_status.events_synced {
        reasons.push("not all events were acknowledged by platform".into());
    }
    if !sync_status.artifacts_recorded {
        reasons.push("not all artifacts were recorded to platform".into());
    }
    if !all_steps_completed {
        let failed: Vec<String> = state
            .steps
            .iter()
            .filter(|s| {
                s.status == StepExecutionStatus::Failed
                    || s.status == StepExecutionStatus::Skipped
            })
            .map(|s| format!("{} ({})", s.id, format!("{:?}", s.status).to_lowercase()))
            .collect();
        reasons.push(format!(
            "not all steps completed successfully: {}",
            failed.join(", ")
        ));
    }

    let eligibility = if reasons.is_empty() {
        PromotionEligibility::Eligible
    } else {
        PromotionEligibility::Ineligible { reasons }
    };

    PromotionCheck {
        workflow_id: state.workflow_id.to_string(),
        workspace_id,
        governance_active,
        events_synced: sync_status.events_synced,
        artifacts_recorded: sync_status.artifacts_recorded,
        all_steps_completed,
        eligibility,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WorkflowState;
    use serde_json::Value as JsonValue;
    use uuid::Uuid;

    fn make_completed_state(
        workspace_id: Option<&str>,
        governance_mode: &str,
    ) -> WorkflowState {
        let mut meta = serde_json::Map::new();
        if let Some(ws) = workspace_id {
            meta.insert(
                "workspace_id".into(),
                JsonValue::String(ws.into()),
            );
        }
        meta.insert(
            "governance_mode".into(),
            JsonValue::String(governance_mode.into()),
        );

        let mut state = WorkflowState::new(
            Uuid::new_v4(),
            "test-workflow",
            "2026-04-11T00:00:00Z".into(),
            vec![
                ("step-01".into(), "lint".into()),
                ("step-02".into(), "test".into()),
            ],
            meta,
        );
        // Mark all steps completed
        for step in &mut state.steps {
            step.status = crate::state::StepExecutionStatus::Completed;
            step.completed_at = Some("2026-04-11T00:01:00Z".into());
        }
        state
    }

    #[test]
    fn sc097_2_eligible_when_all_criteria_met() {
        let state = make_completed_state(Some("ws-001"), "governed");
        let sync = SyncStatus {
            events_synced: true,
            artifacts_recorded: true,
            last_sync_error: None,
        };

        let check = check_promotion_eligibility(&state, &sync);
        assert_eq!(check.eligibility, PromotionEligibility::Eligible);
        assert!(check.governance_active);
        assert!(check.all_steps_completed);
        assert_eq!(check.workspace_id, Some("ws-001".into()));
    }

    #[test]
    fn sc097_2_ineligible_without_workspace() {
        let state = make_completed_state(None, "governed");
        let sync = SyncStatus {
            events_synced: true,
            artifacts_recorded: true,
            last_sync_error: None,
        };

        let check = check_promotion_eligibility(&state, &sync);
        match &check.eligibility {
            PromotionEligibility::Ineligible { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("workspace_id")));
            }
            PromotionEligibility::Eligible => panic!("expected ineligible"),
        }
    }

    #[test]
    fn sc097_2_ineligible_when_governance_bypassed() {
        let state = make_completed_state(Some("ws-001"), "bypass");
        let sync = SyncStatus {
            events_synced: true,
            artifacts_recorded: true,
            last_sync_error: None,
        };

        let check = check_promotion_eligibility(&state, &sync);
        assert!(!check.governance_active);
        match &check.eligibility {
            PromotionEligibility::Ineligible { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("governance")));
            }
            PromotionEligibility::Eligible => panic!("expected ineligible"),
        }
    }

    #[test]
    fn sc097_2_ineligible_when_events_not_synced() {
        let state = make_completed_state(Some("ws-001"), "governed");
        let sync = SyncStatus {
            events_synced: false,
            artifacts_recorded: true,
            last_sync_error: Some("connection refused".into()),
        };

        let check = check_promotion_eligibility(&state, &sync);
        match &check.eligibility {
            PromotionEligibility::Ineligible { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("events")));
            }
            PromotionEligibility::Eligible => panic!("expected ineligible"),
        }
    }

    #[test]
    fn sc097_2_ineligible_when_step_failed() {
        let mut state = make_completed_state(Some("ws-001"), "governed");
        state.steps[1].status = crate::state::StepExecutionStatus::Failed;

        let sync = SyncStatus {
            events_synced: true,
            artifacts_recorded: true,
            last_sync_error: None,
        };

        let check = check_promotion_eligibility(&state, &sync);
        assert!(!check.all_steps_completed);
        match &check.eligibility {
            PromotionEligibility::Ineligible { reasons } => {
                assert!(reasons.iter().any(|r| r.contains("step-02")));
            }
            PromotionEligibility::Eligible => panic!("expected ineligible"),
        }
    }

    #[test]
    fn sc097_2_multiple_ineligibility_reasons() {
        let state = make_completed_state(None, "bypass");
        let sync = SyncStatus {
            events_synced: false,
            artifacts_recorded: false,
            last_sync_error: None,
        };

        let check = check_promotion_eligibility(&state, &sync);
        match &check.eligibility {
            PromotionEligibility::Ineligible { reasons } => {
                assert!(reasons.len() >= 4, "expected at least 4 reasons, got: {reasons:?}");
            }
            PromotionEligibility::Eligible => panic!("expected ineligible"),
        }
    }
}
