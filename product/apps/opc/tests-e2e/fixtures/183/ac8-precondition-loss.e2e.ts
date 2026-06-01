import { describe } from "vitest";

// Spec 183 AC-8 (migrated; spec 187 AC-9 / FR-T7). From a fully-signed-in
// cockpit, killing the axiomregent process MUST unmount the cockpit and remount
// <BootGate> within an implementer-bounded window (~2s of CommandEvent::
// Terminated observation).
//
// MANUAL-ONLY, RECORDED REASON (spec 187 FR-T6 — NOT a flake skip): this AC
// requires reaching a *signed-in cockpit* in CI. OPC sign-in is a full GitHub
// OAuth/PKCE browser flow against Rauthy (src-tauri/src/commands/auth.rs) with
// no dev bypass, and org-session-ready needs a materialised org_id. Reaching
// the cockpit headlessly needs auth-state seeding that does not exist yet.
// Tracked as follow-up [[opc-e2e-auth-state-seeding]]: a seedSignedInSession()
// helper (seed keychain "session" + persisted org_id; the mock duplex already
// accepts any bearer) plus a killProcess({name}) helper. Until that lands AC-8
// is excluded from the nightly auto-run set rather than asserted falsely.
//
// Intended shape once auth seeding lands:
//   const s = await launchOpc({ mode: "healthy" });
//   await seedSignedInSession(s);                      // <-- missing capability
//   await s.driver.waitForPhase("cockpit", { timeoutMs: 20_000 });
//   await killProcess({ name: "axiomregent" });        // SIGKILL the sidecar
//   await s.driver.waitForPhase("boot", { timeoutMs: 3_000 }); // AC-8 window
describe.skip(
  "183 AC-8 — cockpit reverts to BootGate on sidecar kill (BLOCKED: auth-state seeding, FR-T6 manual-only)",
  () => {
    // Body intentionally empty: see the recorded reason above. The follow-up
    // spec adds seedSignedInSession + killProcess and un-skips this fixture.
  },
);
