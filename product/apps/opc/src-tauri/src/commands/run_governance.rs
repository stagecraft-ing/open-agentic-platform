// Spec 198 FR-005/FR-014 — run-side governance for factory pipelines.
//
// The desktop is the keyless executor (ASI10 m6): before any stage runs it
// must (1) verify the platform's admission seal over the factory content it
// is about to trust (ASI04 m1), (2) file the run's intent capsule and obtain
// a signed run-grant, and (3) renew that grant at every stage boundary —
// a refused renewal fails the step fail-closed (goal-shift / revocation
// propagation, ASI01 m4/m7). At emission the certificate binds the admitted
// envelope + capsule, and the platform countersign is patched in when the
// completion round-trip succeeds.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use factory_engine::governance_certificate::CapsuleBinding;
use factory_engine::intent_capsule::IntentCapsule;
use factory_engine::platform_jws::{PlatformJwks, TYP_ADMISSION_SEAL, verify_compact_jws};
use orchestrator::PreStepGate;

use super::factory_platform::{GrantOutcome, GrantRenewArgs, GrantRequestArgs, RunEmitter};
use super::stagecraft_client::StagecraftClient;

/// Everything a governed run carries from admission verification to
/// certificate sealing. Constructed once per run by [`establish`].
pub struct RunGovernance {
    pub capsule: IntentCapsule,
    pub envelope_hash: String,
    pub jwks: PlatformJwks,
    /// Sequence of the most recent grant the platform issued. The next
    /// renewal presents `last_seq + 1`; the reply's seq is stored back
    /// (issuance after an engine restart may resume above 0).
    pub last_seq: AtomicI64,
    emitter: RunEmitter,
}

impl RunGovernance {
    pub fn capsule_binding(&self) -> CapsuleBinding {
        CapsuleBinding {
            admitted_envelope_hash: self.envelope_hash.clone(),
            goal_id: self.capsule.goal_id.clone(),
            intent_capsule_hash: self.capsule.capsule_hash(),
        }
    }
}

/// Verify the bundle's admission seal, file the intent capsule, and obtain
/// the issuance grant. Every failure is fail-closed: the caller MUST NOT
/// dispatch any stage on `Err` (spec 198 FR-001/FR-005/FR-014).
pub async fn establish(
    sc: &StagecraftClient,
    emitter: RunEmitter,
    stagecraft_project_id: &str,
    platform_run_id: &str,
    goal: &str,
    run_dir: &Path,
) -> Result<Arc<RunGovernance>, String> {
    // 1. The standing admission, with the platform seal (ASI04 m1).
    let bundle = sc
        .get_project_opc_bundle(stagecraft_project_id)
        .await
        .map_err(|e| format!("cannot fetch project bundle for admission check: {e}"))?;
    let admission = bundle.admission.ok_or_else(|| {
        "factory is not admitted for this org — a run cannot start ungoverned; \
         re-sync the factory and resolve the admission refusal (spec 198 FR-001)"
            .to_string()
    })?;
    let seal = admission.seal_jws.ok_or_else(|| {
        "factory admission is UNSEALED (no platform signature) — refuse fail-closed; \
         re-sync after the platform signing authority is configured (spec 198 FR-014)"
            .to_string()
    })?;

    let jwks = sc
        .fetch_factory_jwks()
        .await
        .map_err(|e| format!("cannot fetch platform JWKS for seal verification: {e}"))?;
    let verified = verify_compact_jws(&seal, &jwks, TYP_ADMISSION_SEAL)
        .map_err(|e| format!("admission seal rejected: {e} (spec 198 FR-014)"))?;
    let sealed_origin = verified.payload["origin"].as_str().unwrap_or("");
    if sealed_origin != admission.origin {
        return Err(format!(
            "admission seal binds origin '{sealed_origin}' but the bundle claims '{}'",
            admission.origin
        ));
    }
    let envelope_hash = verified.payload["envelope_hash"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if envelope_hash.is_empty()
        || admission.envelope_hash.as_deref() != Some(envelope_hash.as_str())
    {
        return Err(
            "admission seal envelope hash does not match the bundle's admission block".into(),
        );
    }

    // 2. File the intent capsule (FR-005) — persisted next to the run's
    //    other governance artifacts for audit + certificate binding.
    let capsule = IntentCapsule::new(
        goal,
        Vec::new(),
        envelope_hash.clone(),
        stagecraft_project_id,
        platform_run_id,
    );
    capsule
        .persist(run_dir)
        .map_err(|e| format!("cannot persist intent capsule: {e}"))?;

    // 3. Issuance grant. A refusal or an unreachable platform halts the
    //    run before s0 — the executor never self-starts governed work.
    let outcome = emitter
        .request_grant(GrantRequestArgs {
            goal_id: &capsule.goal_id,
            goal: &capsule.goal,
            capsule_hash: &capsule.capsule_hash(),
            envelope_hash: &envelope_hash,
            build_spec_hash: None,
            project_id: Some(stagecraft_project_id),
            constraints: Some(capsule.constraints.clone()),
        })
        .await
        .map_err(|e| format!("run-grant request failed (fail closed): {e}"))?;
    let seq = match outcome {
        GrantOutcome::Granted { seq, .. } => seq,
        GrantOutcome::Refused { reason, detail } => {
            return Err(format!(
                "run-grant refused: {reason}{} (spec 198 FR-005)",
                detail.map(|d| format!(" — {d}")).unwrap_or_default()
            ));
        }
    };

    Ok(Arc::new(RunGovernance {
        capsule,
        envelope_hash,
        jwks,
        last_seq: AtomicI64::new(seq),
        emitter,
    }))
}

/// Stage-boundary grant renewal (spec 198 FR-005 — "signed per execution
/// cycle"). Wired into `DispatchOptions.pre_step`; an `Err` fails the step
/// and halts the run with the platform's attributable refusal.
pub struct GrantRenewalGate {
    gov: Arc<RunGovernance>,
    /// Reads the frozen Build-Spec hash from the run's live pipeline state;
    /// presented from the freeze boundary onward (the platform records it
    /// one-way on the grant chain).
    build_spec_hash: Box<dyn Fn() -> Option<String> + Send + Sync>,
}

impl GrantRenewalGate {
    pub fn new(
        gov: Arc<RunGovernance>,
        build_spec_hash: Box<dyn Fn() -> Option<String> + Send + Sync>,
    ) -> Self {
        Self {
            gov,
            build_spec_hash,
        }
    }
}

#[async_trait::async_trait]
impl PreStepGate for GrantRenewalGate {
    async fn before_step(&self, step_id: &str) -> Result<(), String> {
        let next = self.gov.last_seq.load(Ordering::Acquire) + 1;
        let build_spec = (self.build_spec_hash)();
        let outcome = self
            .gov
            .emitter
            .renew_grant(GrantRenewArgs {
                goal_id: &self.gov.capsule.goal_id,
                capsule_hash: &self.gov.capsule.capsule_hash(),
                seq: next,
                stage_id: Some(step_id),
                build_spec_hash: build_spec.as_deref(),
            })
            .await
            .map_err(|e| {
                format!("run-grant renewal unavailable at {step_id}: {e} (fail closed)")
            })?;
        match outcome {
            GrantOutcome::Granted { seq, .. } => {
                self.gov.last_seq.store(seq, Ordering::Release);
                Ok(())
            }
            GrantOutcome::Refused { reason, detail } => Err(format!(
                "run-grant renewal refused at {step_id}: {reason}{} (spec 198 FR-005)",
                detail.map(|d| format!(" — {d}")).unwrap_or_default()
            )),
        }
    }
}

/// The per-run directory governance artifacts live in
/// (`<project>/.factory/runs/<run-id>/`).
pub fn run_dir_for(project_path: &Path, run_id: &str) -> PathBuf {
    project_path.join(".factory").join("runs").join(run_id)
}
