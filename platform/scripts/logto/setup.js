import { createPool, sql } from '@silverhand/slonik';
import {
  Applications,
  ApplicationsRoles,
  Roles,
  RolesScopes,
  SignInExperiences,
  SsoConnectors,
  Resources,
  Scopes,
  Organizations,
  OrganizationRoles,
  OrganizationScopes,
  OrganizationRoleScopeRelations,
  Connectors,
  Hooks,
} from '@logto/schemas';

import { config } from './config.js';

function required(name) {
  const v = process.env[name];
  if (!v) throw new Error(`Missing required env var: ${name}`);
  return v;
}

/**
 * Convert camelCase keys to snake_case for direct SQL column usage.
 * Keeps existing snake_case keys untouched.
 */
function toSnakeKey(k) {
  if (k.includes('_')) return k;
  return k.replace(/[A-Z]/g, (m) => `_${m.toLowerCase()}`);
}
function toSnakeObject(obj) {
  const out = {};
  for (const [k, v] of Object.entries(obj)) out[toSnakeKey(k)] = v;
  return out;
}

function valueExpr(v) {
  // Slonik: use sql.json for objects/arrays; primitives as-is.
  if (v === null || v === undefined) return sql.null;
  if (Array.isArray(v) || (typeof v === 'object' && v !== null)) return sql.json(v);
  return v;
}

async function upsertBy(
  connection,
  tableName,
  row,
  conflictCols,
  updateCols
) {
  const snakeRow = toSnakeObject(row);
  const cols = Object.keys(snakeRow);
  const values = cols.map((c) => valueExpr(snakeRow[c]));

  const conflictId = sql.join(conflictCols.map((c) => sql.identifier([c])), sql`, `);

  // Build SET col = EXCLUDED.col for updateCols only
  const setFragments = updateCols.map((c) => sql.fragment`${sql.identifier([c])} = EXCLUDED.${sql.identifier([c])}`);
  const setSql = sql.join(setFragments, sql`, `);

  await connection.query(sql`
    INSERT INTO ${sql.identifier([tableName])} (${sql.join(cols.map((c) => sql.identifier([c])), sql`, `)})
    VALUES (${sql.join(values, sql`, `)})
    ON CONFLICT (${conflictId})
    DO UPDATE SET ${setSql}
  `);
}

async function insertDoNothingOnConflict(connection, tableName, row, conflictCols) {
  const snakeRow = toSnakeObject(row);
  const cols = Object.keys(snakeRow);
  const values = cols.map((c) => valueExpr(snakeRow[c]));
  const conflictId = sql.join(conflictCols.map((c) => sql.identifier([c])), sql`, `);

  await connection.query(sql`
    INSERT INTO ${sql.identifier([tableName])} (${sql.join(cols.map((c) => sql.identifier([c])), sql`, `)})
    VALUES (${sql.join(values, sql`, `)})
    ON CONFLICT (${conflictId}) DO NOTHING
  `);
}

/**
 * Special handling for Connectors table because your existing code
 * stringified config/metadata. Keep that behavior for compatibility.
 */
function normalizeConnectorRow(connector) {
  const normalized = {
    ...connector,
    config: typeof connector.config === 'string' ? connector.config : JSON.stringify(connector.config ?? {}),
    metadata: typeof connector.metadata === 'string' ? connector.metadata : JSON.stringify(connector.metadata ?? {}),
  };
  return normalized;
}

async function setupCustomLogto() {
  const dbUrl = required('DB_URL');
  const pool = await createPool(dbUrl);

  try {
    await pool.transaction(async (connection) => {
      // Applications: upsert by (tenant_id, id). Update mutable fields, keep created_at stable (don’t update it).
      for (const app of config.applications) {
        await upsertBy(
          connection,
          Applications.table,
          app,
          ['tenant_id', 'id'],
          [
            'name',
            'secret',
            'description',
            'type',
            'oidc_client_metadata',
            'custom_client_metadata',
            'protected_app_metadata',
            'is_third_party',
          ]
        );
      }
      console.log(`Upserted ${config.applications.length} applications`);

      // Roles: upsert by (tenant_id, id)
      for (const role of config.roles) {
        await upsertBy(
          connection,
          Roles.table,
          role,
          ['tenant_id', 'id'],
          ['name', 'description', 'type']
        );
      }
      console.log(`Upserted ${config.roles.length} roles`);

      // ApplicationsRoles: if this is “relation”, you can DO NOTHING on conflict to avoid churn
      for (const appRole of config.applications_roles) {
        await insertDoNothingOnConflict(
          connection,
          ApplicationsRoles.table,
          appRole,
          ['tenant_id', 'id']
        );
      }
      console.log(`Ensured ${config.applications_roles.length} application-role relations`);

      // Resources: upsert by (tenant_id, id)
      for (const resource of config.resources) {
        await upsertBy(
          connection,
          Resources.table,
          resource,
          ['tenant_id', 'id'],
          ['name', 'indicator', 'is_default', 'access_token_ttl']
        );
      }
      console.log(`Upserted ${config.resources.length} resources`);

      // RolesScopes: treat as relation; DO NOTHING is usually safest
      for (const roleScope of config.roles_scopes) {
        // If roles_scopes uses (tenant_id, id) as PK, this works.
        // If it’s composite (tenant_id, role_id, scope_id), change conflict cols accordingly.
        await insertDoNothingOnConflict(
          connection,
          RolesScopes.table,
          roleScope,
          ['tenant_id', 'id']
        );
      }
      console.log(`Ensured ${config.roles_scopes.length} role-scope relations`);

      // SignInExperiences: upsert by (tenant_id, id='default') with full drift correction
      for (const experience of config.sign_in_experiences) {
        await upsertBy(
          connection,
          SignInExperiences.table,
          experience,
          ['tenant_id', 'id'],
          [
            'color',
            'branding',
            'language_info',
            'terms_of_use_url',
            'privacy_policy_url',
            'sign_in',
            'sign_up',
            'social_sign_in_connector_targets',
            'sign_in_mode',
            'custom_css',
            'custom_content',
            'password_policy',
            'mfa',
            'single_sign_on_enabled',
          ]
        );
      }
      console.log(`Upserted ${config.sign_in_experiences.length} sign-in experiences`);

      // SSO connectors: upsert by (tenant_id, id)
      for (const connector of config.sso_connectors) {
        await upsertBy(
          connection,
          SsoConnectors.table,
          connector,
          ['tenant_id', 'id'],
          ['provider_name', 'connector_name', 'config', 'domains', 'branding', 'sync_profile', 'created_at']
        );
      }
      console.log(`Upserted ${config.sso_connectors.length} SSO connectors`);

      // Organizations: if empty, no-op; if used, upsert by (tenant_id, id)
      for (const org of config.organizations) {
        await upsertBy(
          connection,
          Organizations.table,
          org,
          ['tenant_id', 'id'],
          Object.keys(toSnakeObject(org)).filter((c) => c !== 'tenant_id' && c !== 'id' && c !== 'created_at')
        );
      }
      console.log(`Upserted ${config.organizations.length} organizations`);

      // Organization roles: upsert by (tenant_id, id)
      for (const role of config.organization_roles) {
        await upsertBy(
          connection,
          OrganizationRoles.table,
          role,
          ['tenant_id', 'id'],
          ['name', 'description']
        );
      }
      console.log(`Upserted ${config.organization_roles.length} organization roles`);

      // Organization scopes: upsert by (tenant_id, id)
      for (const scope of config.organization_scopes) {
        await upsertBy(
          connection,
          OrganizationScopes.table,
          scope,
          ['tenant_id', 'id'],
          ['name', 'description']
        );
      }
      console.log(`Upserted ${config.organization_scopes.length} organization scopes`);

      // Organization role-scope relations: composite uniqueness is likely (tenant_id, organization_role_id, organization_scope_id)
      for (const relation of config.organization_role_scope_relations) {
        await insertDoNothingOnConflict(
          connection,
          OrganizationRoleScopeRelations.table,
          relation,
          ['tenant_id', 'organization_role_id', 'organization_scope_id']
        );
      }
      console.log(`Ensured ${config.organization_role_scope_relations.length} organization role-scope relations`);

      // Connectors: upsert by (tenant_id, id) with normalization
      for (const connector of config.connectors) {
        const normalized = normalizeConnectorRow(connector);
        await upsertBy(
          connection,
          Connectors.table,
          normalized,
          ['tenant_id', 'id'],
          ['connector_id', 'config', 'sync_profile', 'metadata']
        );
      }
      console.log(`Upserted ${config.connectors.length} connectors`);

      // Webhooks (Hooks): upsert by (tenant_id, id)
      for (const webhook of config.webhooks) {
        await upsertBy(
          connection,
          Hooks.table,
          webhook,
          ['tenant_id', 'id'],
          ['name', 'events', 'signing_key', 'config', 'enabled']
        );
      }
      console.log(`Upserted ${config.webhooks.length} webhooks`);
    });

    console.log('Custom setup completed successfully');
  } finally {
    await pool.end();
  }
}

export { setupCustomLogto };
