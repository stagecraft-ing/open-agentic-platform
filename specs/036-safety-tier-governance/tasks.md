---
feature_id: "036-safety-tier-governance"
---

# Tasks

- [ ] **T001** — Expand `get_tool_tier()` to explicitly classify all 21 router tools per the proposed tier table in `spec.md`. Remove the `write_file` legacy alias if unreachable.
- [ ] **T002** — Add a test that extracts tool names from the router's `tools/list` response and asserts each has an explicit entry in `get_tool_tier()` (i.e., the Tier3 catch-all is not hit for any known tool).
- [ ] **T003** — Rename `agent::safety::Tier` → `ToolTier` and update all references across `agent`, `axiomregent`, and `desktop` crates.
- [ ] **T004** — Rename `featuregraph::preflight::SafetyTier` → `ChangeTier` and update all references across `featuregraph` and `desktop` crates.
- [ ] **T005** — Extend the governance backend (`analysis.rs`) to emit a per-tool tier map (tool name → tier label) alongside the existing tier reference.
- [ ] **T006** — Update `GovernanceSurface.tsx` to display per-tool tier assignments from the new backend data.
- [ ] **T007** — Verify that `permissions.rs` `requires_file_read/write/network` functions cover all reclassified tools correctly.
- [ ] **T008** — Update `execution/verification.md` with test commands and results.
- [ ] **T009** — Run `spec-compiler compile` and verify registry includes Feature 036.
