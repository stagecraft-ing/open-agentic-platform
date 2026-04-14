---
id: "100-post-convergence-remediation"
title: "Post-Convergence Remediation"
status: approved
implementation: pending
owner: bart
created: "2026-04-12"
risk: high
depends_on:
  - "089"
code_aliases: ["POST_CONVERGENCE_REMEDIATION"]
summary: >
  Remediate security vulnerabilities, integrity gaps, and technical debt surfaced
  during codebase analysis after the governed convergence plan (089) completed.
  Four phases: critical security, high security and platform hardening, integrity
  and consistency, orphaned code feature catalog for UI reintegration.
---

# 100 — Post-Convergence Remediation

Parent plan: [089 Governed Convergence](../089-governed-convergence-plan/spec.md)

## Problem

A comprehensive codebase analysis after the governed convergence plan (specs 090–099)
completed surfaced 19 issues across security, integrity, and orphaned code:

- **2 critical**: live credentials in working tree, JWT validation silently disabled
- **6 high**: Tauri CSP with unsafe-eval, assetProtocol wildcard scope, unvalidated
  webview project_path, path traversal in artifact stores
- **4 medium**: Azure Key Vault purge protection disabled, blocking Mutex in async,
  policy-kernel unwrap panics, deployd temp data dir
- **7 integrity**: stale spec registry, spec-template missing frontmatter, dependency
  version drift (thiserror, axum), superseded specs missing backlinks, unquoted YAML
  dates, untracked desktop Cargo.lock

Additionally, 17 TypeScript packages in `packages/` have zero consumers but contain
fully implemented features (specs 050–071) ready for UI reintegration.

## Solution

Four-phase remediation ordered by blast radius.

### Phase 1 — Critical Security

| Slice | Issue | Files |
|-------|-------|-------|
| 1.1 | Remove live Hetzner .env credentials | `platform/infra/hetzner/.env` (delete), `.gitignore` |
| 1.2 | Require JWT audience + scope validation | `deployd-api-rs/src/config.rs`, `auth.rs` |

### Phase 2 — High Security + Platform Hardening

| Slice | Issue | Files |
|-------|-------|-------|
| 2.1 | Remove unsafe-eval from Tauri CSP, narrow assetProtocol | `tauri.conf.json` |
| 2.2 | Validate project_path from webview | `commands/agents.rs` |
| 2.3 | Path traversal guards in artifact stores | `factory-engine/artifact_store.rs`, `orchestrator/artifact.rs` |
| 2.4 | Azure Key Vault purge protection | `azure_core/main.tf` |
| 2.5 | Handle poisoned Mutex in JWKS cache | `deployd-api-rs/auth.rs` |
| 2.6 | Replace bare unwrap in policy-kernel | `policy-kernel/src/lib.rs` |
| 2.7 | Change deployd data dir from /tmp | `deployd-api-rs/main.rs` |
| 2.8 | Add K8s security contexts | `stagecraft/deployment.yaml`, `deployd-api/deployment.yaml` |

### Phase 3 — Integrity & Consistency

| Slice | Issue | Files |
|-------|-------|-------|
| 3.1 | Add frontmatter to spec template | `.specify/templates/spec-template.md` |
| 3.2 | Create spec 100 | `specs/100-post-convergence-remediation/spec.md` |
| 3.3 | Upgrade thiserror 1.0 → 2 | `factory-contracts/Cargo.toml` |
| 3.4 | Upgrade axum 0.7 → 0.8 | `orchestrator/Cargo.toml` |
| 3.5 | Add superseded_by to specs 038, 040 | `specs/038-*/spec.md`, `specs/040-*/spec.md` |
| 3.6 | Quote YAML dates | `specs/087-*/spec.md`, `specs/088-*/spec.md` |
| 3.7 | Track desktop Cargo.lock | `.gitignore`, `apps/desktop/src-tauri/Cargo.lock` |
| 3.8 | Recompile spec registry (096–100) | `build/spec-registry/registry.json` |

### Phase 4 — Orphaned Code Feature Catalog

17 packages cataloged by integration cluster for UI reintegration:

- **Agent execution backbone**: worktree-agents, notification-orchestrator
- **Governance layer**: hookify-rule-engine, coherence-scoring
- **Prompt construction**: prompt-assembly, session-memory, yaml-standards-schema
- **Chat UX**: file-mention, tool-renderer, panel-event-bus
- **Work management**: conductor-track, verification-profiles
- **Skill system**: agent-frontmatter, skill-command-factory
- **Remote control**: git-panel, oap-ctl, multi-model-chaining

## Acceptance Criteria

- AC-100-1: No live credentials exist in the working tree
- AC-100-2: deployd-api rejects JWTs without matching audience
- AC-100-3: Tauri CSP contains no unsafe-eval; assetProtocol scoped to app dirs
- AC-100-4: Artifact stores reject path traversal in content_hash and filename
- AC-100-5: All K8s deployments have runAsNonRoot and allowPrivilegeEscalation: false
- AC-100-6: Spec registry contains specs 096–100
- AC-100-7: spec-template.md produces V-001/V-002 compliant specs
- AC-100-8: No thiserror or axum version drift across workspace
