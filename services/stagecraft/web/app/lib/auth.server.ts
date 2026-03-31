import { redirect } from "react-router";
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

export async function requireUser(request: Request) {
  const cookies = parseCookie(request.headers.get("Cookie"));
  const token = cookies[USER_COOKIE];
  if (!token) throw redirect("/signin");

  const res = await authSession(request, token);
  if (!res.ok || !res.claims) throw redirect("/signin");

  return res.claims;
}

export async function requireAdmin(request: Request) {
  const cookies = parseCookie(request.headers.get("Cookie"));
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
