---
id: "030-registry-consumer-internal-output-exit-refactor-mvp"
title: "Registry consumer internal output/exit refactor MVP"
feature_branch: "030-registry-consumer-internal-output-exit-refactor-mvp"
status: approved
implementation: complete
kind: platform-delivery
created: "2026-03-22"
authors:
  - "open-agentic-platform"
language: en
summary: >
  Internal refactor only: centralize output/error/exit emission in helper paths
  while preserving all observable CLI behavior under existing contracts.
---

# Feature Specification: Internal output/exit refactor

## Purpose

Validate controlled-extension governance by performing a bounded internal refactor that improves maintainability without changing observable behavior.

## Requirements

- **FR-001**: Refactor `tools/registry-consumer/src/main.rs` to centralize repeated output/error/exit handling into helper functions.
- **FR-002**: Preserve exact observable behavior for stdout, stderr, exit codes, ordering, and JSON/text output.
- **FR-003**: Do not modify fixture corpus under `tools/registry-consumer/tests/fixtures/`.
- **FR-004**: All existing registry-consumer contract suites remain green without fixture updates.

## Out of scope

- New CLI flags, commands, or output formats
- Any intentional contract change
