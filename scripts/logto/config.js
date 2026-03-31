import crypto from 'node:crypto';
import { defaultTenantId } from '@logto/schemas';

function formatDate(date) {
  const year = date.getUTCFullYear();
  const month = String(date.getUTCMonth() + 1).padStart(2, '0');
  const day = String(date.getUTCDate()).padStart(2, '0');
  const hours = String(date.getUTCHours()).padStart(2, '0');
  const minutes = String(date.getUTCMinutes()).padStart(2, '0');
  const seconds = String(date.getUTCSeconds()).padStart(2, '0');
  const microseconds = String(date.getUTCMilliseconds()).padStart(3, '0') + '000';
  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}.${microseconds}+00`;
}

const TENANT_ID = defaultTenantId || 'default';

/**
 * Deterministic, stable, lower-alnum IDs (20 chars).
 * - Uses sha256(seed) -> hex -> base36 -> slice.
 * - Stable across runs as long as `seed` is stable.
 */
function stableId(seed, length = 20) {
  const hex = crypto.createHash('sha256').update(seed).digest('hex');
  // Convert hex to BigInt and to base36 for compact lower-alnum.
  // Slice to requested length.
  const base36 = BigInt(`0x${hex}`).toString(36);
  return base36.padStart(length, '0').slice(0, length);
}

function required(name) {
  const v = process.env[name];
  if (!v) throw new Error(`Missing required env var: ${name}`);
  return v;
}

// Required (non-secret) values
const APP_NAME = required('APP_NAME');
const APP_URL = required('APP_URL');
const APP_DESCRIPTION = process.env.APP_DESCRIPTION || '';
const LOGTO_SPA_API_RESOURCE = required('LOGTO_SPA_API_RESOURCE');

// Required IDs (these should be stable and provided)
const LOGTO_SPA_CLIENT_ID = required('LOGTO_SPA_CLIENT_ID');
const LOGTO_M2M_CLIENT_ID = required('LOGTO_M2M_CLIENT_ID');

// Secrets are still read from env, but you should inject via Secret envFrom in Kubernetes
const LOGTO_SPA_CLIENT_SECRET = required('LOGTO_SPA_CLIENT_SECRET');
const LOGTO_M2M_CLIENT_SECRET = required('LOGTO_M2M_CLIENT_SECRET');
const LOGTO_SPA_API_EVENT_WEBHOOK_SIGNING_KEY = required('LOGTO_SPA_API_EVENT_WEBHOOK_SIGNING_KEY');
const LOGTO_SPA_API_EVENT_WEBHOOK_URL = required('LOGTO_SPA_API_EVENT_WEBHOOK_URL');

const now = new Date();
const createdAt = formatDate(now);

// Deterministic internal IDs
const ROLE_M2M_ID = stableId(`${TENANT_ID}:role:m2m:management-api-access`);
const RESOURCE_ID = stableId(`${TENANT_ID}:resource:${LOGTO_SPA_API_RESOURCE}`);
const WEBHOOK_ID = stableId(`${TENANT_ID}:webhook:postregister:${LOGTO_SPA_API_EVENT_WEBHOOK_URL}`);

// Organization roles + scopes are deterministic too
const ORG_ROLE_ADMIN_ID = stableId(`${TENANT_ID}:org-role:admin`);
const ORG_ROLE_EDITOR_ID = stableId(`${TENANT_ID}:org-role:editor`);
const ORG_ROLE_MEMBER_ID = stableId(`${TENANT_ID}:org-role:member`);

const ORG_SCOPE_CREATE_ID = stableId(`${TENANT_ID}:org-scope:create:resources`);
const ORG_SCOPE_READ_ID = stableId(`${TENANT_ID}:org-scope:read:resources`);
const ORG_SCOPE_EDIT_ID = stableId(`${TENANT_ID}:org-scope:edit:resources`);
const ORG_SCOPE_DELETE_ID = stableId(`${TENANT_ID}:org-scope:delete:resources`);

// Relation IDs: deterministic (instead of random)
const APP_ROLE_RELATION_ID = stableId(
  `${TENANT_ID}:applications_roles:${LOGTO_M2M_CLIENT_ID}:${ROLE_M2M_ID}`
);

export const config = {
  tenantId: TENANT_ID,

  applications: [
    {
      tenantId: TENANT_ID,
      id: LOGTO_M2M_CLIENT_ID,
      name: `${APP_NAME} hub`,
      secret: LOGTO_M2M_CLIENT_SECRET,
      description: `${APP_NAME} m2m`,
      type: 'MachineToMachine',
      oidcClientMetadata: { redirectUris: [], postLogoutRedirectUris: [] },
      customClientMetadata: {},
      protectedAppMetadata: null,
      isThirdParty: false,
      createdAt,
    },
    {
      tenantId: TENANT_ID,
      id: LOGTO_SPA_CLIENT_ID,
      name: APP_NAME,
      secret: LOGTO_SPA_CLIENT_SECRET,
      description: APP_DESCRIPTION,
      type: 'SPA',
      oidcClientMetadata: {
        redirectUris: [`${APP_URL}/callback`],
        postLogoutRedirectUris: [`${APP_URL}`],
      },
      customClientMetadata: {
        idTokenTtl: 3600,
        corsAllowedOrigins: [],
        rotateRefreshToken: true,
        refreshTokenTtlInDays: 7,
        alwaysIssueRefreshToken: false,
      },
      protectedAppMetadata: null,
      isThirdParty: false,
      createdAt,
    },
  ],

  // Connectors (optional)
  connectors:
    process.env.LOGTO_GOOGLE_CONNECTOR_ID &&
      process.env.LOGTO_GOOGLE_CLIENT_ID &&
      process.env.LOGTO_GOOGLE_CLIENT_SECRET
      ? [
        {
          tenantId: TENANT_ID,
          id: process.env.LOGTO_GOOGLE_CONNECTOR_ID,
          connectorId: 'google-universal',
          config: {
            scope: 'openid profile email',
            clientId: process.env.LOGTO_GOOGLE_CLIENT_ID,
            clientSecret: process.env.LOGTO_GOOGLE_CLIENT_SECRET,
          },
          syncProfile: false,
          metadata: {},
          createdAt,
        },
      ]
      : [],

  // SSO connectors (optional)
  sso_connectors:
    process.env.LOGTO_GOOGLE_WORKSPACE_CONNECTOR_ID &&
      process.env.LOGTO_GOOGLE_WORKSPACE_CLIENT_ID &&
      process.env.LOGTO_GOOGLE_WORKSPACE_CLIENT_SECRET
      ? [
        {
          tenantId: TENANT_ID,
          id: process.env.LOGTO_GOOGLE_WORKSPACE_CONNECTOR_ID,
          provider_name: 'GoogleWorkspace',
          connector_name: `${APP_NAME} google workspace connector`,
          config: {
            scope: 'openid profile email',
            clientId: process.env.LOGTO_GOOGLE_WORKSPACE_CLIENT_ID,
            clientSecret: process.env.LOGTO_GOOGLE_WORKSPACE_CLIENT_SECRET,
          },
          domains: JSON.parse(process.env.LOGTO_GOOGLE_WORKSPACE_CONNECTOR_APPROVED_DOMAINS || '[]'),
          branding: { displayName: `${APP_NAME} workspace` },
          sync_profile: false,
          created_at: createdAt,
        },
      ]
      : [],

  // Sign-in experience is updated/upserted by (tenantId, id='default')
  sign_in_experiences: [
    {
      tenantId: TENANT_ID,
      id: 'default',
      color: {
        primaryColor: '#0053db',
        darkPrimaryColor: '#0072f0',
        isDarkModeEnabled: true,
      },
      branding: {
        logoUrl: `${APP_URL}/logo.png`,
        darkLogoUrl: `${APP_URL}/logo-dark.png`,
      },
      language_info: { autoDetect: true, fallbackLanguage: 'en' },
      terms_of_use_url: `${APP_URL}/terms-of-service.html`,
      privacy_policy_url: `${APP_URL}/privacy-policy.html`,
      sign_in: { methods: [] },
      sign_up: { verify: false, password: false, identifiers: [] },
      social_sign_in_connector_targets: ['google'],
      sign_in_mode: 'SignInAndRegister',
      custom_css: `[aria-label*="Logto"] { display: none; }`,
      custom_content: {},
      password_policy: {
        length: { max: 256, min: 8 },
        rejects: {
          pwned: true,
          words: [],
          userInfo: true,
          repetitionAndSequence: true,
        },
        characterTypes: { min: 1 },
      },
      mfa: { policy: 'UserControlled', factors: [] },
      single_sign_on_enabled: !!process.env.LOGTO_GOOGLE_WORKSPACE_CLIENT_ID,
    },
  ],

  roles: [
    {
      tenantId: TENANT_ID,
      id: ROLE_M2M_ID,
      name: 'Management API Access',
      description: 'Role with management API access scope.',
      type: 'MachineToMachine',
    },
  ],

  applications_roles: [
    {
      tenantId: TENANT_ID,
      id: APP_ROLE_RELATION_ID,
      application_id: LOGTO_M2M_CLIENT_ID,
      role_id: ROLE_M2M_ID,
    },
  ],

  roles_scopes: [
    {
      tenantId: TENANT_ID,
      // If this table has its own PK, keep deterministic id; if composite PK, id may be ignored.
      id: stableId(`${TENANT_ID}:roles_scopes:${ROLE_M2M_ID}:management-api-all`),
      role_id: ROLE_M2M_ID,
      scope_id: 'management-api-all',
    },
  ],

  resources: [
    {
      tenantId: TENANT_ID,
      id: RESOURCE_ID,
      name: `${APP_NAME} api resource identifier`,
      indicator: LOGTO_SPA_API_RESOURCE,
      is_default: false,
      access_token_ttl: 3600,
    },
  ],

  scopes: [],
  organizations: [],

  organization_roles: [
    { tenantId: TENANT_ID, id: ORG_ROLE_ADMIN_ID, name: 'admin', description: 'Admin Role' },
    { tenantId: TENANT_ID, id: ORG_ROLE_EDITOR_ID, name: 'editor', description: 'Editor Role' },
    { tenantId: TENANT_ID, id: ORG_ROLE_MEMBER_ID, name: 'member', description: 'Member Role' },
  ],

  organization_scopes: [
    { tenantId: TENANT_ID, id: ORG_SCOPE_CREATE_ID, name: 'create:resources', description: 'Create Resources' },
    { tenantId: TENANT_ID, id: ORG_SCOPE_READ_ID, name: 'read:resources', description: 'Read Resources' },
    { tenantId: TENANT_ID, id: ORG_SCOPE_EDIT_ID, name: 'edit:resources', description: 'Edit Resources' },
    { tenantId: TENANT_ID, id: ORG_SCOPE_DELETE_ID, name: 'delete:resources', description: 'Delete Resources' },
  ],

  // Assume this table uses a composite unique key (tenant_id, organization_role_id, organization_scope_id).
  // No need for a separate id; insert with DO NOTHING on conflict.
  organization_role_scope_relations: [
    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_ADMIN_ID, organization_scope_id: ORG_SCOPE_CREATE_ID },
    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_ADMIN_ID, organization_scope_id: ORG_SCOPE_READ_ID },
    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_ADMIN_ID, organization_scope_id: ORG_SCOPE_EDIT_ID },
    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_ADMIN_ID, organization_scope_id: ORG_SCOPE_DELETE_ID },

    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_EDITOR_ID, organization_scope_id: ORG_SCOPE_CREATE_ID },
    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_EDITOR_ID, organization_scope_id: ORG_SCOPE_READ_ID },
    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_EDITOR_ID, organization_scope_id: ORG_SCOPE_EDIT_ID },

    { tenantId: TENANT_ID, organization_role_id: ORG_ROLE_MEMBER_ID, organization_scope_id: ORG_SCOPE_READ_ID },
  ],

  webhooks: [
    {
      tenantId: TENANT_ID,
      id: WEBHOOK_ID,
      name: 'On New User Create',
      events: ['PostRegister'],
      signingKey: LOGTO_SPA_API_EVENT_WEBHOOK_SIGNING_KEY,
      config: { url: LOGTO_SPA_API_EVENT_WEBHOOK_URL },
      enabled: true,
      createdAt,
    },
  ],
};
