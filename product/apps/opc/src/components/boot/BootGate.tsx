// Spec 183 — OPC boot precondition gate.
//
// Outermost observational surface; renders until both FR-T1 (sidecar
// liveness) and FR-T2 (materialised org session + sync.hello receipt)
// green-light. Composes the sign-in affordance from AuthContext so the
// boot state remains a single surface — LoginScreen and OrgPicker
// are absorbed here for the boot phase rather than rendering as
// siblings, satisfying FR-T3's "render ONLY the boot-state UI"
// invariant.
//
// Stage B of the spec 183 implementation: observability + sign-in
// affordance. Stage C will wire FR-T5 mid-session precondition-loss
// restore and FR-T6 Quit sidecar teardown.

import React, { useCallback, useEffect, useState } from 'react';
import { Loader2, LogIn, ServerCog, ShieldCheck, AlertCircle, RefreshCw, FileText, XCircle } from 'lucide-react';
import { Button } from '@opc/ui/button';
import { Card } from '@opc/ui/card';
import { api, type BootGateStatus } from '@/lib/api';
import { useAuth } from '@/contexts/AuthContext';
import { OrgPicker } from '@/components/auth/OrgPicker';

interface BootGateProps {
  /** Called once both preconditions pass — App lifts to render the cockpit. */
  onReady: () => void;
}

// FR-T4 — explicit retry, no silent reconnect. The cadence below is the
// boot gate's *observation* cadence: a periodic read of
// boot_gate_status to refresh the displayed status rows. This is
// structurally distinct from retrying a failed connection attempt,
// which is what FR-T4 binds. To make the distinction concrete:
//
//   * boot_gate_status's sidecar arm does perform a TCP-connect probe
//     to the announced port (see sidecars::probe_port_alive). That
//     probe is *idempotent and read-only* against the diagnostic
//     listener — accept-or-refuse with no side effects on either side.
//     Re-issuing it is observation of a state machine, not a
//     reconnect attempt to a network resource.
//
//   * The org-session arm is a pure read of two flags (org_id present;
//     sync.hello received). The underlying duplex *reconnect* logic
//     lives in sync_client.rs::run_forever, runs on its own backoff,
//     and is gated by the FR-T5(b) give-up threshold — that's the
//     actual "reconnect attempt," and it is owned by Rust, not this
//     poll.
//
//   * The "Retry sidecar" button below performs an explicit observation
//     re-trigger, not a respawn (respawning is stage C+ work). The
//     button exists because FR-T4 binds *user-initiated retry of a
//     failed precondition*; clicking it forces an immediate re-poll,
//     which is the boot-gate consumer's equivalent of "I want to act on
//     the failure."
//
// Net: this useInterval is observation, not retry. FR-T4 is not in
// tension with the cadence.
const POLL_INTERVAL_MS = 1000;

type RowStatus = 'pending' | 'ok' | 'failed';

function statusLabel(s: RowStatus): string {
  return s === 'ok' ? 'ready' : s === 'failed' ? 'not ready' : 'checking…';
}

function StatusDot({ status }: { status: RowStatus }) {
  const cls =
    status === 'ok'
      ? 'bg-emerald-500'
      : status === 'failed'
        ? 'bg-amber-500'
        : 'bg-muted-foreground/40';
  return (
    <span
      aria-hidden
      className={`inline-block h-2 w-2 rounded-full ${cls}`}
    />
  );
}

export const BootGate: React.FC<BootGateProps> = ({ onReady }) => {
  const auth = useAuth();
  const [status, setStatus] = useState<BootGateStatus | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const pollOnce = useCallback(async () => {
    try {
      const next = await api.bootGateStatus();
      setStatus(next);
      setError(null);
      return next;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return null;
    }
  }, []);

  // Polling loop. FR-T4 framing: this is a status re-observation, not a
  // retry. Clicking Retry / Sign in are the only ways to *act on* a
  // failed precondition; the poll just keeps the UI honest about state.
  useEffect(() => {
    let mounted = true;
    void (async () => {
      while (mounted) {
        const next = await pollOnce();
        if (!mounted) return;
        setAttempt((a) => a + 1);
        if (next && next.sidecar_alive && next.org_session_ready) {
          onReady();
          return;
        }
        await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
      }
    })();
    return () => { mounted = false; };
  }, [onReady, pollOnce]);

  // AC-7 load-bearing test surface: if the sidecar is alive but stagecraft
  // is unreachable (no sync.hello yet), the BootGate stays mounted with an
  // accurate per-precondition state. The cockpit cannot render.

  const sidecarStatus: RowStatus =
    status == null ? 'pending' : status.sidecar_alive ? 'ok' : 'failed';

  const orgStatus: RowStatus =
    status == null
      ? 'pending'
      : status.org_session_ready
        ? 'ok'
        : 'failed';

  const handleRetrySidecar = useCallback(() => {
    // Stage C will wire a Tauri command that signals the sidecar
    // launcher to respawn. For now, "Retry" forces an immediate
    // re-poll — useful when the user can see the sidecar process is
    // alive in another tool and just wants the UI to re-evaluate.
    void pollOnce();
  }, [pollOnce]);

  const handleSignIn = useCallback(() => {
    void auth.login();
  }, [auth]);

  const handleQuit = useCallback(async () => {
    // FR-T6 sidecar teardown is stage C — `quit_opc` currently calls
    // app.exit(0) directly (the spawned axiomregent process is reaped
    // by the OS when the parent exits, but that's not the deterministic
    // SIGTERM-with-timeout pattern FR-T6 ultimately binds). The
    // frontend surface is stable; the implementation behind it
    // tightens in stage C.
    if (window.__TAURI_INTERNALS__ || window.__TAURI__) {
      try {
        await api.quitOpc();
      } catch {
        window.close();
      }
    } else {
      window.close();
    }
  }, []);

  const handleOpenLogs = useCallback(async () => {
    // Stage C: Tauri command that opens the OS log folder (~/Library/Logs/opc
    // on macOS, %APPDATA%\opc on Windows, ~/.local/share/opc on Linux).
    // Stub for now — see ~/Library/Logs/opc/ manually.
    console.warn('Open logs: not yet wired (spec 183 stage C).');
  }, []);

  // Org-selection: when AuthContext signals 'org-selection', surface the
  // OrgPicker inline so the user can complete sign-in without leaving
  // the boot surface. This keeps boot a single observational surface
  // per FR-T3.
  if (auth.status === 'org-selection') {
    return (
      <div className="h-full flex items-center justify-center p-6">
        <OrgPicker />
      </div>
    );
  }

  return (
    <div className="h-full flex items-center justify-center p-6 text-foreground bg-background">
      <Card className="w-full max-w-md p-6 space-y-5">
        <header className="space-y-1">
          <h1 className="text-xl font-semibold">Preparing OPC</h1>
          <p className="text-xs text-muted-foreground">
            OPC requires its bundled agent sidecar and a signed-in stagecraft
            session before the cockpit will open.
          </p>
        </header>

        {/* Sidecar row */}
        <div className="rounded-md border border-border/60 p-3 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 min-w-0">
              <ServerCog className="h-4 w-4 text-muted-foreground shrink-0" />
              <span className="text-sm font-medium">Sidecar</span>
              <span className="text-xs text-muted-foreground font-mono">
                axiomregent
              </span>
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              <StatusDot status={sidecarStatus} />
              <span className="text-xs text-muted-foreground">
                {statusLabel(sidecarStatus)}
              </span>
            </div>
          </div>
          {status?.axiomregent_port != null && (
            <p className="text-[11px] text-muted-foreground/80 font-mono">
              probe port {status.axiomregent_port}
            </p>
          )}
          {sidecarStatus === 'failed' && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs w-full gap-1.5"
              onClick={handleRetrySidecar}
            >
              <RefreshCw className="h-3 w-3" />
              Retry sidecar
            </Button>
          )}
        </div>

        {/* Org-session row */}
        <div className="rounded-md border border-border/60 p-3 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 min-w-0">
              <ShieldCheck className="h-4 w-4 text-muted-foreground shrink-0" />
              <span className="text-sm font-medium">Stagecraft</span>
              <span className="text-xs text-muted-foreground">
                {auth.status === 'authenticated'
                  ? auth.org?.org_slug
                    ? `org ${auth.org.org_slug}`
                    : 'signed in'
                  : 'not signed in'}
              </span>
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              <StatusDot status={orgStatus} />
              <span className="text-xs text-muted-foreground">
                {statusLabel(orgStatus)}
              </span>
            </div>
          </div>
          {auth.status === 'authenticated' && !status?.org_session_ready && (
            <p className="text-[11px] text-muted-foreground/80">
              Waiting for stagecraft duplex handshake (sync.hello)…
            </p>
          )}
          {auth.status === 'unauthenticated' && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs w-full gap-1.5"
              onClick={handleSignIn}
            >
              <LogIn className="h-3 w-3" />
              Sign in to stagecraft
            </Button>
          )}
          {auth.status === 'loading' && (
            <p className="text-[11px] text-muted-foreground/80 flex items-center gap-1.5">
              <Loader2 className="h-3 w-3 animate-spin" />
              Signing in…
            </p>
          )}
          {auth.error && (
            <p className="text-[11px] text-destructive">
              {auth.error}
            </p>
          )}
        </div>

        {error && (
          <p className="text-xs text-destructive flex items-center gap-1.5" role="alert">
            <AlertCircle className="h-3.5 w-3.5" />
            {error}
          </p>
        )}

        {/* Footer actions */}
        <div className="flex items-center justify-between gap-2 pt-1">
          <Button
            size="sm"
            variant="ghost"
            className="h-7 text-xs gap-1.5"
            onClick={() => { void handleOpenLogs(); }}
          >
            <FileText className="h-3 w-3" />
            Open logs
          </Button>
          <p className="text-[10px] text-muted-foreground/60 font-mono">
            poll #{attempt}
          </p>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 text-xs gap-1.5 text-destructive hover:text-destructive"
            onClick={() => { void handleQuit(); }}
          >
            <XCircle className="h-3 w-3" />
            Quit OPC
          </Button>
        </div>
      </Card>
    </div>
  );
};

export default BootGate;
