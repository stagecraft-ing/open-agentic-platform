import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import { auditLog, users } from "../db/schema";
import { desc, eq } from "drizzle-orm";

/** Require admin or owner platform role. Throws 403 if not. */
function requireAdmin(): { userID: string; orgId: string } {
  const auth = getAuthData()!;
  if (auth.platformRole !== "admin" && auth.platformRole !== "owner") {
    throw APIError.permissionDenied("Admin access required");
  }
  return auth;
}

export type UserRow = {
  id: string;
  email: string;
  name: string;
  role: "user" | "admin";
  disabled: boolean;
  createdAt: Date;
};

export type ListUsersResponse = { users: UserRow[] };

export type SetRoleResponse = { ok: true };

export type AuditRow = {
  id: string;
  actorUserId: string;
  action: string;
  targetType: string;
  targetId: string;
  metadata: Record<string, unknown>;
  createdAt: Date;
};

export type ListAuditResponse = { events: AuditRow[] };

export const listUsers = api(
  { expose: true, auth: true, method: "GET", path: "/admin/users" },
  async (): Promise<ListUsersResponse> => {
    requireAdmin();
    const rows = await db.select({
      id: users.id,
      email: users.email,
      name: users.name,
      role: users.role,
      disabled: users.disabled,
      createdAt: users.createdAt,
    }).from(users);

    return { users: rows };
  }
);

export const setRole = api(
  { expose: true, auth: true, method: "POST", path: "/admin/users/set-role" },
  async (req: {
    userId: string;
    role: "user" | "admin";
  }): Promise<SetRoleResponse> => {
    const auth = requireAdmin();
    await db.update(users).set({ role: req.role }).where(eq(users.id, req.userId));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "user.set_role",
      targetType: "user",
      targetId: req.userId,
      metadata: { role: req.role },
    });

    return { ok: true };
  }
);

export const listAudit = api(
  { expose: true, auth: true, method: "GET", path: "/admin/audit" },
  async (): Promise<ListAuditResponse> => {
    requireAdmin();
    const rows = await db
      .select()
      .from(auditLog)
      .orderBy(desc(auditLog.createdAt))
      .limit(200);
    return {
      events: rows.map((r) => ({
        ...r,
        metadata: (r.metadata ?? {}) as Record<string, unknown>,
      })),
    };
  }
);
