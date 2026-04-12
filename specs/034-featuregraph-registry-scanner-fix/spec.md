---
id: "034-featuregraph-registry-scanner-fix"
title: "featuregraph scanner reads compiled registry"
feature_branch: "034-featuregraph-registry-scanner-fix"
status: active
kind: platform
created: "2026-03-29"
authors:
  - "open-agentic-platform"
language: en
summary: >
  Point the featuregraph scanner and related governance inputs at `build/spec-registry/registry.json`
  (compiled by spec-compiler) instead of requiring `spec/features.yaml`, so the governance panel
  can hydrate from the same source of truth as CI and the Inspect surface.
code_aliases:
  - FEATUREGRAPH_REGISTRY
  - GOVERNANCE_ENGINE
owner: bart
risk: low
---

# Feature Specification: featuregraph registry scanner fix

## Purpose

Today `featuregraph::scanner` and related paths assume **`spec/features.yaml`** as the feature manifest. The platform’s canonical registry is **`build/spec-registry/registry.json`** produced by **`spec-compiler`**. This mismatch keeps governance surfaces in a **degraded** or partial state when only the compiled registry exists.

## Scope

### In scope

- Read feature identity / graph inputs from **`registry.json`** (or an adapter that maps registry entries into the scanner’s internal model).
- Preserve existing **preflight** and **violation** semantics where feasible; adjust only data source wiring.
- Update **desktop** `featuregraph_overview` / governance paths so they do not depend on a stale `features.yaml` for core feature listing when the registry is present.
- Document **`spec-compiler compile`** as a prerequisite for local governance dev workflows.

### Out of scope

- Rewriting the entire featuregraph algorithm (only input source changes unless required).
- **035** agent execution routing through axiomregent (separate feature).

## Requirements

- **FR-001**: When `build/spec-registry/registry.json` exists and is valid, the scanner **does not require** `spec/features.yaml` for basic feature membership checks.
- **FR-002**: When the registry is missing, behavior degrades **explicitly** (clear message), matching existing degraded patterns in `GovernanceSurface`.
- **FR-003**: No regression to **registry-consumer** contracts (029–031) beyond intentional dependency bumps.

## Success criteria

- **SC-001**: Governance load path uses registry-backed feature data on a repo that has run `spec-compiler compile`.
- **SC-002**: `execution/verification.md` records commands and results.

## Contract notes

- Registry path convention: **`build/spec-registry/registry.json`** relative to repository root (same as `spec-compiler` output).
