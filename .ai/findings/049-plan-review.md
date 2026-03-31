# 049 Permission System — plan review

> **Reviewer:** claude | **Date:** 2026-03-30 | **Verdict:** APPROVED for Phase 1 start

## Review scope

Compared `.ai/plans/049-permission-system-phased-plan.md` against `specs/049-permission-system/spec.md` for requirement coverage, phase ordering, and decision soundness.

## Requirements coverage

All 12 requirements mapped to at least one phase:

| Requirement | Phase(s) | Status |
|-------------|----------|--------|
| FR-001 (canUseTool hook) | 3, 4 | Covered |
| FR-002 (layered evaluation order) | 3 (via P-003) | Covered |
| FR-003 (wildcard * and **) | 1 | Covered |
| FR-004 (prompt 3 options) | 3 | Covered |
| FR-005 (Allow & remember persistence) | 2, 3 | Covered |
| FR-006 (JSON store schema) | 2 | Covered |
| FR-007 (disallowed over allowed) | 3 (via P-003) | Covered |
| FR-008 (CLI subcommands) | 5 | Covered |
| FR-009 (non-interactive defaults) | 6 | Covered |
| NF-001 (<2ms p99 at 500 entries) | 6 | Covered |
| NF-002 (human-readable store) | 2 | Covered |
| NF-003 (deterministic matcher) | 1 | Covered |

## Success criteria coverage

All 6 success criteria assigned to validation steps:

| Criterion | Phase | Status |
|-----------|-------|--------|
| SC-001 (bypass short-circuit) | 3 | Covered |
| SC-002 (disallowed over allowed wildcard) | 3 | Covered |
| SC-003 (remember persists + suppresses prompt) | 3 | Covered |
| SC-004 (Read path ** matching) | 1 | Covered |
| SC-005 (list output) | 5 | Covered |
| SC-006 (revoke re-prompts) | 5 | Covered |

## Phase ordering assessment

Phase 1 (types + matcher) -> Phase 2 (store) -> Phase 3 (evaluator combining 1+2) -> Phase 4 (hook wiring) -> Phase 5 (CLI) -> Phase 6 (non-interactive + perf verification).

Each phase depends only on prior phases. Ordering is sound and matches the spec's own implementation approach section.

## Decision review

| Decision | Spec-faithful | Notes |
|----------|---------------|-------|
| P-001 (store paths) | Yes | Matches FR-006 paths exactly. Session in-memory is a reasonable interpretation of scope levels. |
| P-002 (canonical form) | Yes | Supports NF-002 (readable) and NF-003 (deterministic). |
| P-003 (precedence) | Yes | Matches spec Layer 1-5 order exactly. FR-007 disallowed-over-allowed explicit. |
| P-004 (target extraction) | Yes | Addresses R-003 mitigation. Reasonable heuristic for multi-value inputs. |
| P-005 (scope merge) | Yes | Session + project + global merge view with scoped mutations is spec-faithful. |
| P-006 (integration seam) | Yes | canUseTool callback matches spec architecture. Abstract prompt transport keeps UI out of scope. |

## Findings

- **F-001** (LOW): Spec store schema includes `expiresAt` and FR-008 mentions `permissions clear --expired`. Phase 2 lists `clearExpired` in deliverables but doesn't explicitly describe expiry timestamp support in the store types. Phase 1 types should include `expiresAt?: string | null` on `PermissionEntry` to avoid a Phase 2 retrofit.

- **F-002** (LOW): The colon-delimited pattern syntax (`Bash(git commit:*)`) implies a specific mapping from tool input to matchable string. P-004 mentions "primary argument string" extraction but the plan doesn't specify whether space-delimited command args become colon-delimited segments or if the colon is literal in the input. Phase 1 matcher tests should document this mapping explicitly.

- **F-003** (LOW): Phase 5 CLI lists `permissions list`, `revoke`, and `clear --expired` but the spec also mentions `permissions edit` capability (FR-008: "listed, revoked, and edited"). The plan omits an edit subcommand. This is acceptable if edit means hand-editing the JSON file (NF-002 guarantees human-readability), but should be documented as a conscious scope decision.

- **F-004** (INFO): Phase 4 mentions "wiring in governed execution/bridge path" without naming specific files. Acceptable at plan level; implementation will resolve file targets.

- **F-005** (INFO): The spec dependency on 048-hookify-rule-engine notes "permission decisions may be expressed as hook rules." The plan does not address this integration, which is reasonable — it's a future concern, not a Phase 1-6 requirement.

- **F-006** (INFO): Package structure matches spec proposal. TypeScript package at `packages/permission-system/` is consistent with 048 and 051 patterns.

## Verdict

**APPROVED for Phase 1 start.** All 12 requirements and 6 success criteria are covered across 6 phases. Phase ordering is sound. P-001 through P-006 decisions are spec-faithful. No blockers. F-001 and F-002 are LOW-severity items that Phase 1 implementation should address inline (include `expiresAt` in types, document colon-segment mapping in matcher tests). F-003 is a scope clarification, not a blocker.
