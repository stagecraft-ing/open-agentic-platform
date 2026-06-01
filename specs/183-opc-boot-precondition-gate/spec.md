---
id: "183-opc-boot-precondition-gate"
slug: opc-boot-precondition-gate
title: "OPC boot precondition gate — sidecar liveness + materialised org session, with precondition-loss restore"
status: approved
implementation: complete
owner: bart
created: "2026-05-25"
kind: governance
domain: opc
risk: medium
depends_on:
  - "032"  # opc-inspect-governance-wiring-mvp (the cockpit this spec gates entry to)
  - "073"  # axiomregent-unification (the sidecar this spec asserts liveness of)
  - "087"  # unified-workspace-architecture (the duplex stream whose `sync.hello` is the org-session verification anchor)
  - "106"  # rauthy-native-oidc-and-membership (the OIDC identity layer whose org claim materialises into `StagecraftState.org_id`)
  - "112"  # factory-project-lifecycle (the `project.catalog.snapshot.complete` envelope is one of the org-session-verified post-handshake snapshots)
  - "133"  # amends-aware-coupling-gate (satisfaction predicate this spec rides under)
  - "147"  # spec-kind-grammar (`kind: governance`)
  - "180"  # opc-shell-codification (broad OPC shell authority; this spec sits on its Tier 1 invariant surface for a runtime-precondition concern)
code_aliases:
  - "OPC_BOOT_PRECONDITION_GATE"
extends:
  # Mechanical featuregraph-golden refresh when this spec's lifecycle
  # flipped to status: approved + implementation: complete. The
  # fixture's spec-183 record reflects this spec's frontmatter 1:1;
  # no semantic change to spec 034's claims (same precedent as
  # specs 167/168/169 carried during the 178 rename). Required by
  # spec 177 ci-orchestrator-pr-gate atomicity contract — the
  # featuregraph-golden check is a ci-gate, so lifecycle flips
  # must carry their fixture refresh inside the same PR.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "boot-state-precondition-discipline"
    unit: { kind: file, path: product/apps/opc/src/App.tsx }
    refines_specs: ["180-opc-shell-codification"]
  - aspect: "sidecar-liveness-observation"
    unit: { kind: file, path: product/apps/opc/src-tauri/src/sidecars.rs }
    refines_specs: ["073-axiomregent-unification"]
  - aspect: "org-session-materialisation-observation"
    unit: { kind: file, path: product/apps/opc/src-tauri/src/commands/stagecraft_client.rs }
    refines_specs: ["087-unified-workspace-architecture"]
  - aspect: "boot-gate-auth-callback-coupling"
    unit: { kind: file, path: product/apps/opc/src/contexts/AuthContext.tsx }
    refines_specs: ["106-rauthy-native-oidc-and-membership"]
  - aspect: "sync-hello-observer-and-duplex-give-up-signal"
    unit: { kind: file, path: product/apps/opc/src-tauri/src/commands/sync_client.rs }
    refines_specs: ["110-stagecraft-to-opc-factory-trigger"]
  - aspect: "boot-gate-command-registration-and-quit-handler"
    unit: { kind: file, path: product/apps/opc/src-tauri/src/lib.rs }
    refines_specs: ["180-opc-shell-codification"]
  - aspect: "boot-gate-component-affordances"
    unit: { kind: file, path: product/apps/opc/src/components/boot/BootGate.tsx }
    refines_specs: ["180-opc-shell-codification"]
  - aspect: "boot-gate-frontend-tauri-bindings"
    unit: { kind: file, path: product/apps/opc/src/lib/api.ts }
    refines_specs: ["180-opc-shell-codification"]
references:
  - role: parent-authority
    unit: { kind: file, path: specs/180-opc-shell-codification/spec.md }
summary: >
  Codifies the runtime preconditions that gate the OPC boot→cockpit
  transition and the precondition-loss semantics that keep the cockpit
  honest mid-session. The shell occupies a single observational *boot*
  state until two preconditions both green-light: the bundled
  axiomregent sidecar's probe port has been observed AND a TCP connection
  to that port succeeds (the diagnostic listener accepts the connection
  before closing it); and the desktop holds a materialised org session
  (`org_id` populated on the Rust-side `StagecraftState` AND a
  `sync.hello` envelope received from stagecraft over the duplex stream,
  which proves the org was accepted by the server for this client).
  While in boot, the only affordances are precondition status, retry
  bound to those preconditions, a logs entrypoint, and a Quit action
  that tears down spawned sidecars cleanly. Retry is user-initiated, not
  a silent reconnect loop. If either precondition is lost mid-session
  (sidecar crash, duplex disconnect with no reconnect, org session
  expiry), the shell restores to the boot state rather than running
  degraded. Non-goals are written down explicitly: no offline mode, no
  sidecar-optional mode, no "skip for now" bypass, no per-feature
  degraded paths. The gate is the constitutional answer to a 2026-05-25
  session in which Issues 3a/6/7/8/9 surfaced as five symptoms of one
  drift — the cockpit rendered before its substrate was ready.
---

# 183 — OPC boot precondition gate

## 1. Preamble

OPC is OAP's cockpit. Without the bundled axiomregent sidecar and an
authenticated org session, every cockpit surface degrades to a different
shape of failure: factory pipelines fail with `FactoryError::NoOrgId`;
semantic search, call-graph, and checkpoint UIs error out because their
MCP routing never resolves; the project catalog stays empty because the
duplex handshake never landed; the governance panel shows "probe port
not announced yet" indefinitely. Each was previously patched
per-feature, which produced the matrix this spec retires.

The constitutional claim this spec encodes is *OPC requires
axiomregent and an authenticated org session to do its job*. The
operational expression is a boot-state gate that the shell cannot
transition out of until both preconditions verifiably hold, plus the
mid-session symmetric rule that the cockpit returns to boot on
precondition loss. Together those make boot the only "preconditions
not satisfied" state the app can occupy. Per-feature degraded modes
become inexpressible by construction; they don't need policing.

The gate is not a wall. The boot screen surfaces live status,
explicit retry controls, and a logs entrypoint so a misconfigured
proxy or a sidecar startup race is diagnosable rather than terminal.
A genuinely unrecoverable failure mode still has a Quit affordance
that cleans up the spawned sidecar before exit so the next launch
doesn't inherit ghost processes or stale port collisions.

This spec sits on spec 180's broad OPC shell authority for a
runtime-precondition concern. Spec 180 establishes authority over the
shell directories and binds Tier 1 invariants on the tab/IPC seam;
this spec adds Tier 1 invariants on the boot→cockpit transition and
the mid-session precondition-loss path. Topically adjacent, framed
separately so 180 can land as drafted without bloating its surface.

## 2. Authority enumeration

This spec does not establish new directories. It refines existing
authority on eight files that the boot gate touches:

- `product/apps/opc/src/App.tsx` — shell entry point; gains an
  App-level component-state branch between `<BootGate>` and
  `<Cockpit>` rendering (refines spec 180's broad OPC shell
  authority).
- `product/apps/opc/src-tauri/src/sidecars.rs` — sidecar launcher;
  gains a TCP-connect liveness probe added to the existing
  port-announcement parser, plus the FR-T4 respawn helper and the
  FR-T6 Quit handler (refines spec 073's axiomregent unification
  authority). The launcher also pins a writable data directory and
  working directory on the spawned child and surfaces its stderr, so
  the bundled sidecar starts under a Finder-launched `.app` (cwd `/`)
  and a startup failure is diagnosable rather than an opaque
  "terminated code 1" (FR-T1 launch environment, below).
- `product/apps/opc/src-tauri/src/commands/stagecraft_client.rs` —
  the Rust-side org-session holder; gains a "verified org session"
  observability surface keyed on `sync.hello` receipt (refines spec
  087's duplex-stream authority).
- `product/apps/opc/src/contexts/AuthContext.tsx` — frontend auth
  context; gains a tighter coupling to the Rust-side `StagecraftState`
  org_id-materialisation moment (refines spec 106's Rauthy/OIDC
  identity authority).
- `product/apps/opc/src-tauri/src/commands/sync_client.rs` — the
  duplex consumer that observes `sync.hello` (FR-T2(b) gate-flip)
  and emits the duplex give-up signal (FR-T5(b) precondition-loss)
  (refines spec 110's stagecraft→OPC trigger authority). Its
  `ENVELOPE_SCHEMA_VERSION` MUST equal the server's (v2 per spec 119);
  a stale version makes every server frame — `sync.hello` included —
  fail the `is_server_envelope` guard, so FR-T2(b) can never flip
  (envelope-version parity, below).
- `product/apps/opc/src-tauri/src/lib.rs` — registers the boot-gate
  Tauri commands (`boot_gate_status`, `open_logs_folder`,
  `respawn_axiomregent`, `quit_opc`) into the invoke handler
  (refines spec 180's OPC shell authority).
- `product/apps/opc/src/components/boot/BootGate.tsx` — the consumer
  of the precondition state; renders the four FR-T3 affordances
  (refines spec 180's `src/components` directory establishment).
- `product/apps/opc/src/lib/api.ts` — frontend Tauri bindings for
  the boot-gate command surface (refines spec 180's OPC shell
  authority).

The directory `product/apps/opc/src/components/boot/` itself remains
under spec 180's `src/components` `establishes:` claim; spec 183 adds
file-level refinement on the BootGate component within it.

### 2.1 Why `refines:`, not `establishes:`

Each file already has a primary owner (180 for `App.tsx` via its
`src/components`/`src/lib`/etc. establishes claims; 073 for the
sidecar launcher; 087 for the duplex client; 106 for AuthContext).
This spec adds a discipline on a specific aspect of each — the
boot-precondition aspect — without displacing the primary owner.
That is precisely the shape `refines:` exists for.

## 3. Tier 1 — Structural invariants

These are boolean assertions about code shape, enforceable by lint
rule, unit test, or coupling-gate edit-trigger. Each invariant cites
the path(s) it constrains.

### 3.1 Sidecar liveness gate (FR-T1)

**FR-T1.** The OPC shell MUST observe both of the following before
transitioning out of the boot state:

(a) the bundled axiomregent sidecar's `OPC_AXIOMREGENT_PORT=<n>` line
on stderr (the probe-port announcement, already parsed by
`parse_axiomregent_port_line` in `sidecars.rs`); AND

(b) a `TcpStream::connect("127.0.0.1:<n>")` call from the desktop
process succeeds — the diagnostic listener in `axiomregent/src/main.rs`
accepts the connection (and immediately drops it; payload semantics
are intentionally not part of the contract). A `ConnectionRefused`
result MUST NOT satisfy this gate even if the port number was parsed.

> *Rationale.* The probe port is purely a liveness signal. Its
> protocol is "process is running and has bound a local TCP port" —
> there is no `/health` endpoint and no JSON-RPC `initialize` round
> trip available there (MCP traffic flows over stdio, not over the
> probe port). The minimal correct green-light test is therefore TCP
> connection establishment. A sidecar that announced and then crashed
> before completing port-bind will fail connection-establishment; a
> sidecar that announced and is still running will accept it. That is
> the binary signal the gate needs.

The probe-port connection check MAY be implemented as a single TCP
connect attempt at the moment of gate evaluation (no persistent
connection, no payload). The check MUST apply a bounded connect
timeout so a hung connect does not stall the boot-screen UI. The
timeout shape is spec-bound; the timeout value is implementer's
choice (it is a latency value, not a latency invariant — §5
explicitly excludes absolute latency budgets from this spec's
binding surface).

> *Observation, not invariant.* On a healthy local sidecar the
> connect typically completes well under a second. That is a
> descriptive expectation about the OS-level loopback path, not a
> binding budget; do not read it as a Tier 3 carve-in.

**FR-T1 launch environment (binding).** The gate observes liveness; it
cannot manufacture it. The launcher MUST therefore spawn axiomregent in
an environment where it can actually start:

- A **writable data directory** — `AXIOMREGENT_DATA_DIR` resolved to the
  OPC app-data dir (with a writable working directory). axiomregent's
  default store is `<cwd>/.axiomregent/data`
  (`crates/axiomregent/src/config/mod.rs`); a Finder-launched macOS
  `.app` inherits `cwd = /`, where `init_hiqlite` cannot create its
  store and the process exits 1 *before* binding its probe port. A
  sidecar that cannot start can never satisfy (b), so the launch
  environment is part of this gate's contract, not an implementation
  detail.
- **Surfaced stderr** — the launcher MUST log the sidecar's stderr.
  Swallowing it (parsing only the `OPC_AXIOMREGENT_PORT=` line) reduces
  every startup failure to an opaque "terminated code 1" with no cause,
  defeating the diagnosability the boot screen promises (§1).

**Files FR-T1 binds on:**
- `product/apps/opc/src-tauri/src/sidecars.rs` — the announcement
  parser is here; the TCP-connect liveness check lives here or in a
  sibling file under `src-tauri/src/`.
- `product/apps/opc/src/components/boot/` (to be created) — the
  consumer of the liveness state.

### 3.2 Org-session gate (FR-T2)

**FR-T2.** The OPC shell MUST observe both of the following before
transitioning out of the boot state:

(a) `StagecraftState.org_id` is populated with a non-empty value on
the Rust side
(`product/apps/opc/src-tauri/src/commands/stagecraft_client.rs`); AND

(b) the duplex client has received a `sync.hello` envelope from
stagecraft for the current session (the envelope kind is owned by
spec 087; receipt is observable in
`product/apps/opc/src-tauri/src/commands/sync_client.rs`'s
`run_duplex_session`). `sync.hello` is the server's acknowledgment
that the handshake was accepted for the claimed `(clientId, orgId)`
pair — its receipt is the green-light signal that the org claim has
been *accepted by stagecraft*, not merely set locally.

`is_some()` alone is insufficient: a stale, fabricated, or
not-yet-acknowledged `org_id` would pass the local-presence test
while still failing the org-scoped requests the cockpit depends on.
The `sync.hello` round trip is the cheapest verification that closes
that gap because it rides the same channel every subsequent cockpit
feature uses.

> *Rationale.* The per-feature sign-in CTA landed in commit `f9bdad30`
> is a stopgap for one symptom of this gate's absence: the user
> reaches the cockpit, clicks Start Pipeline, and the Rust factory
> context returns `FactoryError::NoOrgId`. Asserting the org session
> at boot moves the failure mode to the right place (the gate itself,
> not the feature surface) and lets the per-feature CTA be retired.

**FR-T2(b) envelope-version parity (binding).** Receipt of `sync.hello`
presupposes the desktop *accepts* the frame. The duplex consumer's
`is_server_envelope` guard enforces strict equality between the desktop's
`ENVELOPE_SCHEMA_VERSION` and the server's; spec 119 set the wire to **v2**
when it collapsed the session key from `workspace` to `org`. A desktop
pinned to a stale version rejects every server frame — `sync.hello`
included — so (b) can never flip and the gate stays closed even on a
fully authenticated, connected socket. Envelope-version parity with the
deployed server is therefore a precondition of FR-T2(b), not a separate
concern. (The desktop constant lagged at 1 while the server moved to 2;
spec 183's gate is what turned that latent skew — previously a silent
dropped frame — into a hard, observable boot block.)

**FR-T2(b) token liveness (binding, amended 2026-06-01).** Receipt of
`sync.hello` also presupposes the duplex upgrade is *authorized*. The consumer
attaches a Rauthy bearer JWT to the WebSocket handshake; stagecraft's Encore
gateway rejects a missing/expired/invalid token with HTTP 401 *before* the
socket opens, so (b) can never flip. The duplex consumer therefore resolves
its bearer at connect time from the shared OS keychain and, on a 401, drives a
silent Rauthy refresh (`StagecraftClient::refresh_jwt`) before retrying —
recovering an expired access token without user action as long as the refresh
token is still valid. It also spawns whenever a Stagecraft base URL is
configured (idling until sign-in materialises a session) rather than requiring
a token at launch. Before this, an expired access token loaded from the
keychain wedged the org-session gate in an unrecoverable `401 → backoff → 401`
loop with `sync.hello` never received — the precise hang this gate made
observable. Sites: `sync_client.rs::run_forever` / `connect_and_run`, `lib.rs`
(consumer spawn), `stagecraft_client.rs::refresh_jwt`.

**Files FR-T2 binds on:**
- `product/apps/opc/src-tauri/src/commands/stagecraft_client.rs` —
  org_id residence + the verified-receipt flag.
- `product/apps/opc/src-tauri/src/commands/sync_client.rs` — the
  `sync.hello` observer that flips the verified-receipt flag.
- `product/apps/opc/src/contexts/AuthContext.tsx` — the frontend
  observer; reads via a Tauri command rather than re-deriving from
  OAuth state.
- `product/apps/opc/src/components/boot/` (to be created) — the
  consumer of the org-session state.

### 3.3 Observational boot state (FR-T3)

**FR-T3.** While in the boot state, the shell MUST render ONLY a
boot-state UI surface (the `<BootGate>` component) whose affordances
are restricted to:

- precondition status (sidecar status row; sign-in / org-session
  status row);
- retry controls bound to those preconditions (Retry sidecar; Sign
  in to stagecraft);
- a logs entrypoint (Open logs);
- a Quit action.

No tabs, no recent-projects list, no settings panel beyond what the
precondition controls require, no "skip for now" control, no
degraded-mode entry. State change in the boot surface MUST be
confined to the `<BootGate>` component subtree; the rest of the tab
system, store layer, and route tree MUST NOT mount or read state
from within boot.

**Implementation shape (binding):** boot vs. cockpit MUST be an
App-level component-state branch in `src/App.tsx` (i.e.,
`<BootGate>` OR `<Cockpit>` rendered as an outer conditional), NOT
a route under `src/routes/` (e.g., `/boot`). A route-driven boot
state cannot structurally enforce "the rest of the route tree is
not observable from within boot," because the route tree must
exist to host the boot route alongside the rest. Only an outer
conditional render achieves the invariant.

> *Rationale.* The constitutional claim is "boot is the only state
> the app can occupy until preconditions pass." Letting the boot
> state share UI with the cockpit (recent projects, settings tweaks)
> erodes the invariant by surface area. Constrain boot to a single
> observational surface so a future "lightweight OPC" proposal must
> amend an invariant, not just merge a flag.

**Files FR-T3 binds on:**
- `product/apps/opc/src/App.tsx` — the conditional render boundary.
- `product/apps/opc/src/components/boot/` (to be created) — the
  `<BootGate>` component subtree; this directory MUST NOT mount any
  cockpit surface (no `TabManager`, no `ProjectList`, no
  `MCPManager`, no `Settings`, etc.) directly or transitively.

### 3.4 Bounded explicit retry (FR-T4)

**FR-T4.** Boot-state retry of a failed precondition MUST be
user-initiated (button click) rather than a silent reconnect loop.
Retry attempts MAY surface inline progress (e.g., `attempt 3,
waiting for port announce…`) but MUST NOT cycle without user
action. After N retry failures (implementer's choice; not
spec-bound) the boot screen MUST surface the logs entrypoint more
prominently; it MUST NOT escalate to forced Quit.

> *Rationale.* Inside a running app, silent retry is a conservative
> recovery posture (this is what the cockpit's auto-poll in commit
> `993de5ae` does, and that is the right shape *there*). At boot,
> silent retry creates a confusing wall — the user doesn't know
> whether to wait, restart, or check logs. Explicit retry teaches
> the failure-mode pattern (boot blocked → user acts) and matches
> the Government-of-Alberta-class real failure modes (corporate
> proxy ate the request, port collision, ghost process) where the
> user is the right authority on whether to give up.

**Files FR-T4 binds on:**
- `product/apps/opc/src/components/boot/` (to be created) — the
  retry-button affordances and the inline-status surface.

### 3.5 Precondition-loss restore to boot (FR-T5)

**FR-T5.** If, after a successful boot→cockpit transition, either
precondition is observed lost, the shell MUST restore to the boot
state. The observable losses are:

(a) the bundled axiomregent sidecar terminates (the desktop sees
`CommandEvent::Terminated` from the shell sidecar handle in
`sidecars.rs`) — this lapses FR-T1;

(b) the duplex client reports persistent reconnect failure. The
desktop's `run_forever` reconnect loop in `sync_client.rs` retries
without an intrinsic exhaustion bound, so this invariant requires
that the loop expose a "give-up signal" — a state transition the
implementer wires when reconnect attempts exceed an implementer-
chosen threshold (count, elapsed time, or backoff ceiling reached;
the threshold value is not spec-bound). The boot-gate consumer
subscribes to that signal. A single disconnect that the reconnect
loop recovers from before the threshold MUST NOT trigger restore;
crossing the threshold MUST. This lapses FR-T2(b);

(c) `StagecraftState.org_id` is cleared (e.g., explicit logout, or
the auth refresh path determines the session is irrecoverable) —
this lapses FR-T2(a).

Restore semantics: the cockpit subtree unmounts, the `<BootGate>`
subtree mounts in its place with retry affordances appropriate to
the lost precondition pre-selected, and any in-flight cockpit
operations (factory runs, MCP calls, etc.) MUST be cancelled or
allowed to error out into the user surface — they MUST NOT continue
silently behind the boot screen.

> *Rationale.* Without FR-T5, the spec is one-shot at startup and
> silent on the steady state. A sidecar crash mid-session would leave
> the cockpit running in exactly the degraded state the spec exists
> to forbid, and per-feature code would re-acquire the matrix of
> ad-hoc fallbacks. FR-T5 closes that seam: the boot state is the
> *only* "preconditions not satisfied" state, full-stop, regardless
> of whether the cockpit ever rendered.

**Files FR-T5 binds on:**
- `product/apps/opc/src-tauri/src/sidecars.rs` — sidecar termination
  observer (CommandEvent::Terminated propagation to the boot-gate
  consumer).
- `product/apps/opc/src-tauri/src/commands/sync_client.rs` — the
  reconnect-window-exceeded signal.
- `product/apps/opc/src-tauri/src/commands/stagecraft_client.rs` —
  org_id-cleared observer.
- `product/apps/opc/src/App.tsx` — the conditional render boundary
  that flips back to `<BootGate>`.

### 3.6 Quit affordance — clean sidecar teardown (FR-T6)

**FR-T6.** When the user invokes Quit from the boot state (or the
restored-to-boot state under FR-T5), the shell MUST tear down all
spawned sidecars before the desktop process exits. A graceful SIGTERM
attempt with a short timeout (implementer's choice, e.g. 2s), followed
by SIGKILL on timeout, is acceptable. The Quit handler MUST NOT
return until the sidecar handle reports termination (or the timeout
elapses); it MUST NOT leave the process tree in a state where the
next OPC launch could collide on the same probe port or inherit a
stale lockfile.

> *Rationale.* A Quit that orphans the axiomregent process is a
> footgun on the next launch — port collisions, stale lockfiles,
> ghost processes that hold open files the new sidecar wants. A
> single line of teardown discipline forecloses an entire class of
> "OPC won't start after I quit it" bug reports.

**Files FR-T6 binds on:**
- `product/apps/opc/src-tauri/src/sidecars.rs` — the teardown
  helper (must hold the sidecar `Child` handle or its equivalent
  long enough to invoke kill on Quit; today `spawn_axiomregent`
  drops the child after parsing the port line, which is a
  precondition-restore bug independent of this spec but blocking
  for FR-T6).
- `product/apps/opc/src-tauri/src/lib.rs` — the Quit handler /
  app-exit hook that calls the teardown helper.

## 4. Non-goals (binding)

These are written down to constrain future drift. A change that
contradicts any of them amends the corresponding invariant, not the
implementation.

- **No offline mode.** OPC has no mode in which it operates without
  a reachable axiomregent sidecar AND a materialised org session.
  A future "browse local projects without signing in" proposal
  amends FR-T2; it does not add a flag.
- **No sidecar-optional mode.** The bundled sidecar is a hard
  dependency. An external axiomregent instance MAY substitute for
  it (the gate checks the announced port responds, not which
  process produced the announcement), but zero-axiomregent
  operation is not a supported mode.
- **No "skip for now" boot bypass.** The shell MUST NOT expose a
  control that transitions out of boot without satisfying FR-T1 +
  FR-T2. A future demo-mode proposal amends FR-T3; it does not add
  a flag.
- **No degraded operation.** Cockpit features (factory, semantic
  search, call graph, checkpoint, MCP routing) MUST NOT carry their
  own "is axiomregent available?" or "is org session live?"
  fallback paths. The boot gate already asserts availability via
  FR-T1/T2; FR-T5 enforces the symmetric mid-session invariant.
  Per-feature fallback would reintroduce the matrix of degraded
  states this spec exists to remove.
- **No silent boot retry.** Per FR-T4, retry is user-initiated. A
  future auto-retry-at-boot proposal amends FR-T4; it does not add
  a poll interval to the boot gate.

## 5. Tier 3 exclusion

Absolute latency budgets for the boot→cockpit transition ("must
complete in <X seconds") are explicitly out of scope. Sidecar
startup time varies with cold-launch I/O pressure, anti-virus scan
on the bundled binary, and the user's existing process tree. The
correct boot-UX response is to surface progress (FR-T4) rather than
fail on a latency budget that flakes on slow machines.

The same reasoning excludes absolute thresholds for retry
backoff cadence — FR-T4 binds shape (user-initiated, no silent
loop), not timing.

## 6. Acceptance

- **AC-1.** Spec frontmatter declares `kind: governance`, `domain:
  opc`. The relationship-graph fields (`refines`, `references`,
  `depends_on`) are populated per §2. `code_aliases` includes
  `OPC_BOOT_PRECONDITION_GATE`.
- **AC-2.** `spec-lint` does not regress; **V-020** does not fire on
  this spec (every relationship-graph field is explicitly declared).
- **AC-3.** `make pr-prep` exits clean against `origin/main` with
  this spec.md as the sole new authored artifact, the regenerated
  `.derived/codebase-index/index.json`, AND — exempted from "no
  substantive edits to other specs' bodies" — forward-edge additions
  to an upstream spec's §8 Future Work section that point at this
  spec (specifically the `[[opc-boot-precondition-gate]]` pointer
  landed in spec 180 §8). Forward-edge pointers are not substantive
  edits to the upstream spec's design; they are discoverability
  affordances. No `oap.spec` manifest changes; no edits to upstream
  spec bodies outside §8 Future Work.
- **AC-4.** `registry-consumer by-authority` returns
  `183-opc-boot-precondition-gate` for each path this spec claims a
  `refines:` aspect on:

  ```bash
  registry-consumer by-authority product/apps/opc/src/App.tsx
  registry-consumer by-authority product/apps/opc/src-tauri/src/sidecars.rs
  registry-consumer by-authority product/apps/opc/src-tauri/src/commands/stagecraft_client.rs
  registry-consumer by-authority product/apps/opc/src/contexts/AuthContext.tsx
  ```

  Each query returns this spec alongside the primary owner declared
  by the upstream spec (180, 073, 087, 106).
- **AC-5.** A unit / integration test asserts FR-T1 directly: a
  test fixture that announces a port but then closes the listener
  MUST be rejected by the liveness probe; a fixture that announces
  and keeps the listener open MUST be accepted. This is the
  observable contract that distinguishes "port parsed" from "port
  parsed + still serving."
- **AC-6.** A unit / integration test asserts FR-T2(b): a
  `sync.hello` envelope must be observable before the org-session
  gate flips. Stagecraft already emits `sync.hello` on accepted
  handshake (`api/sync/duplex.ts` line 121, kind `sync.hello`); the
  test fixture exercises the desktop's observer in `sync_client.rs`.
- **AC-7.** An end-to-end assertion on a built OPC: with the
  sidecar binary present but stagecraft unreachable, the boot screen
  MUST render and remain rendered (FR-T1 passes via TCP-connect to
  the local probe; FR-T2 stalls on `sync.hello` and the boot screen
  surfaces the sign-in / connection failure). The cockpit MUST NOT
  appear under these conditions. This is the load-bearing assertion
  for the constitutional claim.
- **AC-8.** An end-to-end assertion on a built OPC: starting from a
  fully-signed-in cockpit, killing the axiomregent process MUST
  cause the cockpit to unmount and the boot screen to mount in its
  place within an implementer-bounded window (e.g., 2s of the
  `CommandEvent::Terminated` observation). This is the FR-T5
  observability test.
- **AC-9.** An end-to-end assertion: invoking Quit from the boot
  screen MUST leave no axiomregent process running on the system
  (assert by polling `ps` or its equivalent for a window after
  exit). This is the FR-T6 cleanup test.

AC-7, AC-8, AC-9 are end-to-end and may be gated as nightly /
manual rather than per-PR; AC-5 and AC-6 are unit-test-shaped and
SHOULD ride per-PR.

## 7. Out of scope (and why)

- **Boot-state UI styling beyond the four affordances.** Layout,
  colour, animation are implementer's choice. The spec binds
  *which* affordances exist, not how they look. (Earlier draft
  carried a mock; the row-label conventions noted on review will
  inform implementation but are not spec-bound.)
- **MCP routing implementation for Semantic Search / Call Graph /
  Checkpoint.** These features need an MCP stdio-framed client in
  the desktop shell to reach axiomregent's tool providers. Building
  that client is a separate spec / unit of work; this gate
  *enables* it (by guaranteeing the sidecar is alive when the
  cockpit renders) but does not specify it.
- **External-axiomregent substitution.** A future "use my own
  axiomregent" mode (point the boot gate at a non-bundled
  instance's probe port) is consistent with the non-goal "an
  external instance MAY substitute" but specifying its config
  surface is out of scope here.
- **Pre-boot diagnostics.** A future "OPC won't start at all"
  surface (before even the boot screen renders, e.g., for fatal
  bundled-binary corruption) is out of scope; the boot screen
  itself is the diagnostic surface for the failure modes this
  spec contemplates.

## 8. Future work

- `[[opc-mcp-stdio-router]]` — the desktop-side MCP stdio client
  that multiplexes UI requests through the axiomregent sidecar's
  stdin/stdout. Unblocked by this spec's FR-T1; replaces the
  hard-coded "not available in this build" errors in
  `SemanticSearchPanel.tsx`, `CallGraphPanel.tsx`,
  `useCheckpointFlow.ts`, and retires the
  `mcp_call_tool` stub at `commands/mcp.rs:91`.
- `[[opc-external-axiomregent-mode]]` — config-surface spec for
  pointing the boot gate at a non-bundled axiomregent instance,
  consistent with the §4 "external instance MAY substitute"
  non-goal carve-out.
- `[[opc-boot-telemetry]]` — should boot-gate transitions emit a
  governance event (transition time, retry count, precondition-loss
  cause), and if so where it lands.

## 9. Cross-references

- **Spec 032** — OPC inspect+governance MVP; the cockpit this spec
  gates entry into.
- **Spec 073** — axiomregent unification; the sidecar this spec
  asserts liveness of.
- **Spec 087** — unified workspace architecture; defines the duplex
  stream and the `sync.hello` envelope this spec uses as the
  org-session verification anchor.
- **Spec 106** — Rauthy native OIDC and membership; the identity
  layer whose org claim materialises into `StagecraftState.org_id`.
- **Spec 112** — factory project lifecycle; the
  `project.catalog.snapshot.complete` envelope (added 2026-05-25)
  is one of the post-handshake snapshots that ride the
  org-session-verified path FR-T2 asserts.
- **Spec 133** — amends-aware coupling gate; the satisfaction
  predicate this spec rides under for `refines:` edits.
- **Spec 147** — spec-kind grammar; `kind: governance` is the fit
  per the 132/153/180 precedent for invariant-binding governance
  specs.
- **Spec 180** — OPC shell codification; this spec sits on 180's
  broad OPC authority for a runtime-precondition concern. 180 §8
  future-work entry `[[opc-boot-precondition-gate]]` points here.
