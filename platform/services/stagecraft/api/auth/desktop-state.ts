/**
 * Shared state for desktop OAuth flows (spec 080 Phase 1).
 *
 * Separated to avoid circular imports between github.ts and desktop.ts.
 */
import crypto from "crypto";

export interface PendingDesktopFlow {
  codeChallenge: string;
  codeChallengeMethod: string;
  redirectUri: string;
  desktopState: string; // The state OPC sent (returned in the deep-link)
  createdAt: number;
}

export interface PendingDesktopSession {
  userId: string;
  rauthyUserId: string;
  email: string;
  name: string;
  githubLogin: string;
  avatarUrl: string;
  codeChallenge: string; // Passed through from PendingDesktopFlow for PKCE verification
  matchedOrgs: Array<{
    orgId: string;
    orgSlug: string;
    workspaceId: string;
    githubOrgLogin: string;
    platformRole: string;
  }>;
  createdAt: number;
}

// Maps keyed by the GITHUB OAuth state (used in the callback)
export const pendingDesktopFlows = new Map<string, PendingDesktopFlow>();

// Maps keyed by a one-time auth code (returned to OPC in the deep-link)
export const pendingDesktopSessions = new Map<string, PendingDesktopSession>();

const FLOW_TTL_MS = 10 * 60 * 1000; // 10 minutes
const SESSION_TTL_MS = 5 * 60 * 1000; // 5 minutes

export function cleanupDesktopState(): void {
  const now = Date.now();
  for (const [key, val] of pendingDesktopFlows) {
    if (now - val.createdAt > FLOW_TTL_MS) pendingDesktopFlows.delete(key);
  }
  for (const [key, val] of pendingDesktopSessions) {
    if (now - val.createdAt > SESSION_TTL_MS) pendingDesktopSessions.delete(key);
  }
}

/** Check whether a GitHub OAuth callback state belongs to a desktop flow. */
export function isDesktopFlow(githubState: string): boolean {
  return pendingDesktopFlows.has(githubState);
}

/** Consume a desktop flow entry (returns and deletes it). */
export function consumeDesktopFlow(githubState: string): PendingDesktopFlow | undefined {
  const flow = pendingDesktopFlows.get(githubState);
  if (flow) pendingDesktopFlows.delete(githubState);
  return flow;
}

/** Generate a one-time auth code and store the pending session. */
export function storeDesktopSession(session: PendingDesktopSession): string {
  const authCode = crypto.randomBytes(32).toString("base64url");
  pendingDesktopSessions.set(authCode, session);
  return authCode;
}

/** Consume a pending desktop session (returns and deletes it). */
export function consumeDesktopSession(authCode: string): PendingDesktopSession | undefined {
  const session = pendingDesktopSessions.get(authCode);
  if (session) pendingDesktopSessions.delete(authCode);
  return session;
}
