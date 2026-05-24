//! Spec-spine shared types.
//!
//! Canonical home for the vocabularies and code registries that
//! spec-compiler, spec-lint, codebase-indexer, and policy-compiler all
//! consume. Frontmatter parsing helpers (formerly the
//! `open_agentic_frontmatter` crate) live here too; they have no
//! semantic dependency on the vocabularies but ship from the same
//! leaf crate so every spec-spine producer takes exactly one
//! foundational dep.
//!
//! Hard leaf — depends only on `serde` / `serde_yaml`.

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

// ─────────────────────────────────────────────────────────────────────
// Frontmatter parsing (absorbed from open_agentic_frontmatter)
// ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum FrontmatterError {
    MissingFrontmatter,
    Yaml(serde_yaml::Error),
}

impl From<serde_yaml::Error> for FrontmatterError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Yaml(value)
    }
}

pub fn split_frontmatter_required(raw: &str) -> Result<(Value, String), FrontmatterError> {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let rest = raw
        .strip_prefix("---")
        .ok_or(FrontmatterError::MissingFrontmatter)?;
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
        .ok_or(FrontmatterError::MissingFrontmatter)?;

    let (yaml_str, body) = if let Some(i) = rest.find("\n---\n") {
        (&rest[..i], rest[i + 5..].to_string())
    } else if let Some(i) = rest.find("\r\n---\r\n") {
        (&rest[..i], rest[i + 7..].to_string())
    } else {
        return Err(FrontmatterError::MissingFrontmatter);
    };

    let value: Value = serde_yaml::from_str(yaml_str)?;
    Ok((value, body))
}

pub fn split_frontmatter_optional(raw: &str) -> Option<(Value, String)> {
    split_frontmatter_required(raw).ok()
}

// ─────────────────────────────────────────────────────────────────────
// Spec-format vocabularies (canonical source; consumers hoist from here
// once W-02 lands)
// ─────────────────────────────────────────────────────────────────────

/// Known frontmatter keys consumed into normalized fields (remainder → extraFrontmatter).
///
/// Cut D W-06c: `compliance` retained in this allowlist but NO LONGER
/// emitted by spec-compiler (the FeatureRecord.compliance field was
/// removed in W-06c). The OAP-side enricher (oap-registry-enrich) is
/// the canonical reader of `compliance:` and emits it to
/// registry-oap.json. Keeping the key in KNOWN_KEYS prevents V-002
/// errors when the spec corpus carries `compliance:` frontmatter that
/// extra_frontmatter would otherwise reject as an unsupported complex
/// type. (KNOWN_KEYS is a "permitted frontmatter" allowlist, not a
/// "fields emitted by spec-compiler" list — the two were aligned
/// before W-06c.)
pub const KNOWN_KEYS: &[&str] = &[
    "id",
    "title",
    "status",
    "created",
    "summary",
    "authors",
    "kind",
    "feature_branch",
    "code_aliases",
    "depends_on",
    "owner",
    "risk",
    "implementation",
    "implements",
    "compliance",
    // Spec 132 — unamendable invariants.
    "amends",
    "amends_sections",
    "unamendable",
    // Spec 147 — universal dimensions and governance lifecycle.
    "shape",
    "category",
    "supersedes",
    "superseded_by",
    "retirement_rationale",
    // Spec 147 — per-kind structural fields (kind: capability / registry / profile).
    "provides",
    "composition",
    "selectable_by",
    "selector",
    "default",
    "production_forbidden",
    "member_contract",
    "identity",
    "selects",
    "policy",
    // Spec 130 (relationship graph) — eight relationship fields. Authors
    // declare relationships explicitly; `implements:` is derived from the
    // union of paths in `establishes`, `extends.paths`, `refines.paths`,
    // and `co_authority.paths`. `origin: retroactive: true` is the
    // bootstrap marker for specs not yet curated into the graph.
    "establishes",
    "extends",
    "refines",
    "supersedes",  // already valid as V147 list; rebound by spec 130 to relationship-graph semantics (object form: {spec, scope, paths?, rationale}). Backward-compatible: string-list form treated as scope=full.
    "amends",      // already in spec 132 list-of-ids form; rebound to support object form ({spec, change_type, paths}) when relationship-graph semantics are desired.
    "co_authority",
    "constrains",
    "origin",
    // Spec 154 (logical-unit ownership grammar) — ninth relationship,
    // declaratively non-owning. Items are units the spec mentions for
    // evidence / illustration / provenance without claiming authority
    // over them. The coupling gate ignores references; the indexer
    // surfaces them for navigation.
    "references",
    // Spec 179 — universal tract-authority lens. Closed enum
    // (`opc | platform | substrate | tooling`). V-030 validates the
    // enum at error severity; V-031 emits at warning severity when
    // the field is absent.
    "domain",
];

/// Valid values for the `risk` frontmatter field.
pub const VALID_RISK_LEVELS: &[&str] = &["low", "medium", "high", "critical"];

/// Spec 147 — valid values for the `kind` frontmatter field (V-012).
pub const VALID_KINDS: &[&str] = &[
    "platform",
    "platform-delivery",
    "governance",
    "product",
    "amendment",
    "tooling",
    "desktop",
    "process",
    "ui",
    "architecture",
    "constitutional-bootstrap",
    "migration",
    "product-consolidation",
    "capability",
    "registry",
    "profile",
];

/// Spec 147 — declared `(kind, shape)` table. Reserved for downstream
/// consumers: spec-lint emits W-131 against entries outside this table.
pub const SHAPE_TABLE: &[(&str, &[&str])] = &[
    (
        "capability",
        &["driver", "module", "web-snippet", "middleware-stack"],
    ),
    (
        "amendment",
        &[
            "field-addition",
            "field-modification",
            "mechanism-add",
            "mechanism-modification",
            "bug-fix",
            "retirement-record",
            "consolidation",
        ],
    ),
];

/// Spec 179 — valid values for the `domain:` frontmatter field (V-030).
/// Closed enum; future amendments widen it.
pub const VALID_DOMAINS: &[&str] = &["opc", "platform", "substrate", "tooling"];

/// Spec 147 — conventional `category:` vocabulary (W-130, info severity).
pub const CONVENTIONAL_CATEGORIES: &[&str] = &[
    "security",
    "auth",
    "data",
    "ui",
    "infrastructure",
    "governance",
    "audit",
    "compliance",
    "identity",
    "lifecycle",
    "policy",
    "performance",
    "observability",
    "release",
    "testing",
];

// ─────────────────────────────────────────────────────────────────────
// Diagnostic-code registries (V-xxx compiler violations, W-xxx lint
// warnings)
// ─────────────────────────────────────────────────────────────────────

/// Severity tier for a diagnostic. Mirrors spec 128 §7 vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Stable identifier for a single diagnostic code (`V-013`, `W-131`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ViolationCode(pub &'static str);

impl ViolationCode {
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for ViolationCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

// Compiler V-codes (emitted by spec-compiler).
pub const V_001: ViolationCode = ViolationCode("V-001");
pub const V_002: ViolationCode = ViolationCode("V-002");
pub const V_003: ViolationCode = ViolationCode("V-003");
pub const V_004: ViolationCode = ViolationCode("V-004");
pub const V_005: ViolationCode = ViolationCode("V-005");
pub const V_006: ViolationCode = ViolationCode("V-006");
pub const V_007: ViolationCode = ViolationCode("V-007");
pub const V_008: ViolationCode = ViolationCode("V-008");
pub const V_010: ViolationCode = ViolationCode("V-010");
pub const V_011: ViolationCode = ViolationCode("V-011");
pub const V_012: ViolationCode = ViolationCode("V-012");
pub const V_013: ViolationCode = ViolationCode("V-013");
pub const V_014: ViolationCode = ViolationCode("V-014");
pub const V_015: ViolationCode = ViolationCode("V-015");
pub const V_016: ViolationCode = ViolationCode("V-016");
pub const V_017: ViolationCode = ViolationCode("V-017");
pub const V_018: ViolationCode = ViolationCode("V-018");
pub const V_019: ViolationCode = ViolationCode("V-019");
/// Spec 130 — emitted by spec-lint when a spec carries no relationship
/// fields (`establishes`, `extends`, `refines`, `supersedes`, `amends`,
/// `co_authority`, `constrains`) and no `origin: retroactive: true`
/// bootstrap marker. Initial severity: warning (corpus migration is
/// staged; the gate falls back to legacy `implements:` claim semantics
/// for un-annotated specs). Promotion to error follows the curated-
/// annotation pass.
pub const V_020: ViolationCode = ViolationCode("V-020");

/// Spec 154 — fired by spec-compiler when a `crate:` unit's `id` does
/// not appear in the root workspace manifest's `[workspace] members`
/// array. Hard error: the crate id is the unit's stable identifier
/// and an unresolvable id means the spec is referring to a
/// workspace member that does not exist.
pub const V_021: ViolationCode = ViolationCode("V-021");

/// Spec 154 — fired by spec-compiler when a `directory:` unit's
/// `path` does not exist as a directory in the worktree. Hard error.
pub const V_022: ViolationCode = ViolationCode("V-022");

/// Spec 154 — fired by spec-compiler when a `file:` unit's `path`
/// does not exist as a file in the worktree. Hard error. (Git rename
/// trace handling is deferred to the codebase-indexer resolver in
/// Tier 2 Segment 3; today the check is literal-existence only.)
pub const V_023: ViolationCode = ViolationCode("V-023");

/// Spec 154 — fired by spec-compiler when a logical-unit declaration
/// in a relationship field is malformed (unknown `kind:` value,
/// missing required field for the declared kind, or not a string /
/// mapping shape). Hard error.
pub const V_024: ViolationCode = ViolationCode("V-024");

/// Spec 156 — fired by spec-compiler when a `references:` entry
/// carries both `unit:` and `provenance:` (mutually exclusive) or
/// neither (the entry has no target). Hard error. The two arms are
/// declaratively distinct: `unit:` points at an in-tree logical unit
/// (spec 154 grammar); `provenance:` points at an external derivation
/// source (knowledge item, code fingerprint).
pub const V_025: ViolationCode = ViolationCode("V-025");

/// Spec 156 — fired by spec-compiler when `provenance.kind` is not
/// one of the two accepted values (`knowledge`, `code-fingerprint`).
/// The enum is closed; adding a kind requires an amendment spec that
/// widens both this enum and V-027's scheme alignment table.
pub const V_026: ViolationCode = ViolationCode("V-026");

/// Spec 156 — fired by spec-compiler when `provenance.ref` scheme
/// does not align with the declared `provenance.kind`. `knowledge`
/// requires the `stagecraft://project/<uuid>/knowledge/<uuid>` shape;
/// `code-fingerprint` requires `xray-fingerprint://<sha256>`. Scheme
/// mismatch, missing project segment, or malformed UUID/digest is a
/// hard error.
pub const V_027: ViolationCode = ViolationCode("V-027");

/// Spec 156 — fired by spec-compiler when `provenance.ref` is not a
/// well-formed URI for its kind, or when the opaque body (UUID-pair
/// for `knowledge`, hex digest for `code-fingerprint`) is empty.
/// Hard error.
pub const V_028: ViolationCode = ViolationCode("V-028");

/// Spec 156 — advisory emitted by spec-compiler when a `provenance:`
/// entry omits `role:`. Recommends `role: derivation` for
/// searchability and consistent rendering. Severity: warning (not
/// blocking — does NOT flip `validation.passed`).
pub const V_029: ViolationCode = ViolationCode("V-029");

/// Spec 179 — fired by spec-compiler when `domain:` is present but
/// its value is not one of the four closed-enum values
/// (`opc | platform | substrate | tooling`). Hard error: the field
/// is a lens over the unified corpus and an invalid value silently
/// excludes the spec from every scoped query. Also emitted by
/// spec-lint with the same semantics for contributors running the
/// linter in isolation.
pub const V_030: ViolationCode = ViolationCode("V-030");

/// Spec 179 — fired by spec-lint when a spec's frontmatter omits
/// `domain:`. Severity: warning (corpus-wide backfill is staged in
/// the spec 179 PR; promotion to error severity is deferred to a
/// follow-on amendment after the warning-tier corpus is empirically
/// clean).
pub const V_031: ViolationCode = ViolationCode("V-031");

// Lint W-codes (emitted by spec-lint).
pub const W_001: ViolationCode = ViolationCode("W-001");
pub const W_002: ViolationCode = ViolationCode("W-002");
pub const W_003: ViolationCode = ViolationCode("W-003");
pub const W_004: ViolationCode = ViolationCode("W-004");
pub const W_005: ViolationCode = ViolationCode("W-005");
pub const W_006: ViolationCode = ViolationCode("W-006");
pub const W_007: ViolationCode = ViolationCode("W-007");
pub const W_130: ViolationCode = ViolationCode("W-130");
pub const W_131: ViolationCode = ViolationCode("W-131");
pub const W_132: ViolationCode = ViolationCode("W-132");

/// Spec 161 — emitted by spec-lint when a `references:` entry carries
/// `role: decomposition-origin` but fails the role-reservation contract
/// from spec 161 §2.1/§2.3. The role is reserved for entries that
/// (a) use the `provenance:` arm (not `unit:`), and (b) carry a
/// non-empty `provenance.derived_at:` ISO-8601 timestamp recording when
/// the OPC decomposition pipeline (spec 165) read the source. Severity:
/// `error` — V-026-equivalent (SC-004); fails spec-lint unconditionally,
/// independent of `--fail-on-warn`.
pub const W_161: ViolationCode = ViolationCode("W-161");

/// Spec 154 — advisory soft lint emitted by spec-lint when a legacy
/// path-string (or explicit `file:` unit) sits inside a workspace
/// member's directory tree and could be expressed as the higher-level
/// `crate:` unit. The hint is advisory; corpus migration to the unit
/// grammar is Tier 2 Segment 5. Info severity; does NOT participate
/// in `--fail-on-warn`.
pub const L_005: ViolationCode = ViolationCode("L-005");

// ─────────────────────────────────────────────────────────────────────
// Spec 154 — Logical-unit ownership grammar
// ─────────────────────────────────────────────────────────────────────

/// One logical unit declared inside a relationship-graph field
/// (`establishes`, `extends.unit`, `refines.unit`, `supersedes.unit`,
/// `co_authority.unit`, `constrains.unit`, `references.unit`).
///
/// The six kinds correspond to the six observed ownership shapes
/// across the spec corpus (spec 154 §2). Resolution from a unit to a
/// concrete `(file, span)` set lives in the codebase-indexer (spec 154
/// Tier 2 Segment 3); spec-compiler stops at parsing and basic
/// type-checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalUnit {
    /// `{ kind: crate, id: <workspace-member-name> }`. Stable under
    /// crate-directory relocation; changes on manifest-name rename.
    Crate { id: String },
    /// `{ kind: symbol, id: <rust-path> }`. Stable under in-module
    /// reorderings; changes on symbol rename or cross-module move.
    Symbol { id: String },
    /// `{ kind: module, id: <rust-path> }`. Stable under symbol
    /// additions; changes on module rename or restructure.
    Module { id: String },
    /// `{ kind: section, file: <path>, anchor: <anchor-name> }`. Per-
    /// file-kind anchor semantics live in spec 152 (path-co-authority).
    Section { file: String, anchor: String },
    /// `{ kind: directory, path: <workspace-relative-path> }`. Resolves
    /// to `<path>/**` with the standard exclusion set (spec 154 §3.7).
    Directory { path: String },
    /// `{ kind: file, path: <file-path> }`. Literal worktree path; the
    /// legacy bare-string form parses to this variant.
    File { path: String },
}

/// Failures from [`LogicalUnit::from_yaml`] / [`LogicalUnit::from_json`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalUnitParseError {
    /// Value was neither a string nor a mapping.
    NotStringOrMapping,
    /// `kind:` discriminator was missing on a mapping.
    MissingKind,
    /// `kind:` discriminator was not one of the six accepted values.
    UnknownKind(String),
    /// Required field for the declared kind was absent.
    MissingField {
        kind: &'static str,
        field: &'static str,
    },
    /// Required field was present but not a string.
    FieldNotString {
        kind: &'static str,
        field: &'static str,
    },
    /// Spec 155 §2.1 — `kind: symbol` id contains `<` or `>`. These
    /// characters appear only in type-expression, turbofish, or
    /// qualified-path syntax, none of which are part of an item's
    /// path identity.
    SymbolIdNotItemPath { id: String },
}

impl std::fmt::Display for LogicalUnitParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogicalUnitParseError::NotStringOrMapping => {
                write!(
                    f,
                    "logical unit must be a string (legacy file-path form) or a mapping with `kind:`"
                )
            }
            LogicalUnitParseError::MissingKind => {
                write!(f, "logical unit mapping is missing `kind:`")
            }
            LogicalUnitParseError::UnknownKind(k) => write!(
                f,
                "logical unit kind {k:?} is not one of: crate, symbol, module, section, directory, file"
            ),
            LogicalUnitParseError::MissingField { kind, field } => {
                write!(f, "logical unit kind={kind:?} requires field `{field}:`")
            }
            LogicalUnitParseError::FieldNotString { kind, field } => {
                write!(
                    f,
                    "logical unit kind={kind:?} field `{field}:` must be a string"
                )
            }
            LogicalUnitParseError::SymbolIdNotItemPath { id } => {
                write!(
                    f,
                    "logical unit kind=\"symbol\" id {id:?} contains `<` or `>`; \
                     symbol ids carry a Rust item path only, not type expressions \
                     or turbofish / qualified-path syntax (see spec 155 §2.1)"
                )
            }
        }
    }
}

impl std::error::Error for LogicalUnitParseError {}

impl LogicalUnit {
    /// Stable kind discriminator string.
    pub fn kind_str(&self) -> &'static str {
        match self {
            LogicalUnit::Crate { .. } => "crate",
            LogicalUnit::Symbol { .. } => "symbol",
            LogicalUnit::Module { .. } => "module",
            LogicalUnit::Section { .. } => "section",
            LogicalUnit::Directory { .. } => "directory",
            LogicalUnit::File { .. } => "file",
        }
    }

    /// Parse from a YAML value. A bare string maps to
    /// [`LogicalUnit::File`]; a mapping dispatches on `kind:`.
    pub fn from_yaml(v: &serde_yaml::Value) -> Result<Self, LogicalUnitParseError> {
        if let Some(s) = v.as_str() {
            return Ok(LogicalUnit::File {
                path: s.to_string(),
            });
        }
        let map = v
            .as_mapping()
            .ok_or(LogicalUnitParseError::NotStringOrMapping)?;
        let kind = map
            .get("kind")
            .and_then(|x| x.as_str())
            .ok_or(LogicalUnitParseError::MissingKind)?;
        Self::from_mapping(kind, |k| map.get(k).and_then(|x| x.as_str()))
    }

    /// Parse from a JSON value. Mirrors [`Self::from_yaml`].
    pub fn from_json(v: &serde_json::Value) -> Result<Self, LogicalUnitParseError> {
        if let Some(s) = v.as_str() {
            return Ok(LogicalUnit::File {
                path: s.to_string(),
            });
        }
        let map = v
            .as_object()
            .ok_or(LogicalUnitParseError::NotStringOrMapping)?;
        let kind = map
            .get("kind")
            .and_then(|x| x.as_str())
            .ok_or(LogicalUnitParseError::MissingKind)?;
        Self::from_mapping(kind, |k| map.get(k).and_then(|x| x.as_str()))
    }

    fn from_mapping<'a>(
        kind: &str,
        get: impl Fn(&str) -> Option<&'a str>,
    ) -> Result<Self, LogicalUnitParseError> {
        match kind {
            "crate" => {
                let id = get("id").ok_or(LogicalUnitParseError::MissingField {
                    kind: "crate",
                    field: "id",
                })?;
                Ok(LogicalUnit::Crate { id: id.to_string() })
            }
            "symbol" => {
                let id = get("id").ok_or(LogicalUnitParseError::MissingField {
                    kind: "symbol",
                    field: "id",
                })?;
                if id.contains('<') || id.contains('>') {
                    return Err(LogicalUnitParseError::SymbolIdNotItemPath {
                        id: id.to_string(),
                    });
                }
                Ok(LogicalUnit::Symbol { id: id.to_string() })
            }
            "module" => {
                let id = get("id").ok_or(LogicalUnitParseError::MissingField {
                    kind: "module",
                    field: "id",
                })?;
                Ok(LogicalUnit::Module { id: id.to_string() })
            }
            "section" => {
                let file = get("file").ok_or(LogicalUnitParseError::MissingField {
                    kind: "section",
                    field: "file",
                })?;
                let anchor = get("anchor").ok_or(LogicalUnitParseError::MissingField {
                    kind: "section",
                    field: "anchor",
                })?;
                Ok(LogicalUnit::Section {
                    file: file.to_string(),
                    anchor: anchor.to_string(),
                })
            }
            "directory" => {
                let path = get("path").ok_or(LogicalUnitParseError::MissingField {
                    kind: "directory",
                    field: "path",
                })?;
                Ok(LogicalUnit::Directory {
                    path: path.to_string(),
                })
            }
            "file" => {
                let path = get("path").ok_or(LogicalUnitParseError::MissingField {
                    kind: "file",
                    field: "path",
                })?;
                Ok(LogicalUnit::File {
                    path: path.to_string(),
                })
            }
            other => Err(LogicalUnitParseError::UnknownKind(other.to_string())),
        }
    }

    /// Canonical JSON representation. Round-trips through [`Self::from_json`].
    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            LogicalUnit::Crate { id } => json!({ "kind": "crate", "id": id }),
            LogicalUnit::Symbol { id } => json!({ "kind": "symbol", "id": id }),
            LogicalUnit::Module { id } => json!({ "kind": "module", "id": id }),
            LogicalUnit::Section { file, anchor } => {
                json!({ "kind": "section", "file": file, "anchor": anchor })
            }
            LogicalUnit::Directory { path } => json!({ "kind": "directory", "path": path }),
            LogicalUnit::File { path } => json!({ "kind": "file", "path": path }),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Spec 156 — References-edge provenance grammar
// ─────────────────────────────────────────────────────────────────────

/// One provenance entry — the sibling arm on `references:` entries
/// introduced by spec 156. Mutually exclusive with `unit:` at the
/// references-entry level (V-025).
///
/// Provenance entries are declaratively non-owning by inheritance
/// from spec 154 §4 (`references:` is the ninth, non-owning
/// relationship). Two initial kinds carry typed URI references to
/// external derivation sources:
///
/// - `Knowledge { project_uuid, knowledge_uuid }` — stagecraft's
///   `knowledge_objects` table. The URI is
///   `stagecraft://project/<project-uuid>/knowledge/<knowledge-uuid>`.
/// - `CodeFingerprint { digest }` — content-addressed SHA-256 over an
///   imported tree (`crates/xray/src/tools.rs::xray_fingerprint`). The
///   URI is `xray-fingerprint://<sha256>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceKind {
    Knowledge {
        project_uuid: String,
        knowledge_uuid: String,
    },
    CodeFingerprint {
        digest: String,
    },
}

impl ProvenanceKind {
    /// Stable kind discriminator string. Matches the value the author
    /// writes under `provenance.kind:`.
    pub fn kind_str(&self) -> &'static str {
        match self {
            ProvenanceKind::Knowledge { .. } => "knowledge",
            ProvenanceKind::CodeFingerprint { .. } => "code-fingerprint",
        }
    }

    /// Re-emit the canonical URI string. Round-trips through
    /// [`ProvenanceKind::parse_uri`].
    pub fn to_uri(&self) -> String {
        match self {
            ProvenanceKind::Knowledge {
                project_uuid,
                knowledge_uuid,
            } => format!("stagecraft://project/{project_uuid}/knowledge/{knowledge_uuid}"),
            ProvenanceKind::CodeFingerprint { digest } => {
                format!("xray-fingerprint://{digest}")
            }
        }
    }

    /// Parse a `(kind_str, uri)` pair into a typed [`ProvenanceKind`].
    /// Surfaces the precise violation code on failure so callers can
    /// route each into the right V-code emission path.
    pub fn parse(kind_str: &str, uri: &str) -> Result<Self, ProvenanceParseError> {
        match kind_str {
            "knowledge" => parse_knowledge_uri(uri),
            "code-fingerprint" => parse_fingerprint_uri(uri),
            other => Err(ProvenanceParseError::UnknownKind {
                kind: other.to_string(),
            }),
        }
    }
}

/// Provenance parse failures. Each variant maps to one V-code:
/// - `UnknownKind` → V-026
/// - `SchemeMismatch`, `MissingProjectSegment` → V-027
/// - `EmptyBody`, `MalformedUuid`, `MalformedDigest`, `MalformedUri` → V-028
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceParseError {
    /// `provenance.kind:` is not one of `{knowledge, code-fingerprint}`.
    UnknownKind { kind: String },
    /// `kind:` and the URI scheme disagree (e.g. `kind: knowledge`
    /// paired with `xray-fingerprint://...`).
    SchemeMismatch { kind: String, scheme: String },
    /// `kind: knowledge` URI is missing the `/project/<uuid>/knowledge/<uuid>/`
    /// shape required by V-027.
    MissingProjectSegment { uri: String },
    /// URI opaque body is empty (UUID-pair for `knowledge`, hex digest
    /// for `code-fingerprint`).
    EmptyBody { kind: String },
    /// URI carries something that should be a canonical UUID but
    /// isn't (8-4-4-4-12 hex, case-insensitive).
    MalformedUuid { value: String },
    /// URI carries something that should be a 64-char hex SHA-256
    /// digest but isn't.
    MalformedDigest { value: String },
    /// URI is structurally unparseable (e.g. missing `://`).
    MalformedUri { uri: String },
}

impl std::fmt::Display for ProvenanceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvenanceParseError::UnknownKind { kind } => write!(
                f,
                "provenance kind {kind:?} is not one of: knowledge, code-fingerprint"
            ),
            ProvenanceParseError::SchemeMismatch { kind, scheme } => write!(
                f,
                "provenance kind {kind:?} does not align with URI scheme {scheme:?}"
            ),
            ProvenanceParseError::MissingProjectSegment { uri } => write!(
                f,
                "knowledge URI {uri:?} is missing the required `/project/<uuid>/knowledge/<uuid>` segment shape"
            ),
            ProvenanceParseError::EmptyBody { kind } => {
                write!(f, "provenance kind {kind:?} has an empty URI body")
            }
            ProvenanceParseError::MalformedUuid { value } => write!(
                f,
                "{value:?} is not a canonical UUID (expected 8-4-4-4-12 hex, case-insensitive)"
            ),
            ProvenanceParseError::MalformedDigest { value } => write!(
                f,
                "{value:?} is not a 64-character hex SHA-256 digest"
            ),
            ProvenanceParseError::MalformedUri { uri } => {
                write!(f, "URI {uri:?} is structurally malformed")
            }
        }
    }
}

impl std::error::Error for ProvenanceParseError {}

fn parse_knowledge_uri(uri: &str) -> Result<ProvenanceKind, ProvenanceParseError> {
    let rest = uri
        .strip_prefix("stagecraft://")
        .ok_or_else(|| match uri.split_once("://") {
            Some((scheme, _)) => ProvenanceParseError::SchemeMismatch {
                kind: "knowledge".to_string(),
                scheme: scheme.to_string(),
            },
            None => ProvenanceParseError::MalformedUri {
                uri: uri.to_string(),
            },
        })?;
    // Required shape: project/<uuid>/knowledge/<uuid>
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 4 || parts[0] != "project" || parts[2] != "knowledge" {
        return Err(ProvenanceParseError::MissingProjectSegment {
            uri: uri.to_string(),
        });
    }
    let project_uuid = parts[1];
    let knowledge_uuid = parts[3];
    if project_uuid.is_empty() || knowledge_uuid.is_empty() {
        return Err(ProvenanceParseError::EmptyBody {
            kind: "knowledge".to_string(),
        });
    }
    if !is_canonical_uuid(project_uuid) {
        return Err(ProvenanceParseError::MalformedUuid {
            value: project_uuid.to_string(),
        });
    }
    if !is_canonical_uuid(knowledge_uuid) {
        return Err(ProvenanceParseError::MalformedUuid {
            value: knowledge_uuid.to_string(),
        });
    }
    Ok(ProvenanceKind::Knowledge {
        project_uuid: project_uuid.to_string(),
        knowledge_uuid: knowledge_uuid.to_string(),
    })
}

fn parse_fingerprint_uri(uri: &str) -> Result<ProvenanceKind, ProvenanceParseError> {
    let rest = uri
        .strip_prefix("xray-fingerprint://")
        .ok_or_else(|| match uri.split_once("://") {
            Some((scheme, _)) => ProvenanceParseError::SchemeMismatch {
                kind: "code-fingerprint".to_string(),
                scheme: scheme.to_string(),
            },
            None => ProvenanceParseError::MalformedUri {
                uri: uri.to_string(),
            },
        })?;
    if rest.is_empty() {
        return Err(ProvenanceParseError::EmptyBody {
            kind: "code-fingerprint".to_string(),
        });
    }
    if !is_sha256_digest(rest) {
        return Err(ProvenanceParseError::MalformedDigest {
            value: rest.to_string(),
        });
    }
    Ok(ProvenanceKind::CodeFingerprint {
        digest: rest.to_string(),
    })
}

/// Canonical UUID shape: 8-4-4-4-12 lowercase or uppercase hex.
fn is_canonical_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (i, &c) in b.iter().enumerate() {
        let want_hyphen = matches!(i, 8 | 13 | 18 | 23);
        if want_hyphen {
            if c != b'-' {
                return false;
            }
        } else if !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

/// 64 hex characters (case-insensitive).
fn is_sha256_digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}

/// Baseline (contract-floor) exclusion set applied by the
/// codebase-indexer's resolver when materialising a `crate:` or
/// `directory:` unit into a glob (spec 154 §3.7). Owned here so
/// consumers downstream of spec-compiler (codebase-indexer, coupling
/// gate) share one truth.
///
/// Per the 2026-05-24 amendment to §3.7, the resolver also honors
/// committed `.gitignore` files in the worktree (via
/// `ignore::WalkBuilder` with `git_ignore(true).git_exclude(false)
/// .git_global(false)`). This baseline list is preserved as a
/// defensive floor for repos whose `.gitignore` omits the canonical
/// build artifact directories — additions still require a spec
/// amendment to §3.7.
pub const RESOLVER_EXCLUSIONS: &[&str] = &[
    "target/**",
    "node_modules/**",
    ".derived/**",
    "dist/**",
    "build/**",
    ".next/**",
];

#[cfg(test)]
mod frontmatter_tests {
    use super::*;

    #[test]
    fn splits_required_frontmatter() {
        let raw = "---\nid: x\n---\nbody\n";
        let (fm, body) = split_frontmatter_required(raw).unwrap();
        assert_eq!(fm.get("id").and_then(|v| v.as_str()), Some("x"));
        assert_eq!(body, "body\n");
    }

    #[test]
    fn missing_frontmatter_returns_err() {
        let raw = "no frontmatter here";
        assert!(matches!(
            split_frontmatter_required(raw),
            Err(FrontmatterError::MissingFrontmatter)
        ));
    }

    #[test]
    fn optional_returns_none_when_absent() {
        assert!(split_frontmatter_optional("no frontmatter").is_none());
    }
}

#[cfg(test)]
mod logical_unit_tests {
    use super::*;

    fn yaml(s: &str) -> serde_yaml::Value {
        serde_yaml::from_str(s).expect("parse YAML")
    }

    #[test]
    fn bare_string_parses_as_file_unit() {
        let v = yaml("crates/foo/src/lib.rs");
        let u = LogicalUnit::from_yaml(&v).unwrap();
        assert_eq!(
            u,
            LogicalUnit::File {
                path: "crates/foo/src/lib.rs".into()
            }
        );
    }

    #[test]
    fn crate_unit_requires_id() {
        let v = yaml("{ kind: crate }");
        let err = LogicalUnit::from_yaml(&v).unwrap_err();
        assert_eq!(
            err,
            LogicalUnitParseError::MissingField {
                kind: "crate",
                field: "id"
            }
        );
    }

    #[test]
    fn crate_unit_with_id() {
        let v = yaml("{ kind: crate, id: canonical-json }");
        let u = LogicalUnit::from_yaml(&v).unwrap();
        assert_eq!(
            u,
            LogicalUnit::Crate {
                id: "canonical-json".into()
            }
        );
    }

    #[test]
    fn section_unit_requires_file_and_anchor() {
        let v = yaml("{ kind: section, file: Makefile }");
        let err = LogicalUnit::from_yaml(&v).unwrap_err();
        assert_eq!(
            err,
            LogicalUnitParseError::MissingField {
                kind: "section",
                field: "anchor"
            }
        );
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let v = yaml("{ kind: invented, id: x }");
        let err = LogicalUnit::from_yaml(&v).unwrap_err();
        assert_eq!(err, LogicalUnitParseError::UnknownKind("invented".into()));
    }

    #[test]
    fn to_json_round_trips() {
        for u in [
            LogicalUnit::Crate { id: "x".into() },
            LogicalUnit::Symbol { id: "x::y".into() },
            LogicalUnit::Module { id: "x::y".into() },
            LogicalUnit::Section {
                file: "Makefile".into(),
                anchor: "deploy".into(),
            },
            LogicalUnit::Directory { path: "infra".into() },
            LogicalUnit::File {
                path: "deny.toml".into(),
            },
        ] {
            let j = u.to_json();
            let back = LogicalUnit::from_json(&j).unwrap();
            assert_eq!(back, u);
        }
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    const PROJ: &str = "8c4f1234-1234-4abc-9def-1234567890ab";
    const KNOWLEDGE: &str = "2a91abcd-1111-4222-a333-444555666777";
    const DIGEST: &str = "5e3b00112233445566778899aabbccddeeff00112233445566778899aabbccdd";

    #[test]
    fn knowledge_uri_parses_and_round_trips() {
        let uri = format!("stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}");
        let p = ProvenanceKind::parse("knowledge", &uri).unwrap();
        assert_eq!(
            p,
            ProvenanceKind::Knowledge {
                project_uuid: PROJ.into(),
                knowledge_uuid: KNOWLEDGE.into(),
            }
        );
        assert_eq!(p.to_uri(), uri);
        assert_eq!(p.kind_str(), "knowledge");
    }

    #[test]
    fn fingerprint_uri_parses_and_round_trips() {
        let uri = format!("xray-fingerprint://{DIGEST}");
        let p = ProvenanceKind::parse("code-fingerprint", &uri).unwrap();
        assert_eq!(
            p,
            ProvenanceKind::CodeFingerprint {
                digest: DIGEST.into(),
            }
        );
        assert_eq!(p.to_uri(), uri);
        assert_eq!(p.kind_str(), "code-fingerprint");
    }

    #[test]
    fn unknown_kind_rejected() {
        let err = ProvenanceKind::parse("sbom", "sbom://anything").unwrap_err();
        assert_eq!(
            err,
            ProvenanceParseError::UnknownKind {
                kind: "sbom".into(),
            }
        );
    }

    #[test]
    fn knowledge_scheme_mismatch() {
        let uri = format!("xray-fingerprint://{DIGEST}");
        let err = ProvenanceKind::parse("knowledge", &uri).unwrap_err();
        assert_eq!(
            err,
            ProvenanceParseError::SchemeMismatch {
                kind: "knowledge".into(),
                scheme: "xray-fingerprint".into(),
            }
        );
    }

    #[test]
    fn knowledge_missing_project_segment() {
        let err = ProvenanceKind::parse(
            "knowledge",
            "stagecraft://item/8c4f1234-1234-4abc-9def-1234567890ab",
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ProvenanceParseError::MissingProjectSegment { .. }
        ));
    }

    #[test]
    fn knowledge_malformed_uuid() {
        let err = ProvenanceKind::parse(
            "knowledge",
            "stagecraft://project/not-a-uuid/knowledge/also-not-a-uuid",
        )
        .unwrap_err();
        assert!(matches!(err, ProvenanceParseError::MalformedUuid { .. }));
    }

    #[test]
    fn fingerprint_short_digest_rejected() {
        let err = ProvenanceKind::parse("code-fingerprint", "xray-fingerprint://abc123")
            .unwrap_err();
        assert!(matches!(err, ProvenanceParseError::MalformedDigest { .. }));
    }

    #[test]
    fn fingerprint_empty_body_rejected() {
        let err = ProvenanceKind::parse("code-fingerprint", "xray-fingerprint://").unwrap_err();
        assert!(matches!(err, ProvenanceParseError::EmptyBody { .. }));
    }

    #[test]
    fn fingerprint_scheme_mismatch() {
        let err = ProvenanceKind::parse(
            "code-fingerprint",
            "stagecraft://project/aaaa/knowledge/bbbb",
        )
        .unwrap_err();
        assert_eq!(
            err,
            ProvenanceParseError::SchemeMismatch {
                kind: "code-fingerprint".into(),
                scheme: "stagecraft".into(),
            }
        );
    }

    #[test]
    fn malformed_uri_no_scheme() {
        let err = ProvenanceKind::parse("knowledge", "no-scheme-here").unwrap_err();
        assert!(matches!(err, ProvenanceParseError::MalformedUri { .. }));
    }
}
