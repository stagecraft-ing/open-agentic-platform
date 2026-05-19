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

// ─────────────────────────────────────────────────────────────────────
// Temporary cross-crate byte-equality check (dropped in W-02 once
// spec-compiler and spec-lint hoist their SHAPE_TABLE et al. from this
// crate). Asserts the duplicated literal source text in spec-compiler
// and spec-lint structurally agrees with the canonical copy here.
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod cross_crate_check {
    //! W-01 intentionally-temporary structural-equivalence check.
    //!
    //! `spec-compiler/src/lib.rs` and `spec-lint/src/lib.rs` still
    //! carry their own copies of `SHAPE_TABLE`, `KNOWN_KEYS`,
    //! `VALID_KINDS`, `VALID_RISK_LEVELS`, and `CONVENTIONAL_CATEGORIES`
    //! until W-02 lands. This test pulls both source files in via
    //! `include_str!` and confirms the literal tokens we expect to
    //! agree are present in all locations.

    const SPEC_COMPILER_LIB: &str = include_str!("../../../spec-compiler/src/lib.rs");
    const SPEC_LINT_LIB: &str = include_str!("../../../spec-lint/src/lib.rs");

    fn contains_all(haystack: &str, needles: &[&str]) -> bool {
        needles.iter().all(|n| haystack.contains(n))
    }

    #[test]
    fn shape_table_pairs_appear_in_both_duplicates() {
        // Pairs the SHAPE_TABLE encodes — present as literal text in
        // each duplicate copy and in this crate's canonical source.
        let pairs = [
            "\"capability\"",
            "\"driver\"",
            "\"module\"",
            "\"web-snippet\"",
            "\"middleware-stack\"",
            "\"amendment\"",
            "\"field-addition\"",
            "\"field-modification\"",
            "\"mechanism-add\"",
            "\"mechanism-modification\"",
            "\"bug-fix\"",
            "\"retirement-record\"",
            "\"consolidation\"",
        ];
        assert!(
            contains_all(SPEC_COMPILER_LIB, &pairs),
            "spec-compiler SHAPE_TABLE literal drifted from shared-types"
        );
        assert!(
            contains_all(SPEC_LINT_LIB, &pairs),
            "spec-lint SHAPE_TABLE literal drifted from shared-types"
        );
    }

    #[test]
    fn known_keys_appear_in_spec_compiler() {
        for key in super::KNOWN_KEYS {
            let lit = format!("\"{key}\"");
            assert!(
                SPEC_COMPILER_LIB.contains(&lit),
                "spec-compiler KNOWN_KEYS missing literal {lit}"
            );
        }
    }

    #[test]
    fn valid_kinds_appear_in_spec_compiler() {
        for kind in super::VALID_KINDS {
            let lit = format!("\"{kind}\"");
            assert!(
                SPEC_COMPILER_LIB.contains(&lit),
                "spec-compiler VALID_KINDS missing literal {lit}"
            );
        }
    }

    #[test]
    fn conventional_categories_appear_in_spec_lint() {
        for cat in super::CONVENTIONAL_CATEGORIES {
            let lit = format!("\"{cat}\"");
            assert!(
                SPEC_LINT_LIB.contains(&lit),
                "spec-lint CONVENTIONAL_CATEGORIES missing literal {lit}"
            );
        }
    }
}

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
