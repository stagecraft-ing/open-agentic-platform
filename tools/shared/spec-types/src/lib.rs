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

/// Standard exclusion set applied by the codebase-indexer's resolver
/// when materialising a `crate:` or `directory:` unit into a glob
/// (spec 154 §3.7). Owned here so consumers downstream of spec-compiler
/// (codebase-indexer, coupling gate) share one truth.
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
