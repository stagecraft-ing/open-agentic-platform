import { api } from "encore.dev/api";
import { db } from "../db/drizzle";
import { users, sessions } from "../db/schema";
import { eq, and, isNull, gt } from "drizzle-orm";
import { hashPassword, verifyPassword } from "./passwords";
import { newToken, hashToken } from "./sessions";

type SignupReq = { email: string; name: string; password: string };
type SigninReq = { email: string; password: string };

export interface AuthSigninResponse {
  ok: boolean;
  setCookie?: string;
  error?: string;
}

export interface AuthSignoutResponse {
  ok: boolean;
  setCookie?: string;
}

export interface SessionClaims {
  userId: string;
  email: string;
  name: string;
  role: "user" | "admin";
  kind: "user" | "admin";
}

export interface SessionResponse {
  ok: boolean;
  claims: SessionClaims | null;
}

const USER_COOKIE = "__session";
const ADMIN_COOKIE = "__admin_session";

const SESSION_TTL_MS = 14 * 24 * 60 * 60 * 1000; // 14 days

function cookieHeader(
  name: string,
  value: string,
  path: string,
  maxAgeSec: number
): string {
  const secure =
    process.env.NODE_ENV === "production" ? " Secure;" : "";
  return `${name}=${value}; Path=${path}; HttpOnly; SameSite=Lax; Max-Age=${maxAgeSec};${secure}`;
}

function clearCookieHeader(name: string, path: string): string {
  const secure =
    process.env.NODE_ENV === "production" ? " Secure;" : "";
  return `${name}=; Path=${path}; HttpOnly; SameSite=Lax; Max-Age=0;${secure}`;
}

async function issueSession(
  user: { id: string; role: "user" | "admin" },
  kind: "user" | "admin"
) {
  const token = newToken();
  const tokenHash = hashToken(token);
  const expiresAt = new Date(Date.now() + SESSION_TTL_MS);

  await db.insert(sessions).values({
    userId: user.id,
    kind,
    tokenHash,
    expiresAt,
  });

  return { token, expiresAt };
}

async function getClaimsFromToken(
  token: string,
  kind: "user" | "admin"
): Promise<SessionClaims | null> {
  const tokenHash = hashToken(token);
  const now = new Date();

  const rows = await db
    .select({
      userId: users.id,
      email: users.email,
      name: users.name,
      role: users.role,
      disabled: users.disabled,
    })
    .from(sessions)
    .innerJoin(users, eq(users.id, sessions.userId))
    .where(
      and(
        eq(sessions.tokenHash, tokenHash),
        eq(sessions.kind, kind),
        isNull(sessions.revokedAt),
        gt(sessions.expiresAt, now)
      )
    )
    .limit(1);

  const u = rows[0];
  if (!u || u.disabled) return null;

  return {
    userId: u.userId,
    email: u.email,
    name: u.name,
    role: u.role,
    kind,
  };
}

export const signup = api(
  { expose: true, method: "POST", path: "/auth/signup" },
  async (req: SignupReq): Promise<AuthSigninResponse> => {
    const passwordHash = await hashPassword(req.password);
    const email = req.email.toLowerCase();

    const bootstrapEmail = process.env.BOOTSTRAP_ADMIN_EMAIL?.toLowerCase();
    const role =
      bootstrapEmail && email === bootstrapEmail ? ("admin" as const) : ("user" as const);

    let created: { id: string; role: "user" | "admin" }[];
    try {
      created = await db
        .insert(users)
        .values({
          email,
          name: req.name,
          passwordHash,
          role,
        })
        .returning({ id: users.id, role: users.role });
    } catch (err) {
      return { ok: false, error: "Email already exists" };
    }

    const user = created[0];
    const { token } = await issueSession(
      { id: user.id, role: user.role },
      "user"
    );

    return {
      ok: true,
      setCookie: cookieHeader(
        USER_COOKIE,
        token,
        "/",
        Math.floor(SESSION_TTL_MS / 1000)
      ),
    };
  }
);

export const signin = api(
  { expose: true, method: "POST", path: "/auth/signin" },
  async (req: SigninReq): Promise<AuthSigninResponse> => {
    const found = await db
      .select({
        id: users.id,
        passwordHash: users.passwordHash,
        role: users.role,
        disabled: users.disabled,
      })
      .from(users)
      .where(eq(users.email, req.email.toLowerCase()))
      .limit(1);

    const u = found[0];
    if (!u || u.disabled) return { ok: false, error: "Invalid credentials" };

    const ok = await verifyPassword(u.passwordHash, req.password);
    if (!ok) return { ok: false, error: "Invalid credentials" };

    const { token } = await issueSession({ id: u.id, role: u.role }, "user");

    return {
      ok: true,
      setCookie: cookieHeader(
        USER_COOKIE,
        token,
        "/",
        Math.floor(SESSION_TTL_MS / 1000)
      ),
    };
  }
);

export const signout = api(
  { expose: true, method: "POST", path: "/auth/signout" },
  async (): Promise<AuthSignoutResponse> => {
    return { ok: true, setCookie: clearCookieHeader(USER_COOKIE, "/") };
  }
);

export const adminSignin = api(
  { expose: true, method: "POST", path: "/admin/auth/signin" },
  async (req: SigninReq): Promise<AuthSigninResponse> => {
    const found = await db
      .select({
        id: users.id,
        passwordHash: users.passwordHash,
        role: users.role,
        disabled: users.disabled,
      })
      .from(users)
      .where(eq(users.email, req.email.toLowerCase()))
      .limit(1);

    const u = found[0];
    if (!u || u.disabled || u.role !== "admin")
      return { ok: false, error: "Invalid credentials" };

    const ok = await verifyPassword(u.passwordHash, req.password);
    if (!ok) return { ok: false, error: "Invalid credentials" };

    const { token } = await issueSession({ id: u.id, role: u.role }, "admin");

    return {
      ok: true,
      setCookie: cookieHeader(
        ADMIN_COOKIE,
        token,
        "/admin",
        Math.floor(SESSION_TTL_MS / 1000)
      ),
    };
  }
);

export const adminSignout = api(
  { expose: true, method: "POST", path: "/admin/auth/signout" },
  async (): Promise<AuthSignoutResponse> => {
    return { ok: true, setCookie: clearCookieHeader(ADMIN_COOKIE, "/admin") };
  }
);

export const session = api(
  { expose: true, method: "POST", path: "/auth/session" },
  async (req: { token: string }): Promise<SessionResponse> => {
    const claims = await getClaimsFromToken(req.token, "user");
    return { ok: !!claims, claims };
  }
);

export const adminSession = api(
  { expose: true, method: "POST", path: "/admin/auth/session" },
  async (req: { token: string }): Promise<SessionResponse> => {
    const claims = await getClaimsFromToken(req.token, "admin");
    return { ok: !!claims, claims };
  }
);
