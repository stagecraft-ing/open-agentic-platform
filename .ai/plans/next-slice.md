# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment - not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Feature 032: COMPLETE** (T000-T013 all done, verification green)
- Next priorities: post-032 convergence work

## Ordered post-032 priority list

1. **Feature 033 - axiomregent activation (first)**
   - Promote a canonical spec for sidecar activation, MCP surfacing, and safety-tier visibility.
   - Smallest move with the highest thesis payoff: it turns dormant governed runtime infrastructure into a live platform surface.

2. **Activate axiomregent at app startup**
   - Wire `spawn_axiomregent(app)` in `lib.rs` and verify sidecar bundling/port discovery on supported targets.
   - Keep this scoped to activation and visibility only - do not yet reroute all agent execution through it.

3. **Surface axiomregent in the desktop UI**
   - Expose tool/resource discovery in MCP management UI and show governance-relevant safety tier information.
   - Goal: make governed capability visible and inspectable before expanding execution authority.

4. **Fix featuregraph scanner to read `registry.json` instead of `features.yaml`**
   - This is the highest-value parallel slice because it removes the governance panel's permanent degraded state.
   - Treat as either Feature 034 or a tightly bounded adjacent slice after 033 is promoted.

5. **Spec-govern the safety tier model**
   - `safety.rs` already defines tiers in code, but tier assignment and meaning are not yet governed by spec.
   - Promote once axiomregent activation scope is stable so enforcement and documentation do not drift apart.

6. **Route agent execution through governed authority**
   - Only after axiomregent is alive and visible.
   - This is the real display-vs-enforcement closure, but it changes execution authority and deserves its own feature.

## Fork resolution

### Chosen path: **Fork C - parallel, but sequenced**

- **Primary track:** axiomregent-first
- **Parallelizable secondary track:** scanner fix
- **Not recommended:** scanner-first as the sole next slice

### Why

- **axiomregent-first** closes the platform's biggest architectural gap: governed runtime exists but is not active.
- **scanner-first alone** improves one degraded panel, but does not change the platform's authority model.
- **parallel** is best only if the work stays split into separate promotable slices: one for activation, one for scanner repair.

## Recommended promotion set

### Promote now

- `specs/032-opc-inspect-governance-wiring-mvp/spec.md`
  - Stays **`status: active`** — delivery proven by tasks.md + verification.md. Registry enum is `draft|active|superseded|retired` only (Feature 000/003).

- `specs/033-axiomregent-activation/spec.md` (scaffolded 2026-03-29)
  - New canonical feature for:
    - sidecar activation on startup
    - sidecar bundling/runtime verification
    - MCP surface exposure in UI
    - safety tier visibility

### Promote next

- `specs/034-featuregraph-registry-scanner-fix/spec.md`
  - New feature for adapting scanner inputs from `features.yaml` to `registry.json`

- Safety-tier governance spec
  - Separate feature unless naturally absorbed into 033 after authoring review

## Scope boundaries for 033

### Include

- app startup activation
- sidecar readiness verification
- MCP/UI visibility for axiomregent
- safety tier display

### Exclude

- agent execution reroute
- permission model replacement
- titor command wiring
- scanner repair unless explicitly merged into a broader approved slice

## After promotion (canonical)

- [x] Record Feature 032 completion via tasks + verification (status stays `active`)
- [x] Create `specs/033-axiomregent-activation/` with spec/plan/tasks (scaffolded 2026-03-29)
- [ ] Claude review of 033 spec against actual code (this pass)
- [ ] Decide whether scanner fix becomes Feature 034 or a narrowly scoped follow-on slice
- [ ] Hand implementation back to Cursor once 033 spec is reviewed
