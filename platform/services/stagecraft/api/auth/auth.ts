/**
 * Auth endpoints — Phase 5 hardened (spec 087).
 *
 * Password-based signup/signin removed. All authentication flows through
 * GitHub OAuth → Rauthy OIDC JWT. Only signout endpoints remain.
 */

import { api } from "encore.dev/api";

export interface AuthSignoutResponse {
  ok: boolean;
  setCookie?: string;
}

const USER_COOKIE = "__session";
const ADMIN_COOKIE = "__admin_session";

function clearCookieHeader(name: string, path: string): string {
  const secure =
    process.env.NODE_ENV === "production" ? " Secure;" : "";
  return `${name}=; Path=${path}; HttpOnly; SameSite=Lax; Max-Age=0;${secure}`;
}

export const signout = api(
  { expose: true, method: "POST", path: "/auth/signout" },
  async (): Promise<AuthSignoutResponse> => {
    return { ok: true, setCookie: clearCookieHeader(USER_COOKIE, "/") };
  }
);

export const adminSignout = api(
  { expose: true, method: "POST", path: "/admin/auth/signout" },
  async (): Promise<AuthSignoutResponse> => {
    return { ok: true, setCookie: clearCookieHeader(ADMIN_COOKIE, "/admin") };
  }
);
