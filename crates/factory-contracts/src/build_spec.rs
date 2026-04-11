// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Rust types for the Factory Build Spec schema.
//!
//! A Build Spec is a tech-agnostic application specification covering project
//! identity, auth, data model, business rules, API, UI, integrations,
//! notifications, audit, security, health checks, error handling, data
//! ingestion, file storage, and traceability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Top-level Build Spec ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSpec {
    #[serde(default)]
    pub schema_version: String,
    pub project: ProjectSpec,
    pub auth: AuthSpec,
    pub data_model: DataModelSpec,
    pub business_rules: Vec<BusinessRule>,
    pub api: ApiSpec,
    pub ui: UiSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrations: Option<Vec<Integration>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notifications: Option<NotificationSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit: Option<AuditSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_checks: Option<HealthChecks>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_handling: Option<ErrorHandling>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_ingestion: Option<Vec<DataIngestion>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_storage: Option<Vec<FileStorage>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceability: Option<TraceabilitySpec>,
}

// ── Project ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSpec {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub org: String,
    pub description: String,
    pub variant: Variant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fiscal_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Variant {
    SinglePublic,
    SingleInternal,
    Dual,
}

// ── Auth ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSpec {
    /// Map of audience name (e.g. "citizen", "staff") to audience definition.
    pub audiences: HashMap<String, Audience>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_to_service: Option<ServiceToService>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audience {
    pub method: AudienceMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudienceMethod {
    Saml,
    Oidc,
    ApiKey,
    Basic,
    Mock,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub role_code: String,
    pub display_name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceToService {
    pub method: S2sMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_validation: Option<TokenValidation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_cache: Option<TokenCache>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum S2sMethod {
    ClientCredentials,
    MutualTls,
    ApiKey,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenValidation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_clients: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCache {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_buffer_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_minutes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_recent_auth_for: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistence: Option<SessionPersistence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub store_type_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionPersistence {
    None,
    Persistent,
}

// ── Data Model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataModelSpec {
    pub entities: Vec<Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Vec<Relationship>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub fields: Vec<Field>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_constraints: Option<Vec<UniqueConstraint>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check_constraints: Option<Vec<CheckConstraint>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexes: Option<Vec<Index>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_rules: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    /// Whether this field is the primary key.
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // For enum type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    // For decimal type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    // For string type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    // For reference type — flat (not nested struct)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_entity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ref_on_delete: Option<RefOnDelete>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FieldType {
    String,
    Text,
    Integer,
    Decimal,
    Boolean,
    Uuid,
    Date,
    Datetime,
    Timestamp,
    Enum,
    Json,
    Reference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RefOnDelete {
    Cascade,
    Restrict,
    SetNull,
    NoAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueConstraint {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckConstraint {
    pub name: String,
    /// Human-readable description of the constraint (not SQL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub fields: Vec<String>,
    #[serde(default)]
    pub unique: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    #[serde(rename = "type")]
    pub rel_type: RelationType,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub through: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RelationType {
    OneToOne,
    OneToMany,
    ManyToMany,
}

// ── Business Rules ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessRule {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub rule_type: BusinessRuleType,
    pub enforced_at: EnforcedAt,
    pub entities: Vec<String>,
    // State-machine fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub states: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transitions: Option<Vec<Transition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_states: Option<Vec<String>>,
    // Computation fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    // Validation fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BusinessRuleType {
    StateMachine,
    Validation,
    Computation,
    Authorization,
    Constraint,
    Privacy,
    Retention,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EnforcedAt {
    Service,
    Database,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_role: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<String>>,
}

// ── API ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_path: Option<String>,
    pub resources: Vec<ApiResource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_endpoints: Option<Vec<SystemEndpoint>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResource {
    pub name: String,
    pub entity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional parent resource for nested routes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ResourceParent>,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceParent {
    pub resource: String,
    pub param: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub method: HttpMethod,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audience: Vec<String>,
    pub auth: AuthRequirement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_roles: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<StackTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<RequestSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_rules: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cases: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_cases: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuthRequirement {
    Required,
    Optional,
    ServiceOnly,
    Public,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StackTarget {
    Public,
    Internal,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<Param>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<Vec<Param>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<RequestBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// Entity name for request body (when body is entity-derived).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// Subset of entity fields accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// Inline schema override when not entity-derived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSpec {
    #[serde(rename = "type")]
    pub response_type: ResponseType,
    /// Entity name for response body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// Subset of entity fields returned (empty = all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    /// Inline schema override when not entity-derived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ResponseType {
    Single,
    List,
    Paginated,
    Empty,
    Binary,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEndpoint {
    pub id: String,
    pub method: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub auth: AuthRequirement,
}

// ── UI ────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSpec {
    pub pages: Vec<Page>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation: Option<Navigation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub page_type: PageType,
    /// Audience name from auth section.
    pub audience: String,
    pub view_type: ViewType,
    #[serde(default)]
    pub requires_auth: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_roles: Option<Vec<String>>,
    #[serde(default)]
    pub guest_only: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_sources: Option<Vec<DataSource>>,
    /// Operation ID this form page submits to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submits_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nav_section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nav_order: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nav_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cases: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_cases: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PageType {
    Landing,
    Dashboard,
    List,
    Detail,
    Form,
    Content,
    Help,
    Profile,
    Login,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ViewType {
    Public,
    PublicAuthenticated,
    PrivateAuthenticated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub operation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<DataSourceTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DataSourceTrigger {
    OnLoad,
    OnAction,
    OnSubmit,
    OnInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Navigation {
    pub sections: Vec<NavSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavSection {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_type: Option<String>,
}

// ── Integrations ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Integration {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub integration_type: IntegrationType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_params: Option<Vec<ConfigParam>>,
    /// Cron expression or keyword (for data-ingestion type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_schedule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_rules: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum IntegrationType {
    FileStorage,
    DataIngestion,
    Email,
    IdentityProvider,
    ExternalApi,
    MessageQueue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigParam {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub required: bool,
}

// ── Notifications ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSpec {
    pub events: Vec<NotificationEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationEvent {
    pub id: String,
    pub trigger: String,
    pub recipient: String,
    pub channel: NotificationChannel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_template: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<NotificationDelivery>,
    /// Only meaningful for `async-with-retry` delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_attempts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationChannel {
    Email,
    InApp,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationDelivery {
    FireAndForget,
    AsyncWithRetry,
    Guaranteed,
}

// ── Audit ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tracked_actions: Option<Vec<TrackedAction>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention: Option<AuditRetention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_rules: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedAction {
    pub action_code: String,
    pub entities: Vec<String>,
    pub captures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRetention {
    pub policy: String,
    #[serde(default)]
    pub immutable: bool,
}

// ── Security ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limiting: Option<Vec<RateLimitTier>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cors: Option<CorsSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_validation: Option<HostValidation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csp: Option<CspSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csrf: Option<CsrfSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<CorrelationId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitTier {
    pub tier: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub max_requests: u32,
    pub window_seconds: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsSpec {
    #[serde(default)]
    pub allow_credentials: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_policy: Option<CorsOriginPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CorsOriginPolicy {
    Explicit,
    SameOrigin,
    Wildcard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostValidation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<HostValidationPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HostValidationPolicy {
    Required,
    Optional,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CspSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub reporting_endpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to: Option<CsrfScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CsrfScope {
    StateChanging,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationId {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    #[serde(default)]
    pub propagate_to_downstream: bool,
}

// ── Health Checks ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChecks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liveness: Option<HealthProbe>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness: Option<ReadinessProbe>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<InfoProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthProbe {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessProbe {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<HealthDependency>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDependency {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoProbe {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub redact_in_production: bool,
}

// ── Error Handling ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorHandling {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<ErrorEnvelope>,
    #[serde(default)]
    pub suppress_details_in_production: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEnvelope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_subfields: Option<Vec<ErrorSubfield>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSubfield {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
}

// ── Data Ingestion ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataIngestion {
    /// References `integrations[].id`.
    pub integration_id: String,
    #[serde(default)]
    pub manual_trigger: bool,
    #[serde(default)]
    pub status_endpoint: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial_failure: Option<PartialFailurePolicy>,
    #[serde(default)]
    pub staleness_tracking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_rules: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PartialFailurePolicy {
    Abort,
    Continue,
    ContinueAndLog,
}

// ── File Storage ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStorage {
    /// References `integrations[].id`.
    pub integration_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload: Option<FileTransferSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download: Option<FileTransferSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub virus_scanning: Option<VirusScanSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_file_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_mime_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransferSpec {
    pub method: FileTransferMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_expiry_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FileTransferMethod {
    PreSignedUrl,
    DirectUpload,
    Multipart,
    Proxy,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusScanSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub scan_before_available: bool,
}

// ── Traceability ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceabilitySpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cases: Option<Vec<UseCase>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_cases: Option<Vec<TestCase>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseCase {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub covers_use_case: Option<String>,
    #[serde(rename = "type")]
    pub test_type: TestCaseType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TestCaseType {
    Unit,
    Integration,
    E2e,
    Smoke,
    Manual,
}
