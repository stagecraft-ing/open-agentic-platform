---
id: "162-dockerised-execution-substrate"
slug: dockerised-execution-substrate
title: "Dockerised execution substrate — ephemeral micro-sandbox runtime for factory-engine codegen (ASI05)"
status: draft
implementation: pending
owner: bart
created: "2026-05-22"
kind: platform
risk: high
depends_on:
  - "072"  # multi-cloud K8s portability (the K8s primitives)
  - "075"  # factory-workflow-engine (the codegen producer)
  - "102"  # governed-excellence (the certificate chain that records sandbox use)
  - "108"  # factory-as-platform-feature
code_aliases: ["ASI05_SANDBOX", "FACTORY_CODEGEN_SANDBOX"]
constrains:
  - flavor: invariant-freeze
    unit: { kind: directory, path: crates/factory-engine }
references:
  - role: decomposition-source
    unit: { kind: file, path: docs/owasp/factory/AIDE-VELOCITY-OAP-INTENT.md }
  - role: doctrine-source
    unit: { kind: file, path: docs/owasp/owasp_top_10_agentic_applications_summary.md }
  - role: producer-spec
    unit: { kind: file, path: specs/075-factory-workflow-engine/spec.md }
  - role: governance-receipt
    unit: { kind: file, path: specs/102-governed-excellence/spec.md }
  - role: existing-k8s-baseline
    unit: { kind: directory, path: platform/k8s/policies/namespace-baseline }
compliance:
  - framework: owasp-asi-2026
    controls: ["ASI05", "ASI10"]
summary: >
  OAP's single largest open OWASP ASI 2026 compliance gap.
  When the factory-engine emits code that must be
  exercised (lint, test, build, run-once for evaluation),
  the execution today happens on host runtimes without a
  hardened isolation boundary. ASI05 ("Unexpected Code
  Execution / RCE") prescribes ephemeral micro-sandbox
  isolation as the canonical mitigation. This spec
  establishes that substrate: a network-isolated,
  TTL-capped, resource-quota'd execution boundary
  (gVisor / Firecracker / strict-network-policy K8s pod
  per execution) for every adapter-emitted code block
  the platform exercises.

  The substrate is non-negotiable. The intent doc §7.3
  records: *"The substrate is not OWASP-compliant until
  it lands."* Spec 162 is the spec-spine home for that
  landing. It applies symmetrically: at L1 (OAP's own
  factory-engine codegen runs) and at L2 (a tenant
  project's own codegen agents, inheriting the
  substrate via the born-with kernel — spec 167).

  Spec 162 constrains spec 075 (factory-workflow-engine)
  with an invariant: no adapter-emitted code is
  exercised outside the sandbox contract. Existing
  K8s baseline policies under
  `platform/k8s/policies/namespace-baseline/` are the
  precedent; the sandbox extends them with
  execution-boundary semantics they currently lack.
---

# 162 — Dockerised execution substrate (ASI05)

## 1. Problem

The intent doc §7.3 is unambiguous:

> *"The single largest open compliance gap is the absence of a
> sandboxed execution substrate for code that the factory-engine
> emits and then exercises. Today, when an adapter generates code
> that needs to be linted, tested, or built, the execution happens
> on host runtimes without a hardened isolation boundary. ASI05's
> core prescription is **ephemeral micro-sandbox isolation** —
> gVisor / Firecracker / strict-network-policy K8s pods, with
> TTL ceilings, `pids_limit`, memory + CPU caps, and no host
> file-system or control-plane access."*

OWASP ASI 2026's ASI05 control names this exactly. The OAP coverage
matrix in the convergence doc §3 tagged it as **Primary gap** —
every other ASI control has at least one spec doing structural work
today; ASI05's mitigation does not.

The shape of the gap:

1. **Factory-engine adapters emit code.** `aim-vue-node` produces
   Vue/Node scaffolds; future adapters produce other shapes. The
   emitted code typically needs lint/test/build cycles to verify
   it before the pipeline marks the stage complete.
2. **The execution happens on host runtimes today.** Whether the
   pipeline runs on a developer's workstation (OPC-driven local
   runs) or in CI, the lint/test/build invocations execute against
   the host's package manager, the host's filesystem, the host's
   network. No isolation boundary exists.
3. **The risk is structural, not theoretical.** A poisoned
   knowledge source (per ASI06), a prompt-injection vector that
   reaches the adapter's code-generation step, a compromised
   upstream dependency in the adapter's template — any of these
   can place attacker-controlled code into the host execution
   context.
4. **Tenant projects inherit the gap.** Per intent doc §7.3 ("at
   both L1 and L2"), a tenant project born of a factory adapter
   carries the same gap. Closing it at the substrate level closes
   it for every produced project by construction.

The K8s baseline policies under
`platform/k8s/policies/namespace-baseline/` already declare
network-deny defaults and resource quotas at the namespace level;
those primitives are necessary but not sufficient. Network deny
applies at namespace boundary, not at per-execution boundary;
resource quotas cap total namespace consumption, not per-execution
spike. The sandbox is the per-execution boundary.

## 2. Decision

Establish an ephemeral micro-sandbox runtime as the canonical
execution boundary for factory-engine adapter codegen. The runtime
is a K8s pod (with optional gVisor or Firecracker sandbox runtime
when the cluster supports it) provisioned per execution, with the
following invariants:

- **Ephemerality.** The pod exists only for the duration of one
  execution. TTL is bounded at the pod spec (a `activeDeadlineSeconds`
  ceiling, default 300s, configurable per stage with a hard
  upper bound enforced by admission policy).
- **Isolation.** Network policy denies all egress by default;
  egress allowlists are declared per stage and pinned to specific
  upstream hosts (package registry, container registry). No host
  filesystem mounts. No service-account token mount. No
  cluster-internal endpoints reachable except those on the
  per-stage allowlist.
- **Resource ceilings.** `requests` + `limits` for CPU and memory
  are required. `pids_limit` is set. No privileged containers, no
  `hostPath`, no `hostNetwork`, no `hostPID`, no `hostIPC`.
- **Auditable.** Every sandbox execution writes an entry to the
  governance certificate's audit chain (spec 102), binding the
  pod's resource state, the executed command, the input artifact
  hashes, and the output artifact hashes. The certificate's
  verifier (FR-007 of spec 102) treats sandbox-execution stages
  as first-class artifact-bearing stages.

The mitigation strategy is layered:

1. **Layer A — Pod admission policies.** K8s baseline policies are
   extended with PodSecurity admission rules under
   `platform/k8s/policies/sandbox/` that reject any sandbox pod
   not matching the invariants above.
2. **Layer B — Pod spec templates.** Per-stage pod spec templates
   live under `platform/k8s/sandbox-templates/` (or relocated per
   spec 160's relocation contract for stage assets). Each template
   carries the network policy, resource ceilings, and
   activeDeadlineSeconds.
3. **Layer C — gVisor / Firecracker overlay.** Where the cluster
   has gVisor installed (via the `runtimeClassName: gvisor`
   primitive) or a Firecracker-based RuntimeClass available, the
   sandbox pod spec selects that runtime by default. Where no
   sandbox runtime is available, the pod falls back to the
   standard runtime *with strict network policy and resource
   ceilings only*, and an explicit warning is recorded in the
   governance certificate noting the degraded isolation tier.

## 3. Functional Requirements

- **FR-001** Every adapter-emitted code block that the
  factory-engine exercises (lint/test/build/run-once for
  evaluation) runs inside a sandbox pod conforming to §2's
  invariants. Execution outside the sandbox is a factory-engine
  violation surfaced at pipeline-time, not at audit-time.
- **FR-002** The sandbox pod's network policy denies all egress by
  default. Per-stage egress allowlists name specific upstream
  hosts (e.g., `registry.npmjs.org`, `ghcr.io`); allowlist entries
  are declared in the stage's manifest and pinned to TLS-verified
  hostnames, not IPs.
- **FR-003** The sandbox pod sets `activeDeadlineSeconds` ≤ 300s
  by default. Stages requiring longer execution declare an
  explicit override in the stage manifest; the override is
  bounded by a hard upper limit (15 minutes) enforced by admission
  policy.
- **FR-004** The sandbox pod sets resource `limits` for CPU and
  memory and a `pids_limit` (default 1024). No `requests` value
  may exceed its `limits` value.
- **FR-005** The sandbox pod runs with `runAsNonRoot: true`,
  `runAsUser != 0`, `allowPrivilegeEscalation: false`,
  `readOnlyRootFilesystem: true`, `seccompProfile: RuntimeDefault`.
  Privileged mode and Linux capability additions are rejected by
  admission policy.
- **FR-006** No `hostPath` mounts, no service-account token mount
  (`automountServiceAccountToken: false`), no `hostNetwork`,
  `hostPID`, or `hostIPC`. The pod sees only its own namespace.
- **FR-007** When the cluster has a sandbox runtime available
  (gVisor `runtimeClassName: gvisor` or Firecracker equivalent),
  the sandbox pod selects it. When no sandbox runtime is
  available, the pod runs on the standard runtime with degraded
  isolation; the governance certificate records the degraded
  tier explicitly. The runtime selection is policy, not
  developer-discretionary.
- **FR-008** Every sandbox execution emits a `sandbox-execution`
  stage entry in the governance certificate's stage list (spec
  102 §FR-007), binding: the executed command, the input artifact
  hashes (pre-execution), the output artifact hashes
  (post-execution), the resource utilization peak, the
  RuntimeClass used, and whether the run hit the
  activeDeadlineSeconds ceiling.
- **FR-009** The factory-engine refuses to exercise adapter-emitted
  code when the sandbox cannot be provisioned (cluster
  unreachable, admission denied, RuntimeClass missing where
  required). The pipeline halts with an explicit error and does
  not fall back to host execution.
- **FR-010** A tenant project, born of the substrate per spec 167
  (born-with kernel), inherits the sandbox contract: its own
  factory-engine runs against the same sandbox primitives, in
  its own K8s namespace, with the same admission policies.

## 4. Success Criteria

- **SC-001** A factory-engine run that includes a codegen-exercise
  stage (lint/test/build/run-once) provisions a sandbox pod for
  the exercise step and tears it down at completion.
- **SC-002** An adversarial test — a deliberately malicious
  adapter template that attempts to read host files, fork
  unbounded processes, or reach a non-allowlisted host — is
  prevented by the sandbox invariants. The malicious attempts
  fail; the pipeline halts; the governance certificate records
  the failure with diagnostic detail.
- **SC-003** A pipeline run in an offline / degraded environment
  (no cluster reachable) halts at the codegen-exercise step
  rather than silently falling back to host execution.
- **SC-004** `make verify-certificate` on a sandbox-execution
  stage's entry confirms the executed-command + input-hash +
  output-hash binding; tampering with any of these fields fails
  verification with a specific diagnostic.

## 5. Scope

### In scope

- The pod admission policies under `platform/k8s/policies/sandbox/`.
- The pod spec templates and per-stage manifests for sandbox
  parameters.
- The factory-engine integration that provisions a sandbox pod
  for each exercise step.
- The governance certificate stage shape for `sandbox-execution`
  entries.
- The fallback tier semantics when a sandbox runtime is
  unavailable.

### Out of scope (deferred)

- **Sandboxed execution for non-factory-engine workloads.** The
  same pattern could apply to OPC-driven local execution of
  agent-emitted code; that is a separate spec.
- **In-process sandboxes (WASM, V8 isolates).** The substrate is
  pod-level for now. WASM-based sandboxes are a viable future
  layer but introduce different threat-model assumptions and
  require separate authoring.
- **Audit-log retention policy.** The governance certificate
  records sandbox execution; retention of the underlying pod
  logs (stdout, stderr) is a separate operational concern.
- **Cost accounting.** Sandbox pod resource consumption appears
  in the certificate; aggregating costs across tenants is a
  separate concern.

## 6. Compliance — load-bearing for ASI05 and ASI10

This spec is the canonical OAP mitigation for **ASI05** (Unexpected
Code Execution / RCE). Per OWASP ASI 2026 doctrine, ephemeral
micro-sandbox isolation is *the* prescribed mitigation; spec 162
is the structural implementation.

It is also a co-mitigation for **ASI10** (Rogue Agents): a
compromised codegen agent cannot spin up parasite processes,
exhaust host resources, or persist artifacts outside the sandbox
because every execution is bounded by the §2 invariants and
audited via spec 102.

The intent doc §7.2 records ASI05 as "**NON-NEGOTIABLE GAP**" and
"**Critical** — must be specced as part of this convergence." Spec
162 is that specing.

## 7. Cross-references

- **INTENT doc** §7.3, §9.4.
- **Spec 075** — factory-workflow-engine; constrained by this
  spec's exercise-step invariant.
- **Spec 102** — governed-excellence; consumes
  `sandbox-execution` stage entries.
- **Spec 072** — multi-cloud K8s portability; provides cluster
  primitives.
- **Spec 167** — born-with kernel emission; tenants inherit the
  sandbox contract via the kernel.
- **Spec 116** — supply-chain policy gates; complementary
  (sandbox protects against runtime exploitation; 116 protects
  against ingestion).
- **`platform/k8s/policies/namespace-baseline/`** — existing
  precedent (network deny + resource quotas + limit range); the
  sandbox extends these with execution-scoped semantics.
