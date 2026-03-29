---
feature_id: "034-featuregraph-registry-scanner-fix"
---

# Changeset

Scaffold — fill as implementation lands.

## Scope

- Registry-backed scanner per [spec.md](../spec.md).

## Touch targets (expected)

- `crates/featuregraph/src/scanner.rs` (and related modules)
- `apps/desktop/src-tauri/src/commands/analysis.rs` (if wiring changes)
- Optional: `apps/desktop/src/features/governance/` (only if contract to frontend changes)

## Verification

- See [verification.md](./verification.md)
