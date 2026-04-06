import { redirect } from "react-router";
import { createHmac, timingSafeEqual } from "crypto";
import {
  authSession,
  authAdminSession,
} from "./auth-api.server";

const USER_COOKIE = "__session";
const ADMIN_COOKIE = "__admin_session";

function parseCookie(header: string | null): Record<string, string> {
  if (!header) return {};
  const out: Record<string, string> = {};
  for (const part of header.split(";")) {
    const [k, ...rest] = part.trim().split("=");
    if (!k) continue;
    out[k] = decodeURIComponent(rest.join("=").trim() || "");
  }
  return out;
}

/**
 * Session claims for the new GitHub OAuth flow (spec 080).
 * The __session cookie is HMAC-signed (payload.signature format).
 */
interface GithubSessionClaims {
  userId: string;
  rauthyUserId: string | null;
  orgId: string;
  orgSlug: string;
  githubLogin: string;
  platformRole: "owner" | "admin" | "member";
  email: string;
  name: string;
  iat?: number;
}

/**
 * Verify HMAC-signed session cookie.
 * Uses the same SESSION_SECRET as the API layer (api/auth/session-crypto.ts).
 *
 * NOTE: The API layer reads SESSION_SECRET via Encore's secret() helper.
 * The web layer (React Router SSR) reads it via process.env because it cannot
 * import encore.dev/config. Both must resolve to the same value.
 * In Encore's dev server, secrets are automatically available as env vars.
 * In production, SESSION_SECRET must be set as both an Encore secret and
 * injected into the web process environment (e.g. via Helm chart envFrom).
 */
function verifySignedSession(token: string): GithubSessionClaims | null {
  const sessionSecret = process.env.SESSION_SECRET;
  if (!sessionSecret) return null;

  const dotIdx = token.lastIndexOf(".");
  if (dotIdx === -1) return null;

  const payload = token.substring(0, dotIdx);
  const sig = token.substring(dotIdx + 1);

  const expected = createHmac("sha256", sessionSecret)
    .update(payload)
    .digest("base64url");

  try {
    if (!timingSafeEqual(Buffer.from(sig), Buffer.from(expected))) {
      return null;
    }
  } catch {
    return null;
  }

  try {
    const data = JSON.parse(Buffer.from(payload, "base64url").toString());
    if (data.userId && data.orgId) return data as GithubSessionClaims;
  } catch {
    // Not a valid signed session
  }
  return null;
}

export async function requireUser(request: Request) {
  const cookies = parseCookie(request.headers.get("Cookie"));
  const token = cookies[USER_COOKIE];
  if (!token) throw redirect("/signin");

  // Try HMAC-signed GitHub session format first
  const ghSession = verifySignedSession(token);
  if (ghSession) {
    return {
      userId: ghSession.userId,
      email: ghSession.email,
      name: ghSession.name,
      role: ghSession.platformRole === "owner" ? ("admin" as const) : ("user" as const),
      kind: "user" as const,
      // New fields from spec 080
      orgId: ghSession.orgId,
      orgSlug: ghSession.orgSlug,
      githubLogin: ghSession.githubLogin,
      platformRole: ghSession.platformRole,
    };
  }

  // Fall back to legacy session validation
  const res = await authSession(request, token);
  if (!res.ok || !res.claims) throw redirect("/signin");

  return res.claims;
}

export async function requireAdmin(request: Request) {
  const cookies = parseCookie(request.headers.get("Cookie"));

  // Check signed session format — owner/admin role grants admin access
  const userToken = cookies[USER_COOKIE];
  if (userToken) {
    const ghSession = verifySignedSession(userToken);
    if (
      ghSession &&
      (ghSession.platformRole === "owner" || ghSession.platformRole === "admin")
    ) {
      return {
        userId: ghSession.userId,
        email: "",
        name: "",
        role: "admin" as const,
        kind: "admin" as const,
        orgId: ghSession.orgId,
        orgSlug: ghSession.orgSlug,
        githubLogin: ghSession.githubLogin,
        platformRole: ghSession.platformRole,
      };
    }
  }

  // Fall back to legacy admin session
  const token = cookies[ADMIN_COOKIE];
  if (!token) throw redirect("/admin/signin");

  const res = await authAdminSession(request, token);
  if (!res.ok || !res.claims || res.claims.role !== "admin") {
    throw redirect("/admin/signin");
  }

  return res.claims;
}

export function getCookieToken(
  request: Request,
  kind: "user" | "admin"
): string | undefined {
  const cookies = parseCookie(request.headers.get("Cookie"));
  return kind === "user" ? cookies[USER_COOKIE] : cookies[ADMIN_COOKIE];
}
