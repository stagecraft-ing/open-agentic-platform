// Spec 137 Phase 2 — Stagecraft API for per-environment access gates.
//
// Four endpoints + audit-log integration + defense-in-depth password
// rejection (FR-007). Storage layer is spec 137 Phase 1 migration 40
// (`environment_access_gates` + `environment_access_gate_allowlist_emails`).
//
// The descriptor is 1:1 with environments — the GET returns either a
// default-disabled descriptor (no row exists yet) or the persisted row.
// PUT is a true upsert. Allowlist endpoints append / remove rows by id.
//
// Permission gate: `org:manage_members` (owner + admin in the project's
// org). Mirrors the admin-only intent in spec.md §"Explicitly in scope"
// and the §"Out of scope" carve-out for self-service flows. A tighter
// per-environment gate is a future refinement when spec 137 §Out of
// scope §"Email allowlist UX for non-administrators" gets its own spec.

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  environments,
  environmentAccessGates,
  environmentAccessGateAllowlistEmails,
  projects,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import {
  ALLOWLIST_KINDS,
  assertNoPasswordFields,
  validateFederatedProviderPair,
  type FederatedProvider,
} from "./accessGatesHelpers";

export { assertNoPasswordFields, type FederatedProvider } from "./accessGatesHelpers";

// ---------------------------------------------------------------------------
// Wire shapes
// ---------------------------------------------------------------------------

export interface AccessGateDescriptor {
  enabled: boolean;
  rauthyClientRef: string | null;
  loginMethodMagicLink: boolean;
  loginMethodFederatedProvider: FederatedProvider | null;
  loginMethodFederatedProviderClientRef: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AccessGateAllowlistEntry {
  id: string;
  kind: "email" | "domain";
  value: string;
  createdAt: string;
}

export interface AccessGateRead extends AccessGateDescriptor {
  environmentId: string;
  allowlist: AccessGateAllowlistEntry[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Resolve the environment + its owning project's org, scoped to the
 * caller's org. Returns the environment row when in-scope, throws
 * `NotFound` otherwise.
 *
 * Centralised so each endpoint enforces the same org-scoping contract;
 * spec 119 §6 collapsed workspace into project so the chain is
 * `environment → project → organization`.
 */
async function loadEnvironmentInOrg(
  environmentId: string,
  orgId: string,
): Promise<{ id: string; projectId: string }> {
  const [row] = await db
    .select({
      id: environments.id,
      projectId: environments.projectId,
    })
    .from(environments)
    .innerJoin(projects, eq(projects.id, environments.projectId))
    .where(and(eq(environments.id, environmentId), eq(projects.orgId, orgId)))
    .limit(1);
  if (!row) {
    throw APIError.notFound("environment not found in this org");
  }
  return row;
}

function defaultDisabledDescriptor(): AccessGateDescriptor {
  const now = new Date().toISOString();
  return {
    enabled: false,
    rauthyClientRef: null,
    loginMethodMagicLink: true,
    loginMethodFederatedProvider: null,
    loginMethodFederatedProviderClientRef: null,
    createdAt: now,
    updatedAt: now,
  };
}

async function fetchAllowlist(
  environmentId: string,
): Promise<AccessGateAllowlistEntry[]> {
  const rows = await db
    .select({
      id: environmentAccessGateAllowlistEmails.id,
      kind: environmentAccessGateAllowlistEmails.kind,
      value: environmentAccessGateAllowlistEmails.value,
      createdAt: environmentAccessGateAllowlistEmails.createdAt,
    })
    .from(environmentAccessGateAllowlistEmails)
    .where(eq(environmentAccessGateAllowlistEmails.environmentId, environmentId));
  return rows.map((r) => ({
    id: r.id,
    kind: r.kind as "email" | "domain",
    value: r.value,
    createdAt: r.createdAt.toISOString(),
  }));
}

async function emitAudit(
  actorUserId: string,
  action: string,
  environmentId: string,
  metadata: Record<string, unknown>,
): Promise<void> {
  await db.insert(auditLog).values({
    actorUserId,
    action,
    targetType: "environment_access_gate",
    targetId: environmentId,
    metadata,
  });
}

// ---------------------------------------------------------------------------
// GET /api/environments/:environmentId/access-gate
// ---------------------------------------------------------------------------

export interface GetAccessGateRequest {
  environmentId: string;
}

export const getAccessGate = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/environments/:environmentId/access-gate",
  },
  async (req: GetAccessGateRequest): Promise<AccessGateRead> => {
    const auth = getAuthData()!;
    const env = await loadEnvironmentInOrg(req.environmentId, auth.orgId);

    const [row] = await db
      .select()
      .from(environmentAccessGates)
      .where(eq(environmentAccessGates.environmentId, env.id))
      .limit(1);

    const allowlist = await fetchAllowlist(env.id);
    if (!row) {
      // No descriptor persisted yet — surface a default-disabled view so
      // the UI's "Access gate" card can render the off-state without a
      // separate 404 path.
      return {
        environmentId: env.id,
        ...defaultDisabledDescriptor(),
        allowlist,
      };
    }

    return {
      environmentId: env.id,
      enabled: row.enabled,
      rauthyClientRef: row.rauthyClientRef,
      loginMethodMagicLink: row.loginMethodMagicLink,
      loginMethodFederatedProvider:
        row.loginMethodFederatedProvider as FederatedProvider | null,
      loginMethodFederatedProviderClientRef:
        row.loginMethodFederatedProviderClientRef,
      createdAt: row.createdAt.toISOString(),
      updatedAt: row.updatedAt.toISOString(),
      allowlist,
    };
  },
);

// ---------------------------------------------------------------------------
// PUT /api/environments/:environmentId/access-gate
// ---------------------------------------------------------------------------

export interface PutAccessGateRequest {
  environmentId: string;
  enabled: boolean;
  rauthyClientRef?: string | null;
  loginMethodMagicLink?: boolean;
  loginMethodFederatedProvider?: FederatedProvider | null;
  loginMethodFederatedProviderClientRef?: string | null;
}

export const putAccessGate = api(
  {
    expose: true,
    auth: true,
    method: "PUT",
    path: "/api/environments/:environmentId/access-gate",
  },
  async (req: PutAccessGateRequest): Promise<AccessGateRead> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "org:manage_members")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to manage access gates",
      );
    }
    assertNoPasswordFields(req);

    const env = await loadEnvironmentInOrg(req.environmentId, auth.orgId);

    // Validate federated-provider value + pair-consistency before
    // hitting the DB so the error surface is precise. The CHECK
    // constraints would also fire, but the typed errors from the
    // helper layer are more actionable.
    const { provider: fedProvider, clientRef: fedRef } =
      validateFederatedProviderPair(
        req.loginMethodFederatedProvider,
        req.loginMethodFederatedProviderClientRef,
      );
    // Enabled requires a Rauthy client ref. The DB CHECK also enforces
    // this; surfacing it here avoids the round-trip on the common
    // mistake of toggling enabled before Phase 3's Rauthy provisioning
    // hooks land.
    if (req.enabled && !req.rauthyClientRef) {
      throw APIError.failedPrecondition(
        "enabling the gate requires rauthyClientRef (Rauthy client provisioning lands in Phase 3)",
      );
    }

    const magicLink = req.loginMethodMagicLink ?? true;
    const now = new Date();

    // Upsert via ON CONFLICT — the descriptor is 1:1 keyed on
    // environment_id so this resolves to insert-or-update on the same
    // row deterministically.
    const inserted = await db
      .insert(environmentAccessGates)
      .values({
        environmentId: env.id,
        enabled: req.enabled,
        rauthyClientRef: req.rauthyClientRef ?? null,
        loginMethodMagicLink: magicLink,
        loginMethodFederatedProvider: fedProvider,
        loginMethodFederatedProviderClientRef: fedRef,
        createdAt: now,
        updatedAt: now,
      })
      .onConflictDoUpdate({
        target: environmentAccessGates.environmentId,
        set: {
          enabled: req.enabled,
          rauthyClientRef: req.rauthyClientRef ?? null,
          loginMethodMagicLink: magicLink,
          loginMethodFederatedProvider: fedProvider,
          loginMethodFederatedProviderClientRef: fedRef,
          updatedAt: now,
        },
      })
      .returning();

    const row = inserted[0]!;
    await emitAudit(
      auth.userId,
      req.enabled
        ? "tenant.gate.descriptor.enabled"
        : "tenant.gate.descriptor.disabled",
      env.id,
      {
        enabled: req.enabled,
        rauthyClientRef: req.rauthyClientRef ?? null,
        loginMethodMagicLink: magicLink,
        loginMethodFederatedProvider: fedProvider,
      },
    );

    const allowlist = await fetchAllowlist(env.id);
    return {
      environmentId: env.id,
      enabled: row.enabled,
      rauthyClientRef: row.rauthyClientRef,
      loginMethodMagicLink: row.loginMethodMagicLink,
      loginMethodFederatedProvider:
        row.loginMethodFederatedProvider as FederatedProvider | null,
      loginMethodFederatedProviderClientRef:
        row.loginMethodFederatedProviderClientRef,
      createdAt: row.createdAt.toISOString(),
      updatedAt: row.updatedAt.toISOString(),
      allowlist,
    };
  },
);

// ---------------------------------------------------------------------------
// POST /api/environments/:environmentId/access-gate/allowlist
// ---------------------------------------------------------------------------

export interface AddAllowlistEntryRequest {
  environmentId: string;
  kind: "email" | "domain";
  value: string;
}

export interface AddAllowlistEntryResponse {
  id: string;
  kind: "email" | "domain";
  value: string;
  createdAt: string;
}

export const addAllowlistEntry = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/environments/:environmentId/access-gate/allowlist",
  },
  async (req: AddAllowlistEntryRequest): Promise<AddAllowlistEntryResponse> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "org:manage_members")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to manage access gate allowlist",
      );
    }
    assertNoPasswordFields(req);

    const env = await loadEnvironmentInOrg(req.environmentId, auth.orgId);

    if (!ALLOWLIST_KINDS.has(req.kind)) {
      throw APIError.invalidArgument("kind must be 'email' or 'domain'");
    }
    const value = req.value?.trim() ?? "";
    if (!value) {
      throw APIError.invalidArgument("value must be a non-empty string");
    }
    // Normalise to lowercase on write; the unique index uses
    // `lower(value)` so case-mismatched duplicates would collide there,
    // but storing lowercased values keeps the read path predictable too.
    const normalised = value.toLowerCase();

    let inserted;
    try {
      inserted = await db
        .insert(environmentAccessGateAllowlistEmails)
        .values({
          environmentId: env.id,
          kind: req.kind,
          value: normalised,
        })
        .returning();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("environment_access_gate_allowlist_emails_unique")) {
        throw APIError.alreadyExists(
          `allowlist entry already exists for kind='${req.kind}' value='${normalised}'`,
        );
      }
      throw err;
    }

    const row = inserted[0]!;
    await emitAudit(
      auth.userId,
      "tenant.gate.allowlist.added",
      env.id,
      { kind: req.kind, value: normalised, entryId: row.id },
    );

    return {
      id: row.id,
      kind: row.kind as "email" | "domain",
      value: row.value,
      createdAt: row.createdAt.toISOString(),
    };
  },
);

// ---------------------------------------------------------------------------
// DELETE /api/environments/:environmentId/access-gate/allowlist/:entryId
// ---------------------------------------------------------------------------

export interface RemoveAllowlistEntryRequest {
  environmentId: string;
  entryId: string;
}

export interface RemoveAllowlistEntryResponse {
  ok: true;
  removed: AccessGateAllowlistEntry | null;
}

export const removeAllowlistEntry = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/environments/:environmentId/access-gate/allowlist/:entryId",
  },
  async (
    req: RemoveAllowlistEntryRequest,
  ): Promise<RemoveAllowlistEntryResponse> => {
    const auth = getAuthData()!;
    if (!hasOrgPermission(auth.platformRole, "org:manage_members")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to manage access gate allowlist",
      );
    }
    const env = await loadEnvironmentInOrg(req.environmentId, auth.orgId);

    const deleted = await db
      .delete(environmentAccessGateAllowlistEmails)
      .where(
        and(
          eq(environmentAccessGateAllowlistEmails.id, req.entryId),
          eq(environmentAccessGateAllowlistEmails.environmentId, env.id),
        ),
      )
      .returning();

    if (deleted.length === 0) {
      // Surface absence as `removed: null` (not 404) — the caller asked
      // "make sure this entry is gone" and that state holds regardless of
      // whether the row was already absent. Mirror of FR-009-style
      // idempotent toggling.
      return { ok: true, removed: null };
    }

    const row = deleted[0]!;
    await emitAudit(
      auth.userId,
      "tenant.gate.allowlist.removed",
      env.id,
      { kind: row.kind, value: row.value, entryId: row.id },
    );

    return {
      ok: true,
      removed: {
        id: row.id,
        kind: row.kind as "email" | "domain",
        value: row.value,
        createdAt: row.createdAt.toISOString(),
      },
    };
  },
);
