//! Library for compiling `specs/*/spec.md` into Feature 000 registry JSON.

use open_agentic_spec_types::{
    FrontmatterError, KNOWN_KEYS, VALID_KINDS, VALID_RISK_LEVELS, split_frontmatter_required,
};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        write!(&mut s, "{b:02x}").expect("write to String");
    }
    s
}

const COMPILER_ID: &str = "open-agentic-spec-compiler";
/// Schema version. 1.5.0 (spec 147) promotes `kind` to a closed enum,
/// adds the `shape` / `category` universal dimensions, introduces
/// per-kind structural fields for `kind: capability | registry | profile`
/// (`provides`, `composition`, `selector`, `member_contract`, `identity`,
/// `selects`, etc.), serializes `implements:` to registry output with
/// scalar/list shape disambiguation, and adds governance-lifecycle
/// fields (`supersedes`, `superseded_by`, `retirement_rationale`).
/// Validation invariants V-012..V-019 fire at warning severity in
/// Phase 1. Spec 147 phase-gates severity promotions:
///   - V-012 → error in Phase 2 (corpus-wide `kind:` backfill).
///   - V-018, V-019 → error in Phase 4 (governance-lifecycle fields
///     are now KNOWN_KEYS in the registry, and the 4 superseded
///     specs already carry valid `superseded_by:` pointers).
///   - V-013..V-017 remain at warning severity; their promotion to
///     error severity is the subject of a separately-funded
///     follow-on amendment after the contract is exercised against
///     unforeseen cases.
///
/// 1.4.0 (spec 132) added the `unamendable` and `amends_sections`
/// frontmatter fields plus the V-011 violation (amends_sections ∩
/// unamendable ≠ ∅).
/// Cut D W-06c — major bump to 2.0.0: registry.json no longer carries
/// `factoryProjects` (top-level) or per-feature `compliance:`. Both
/// fields moved to oap-registry-enrich and are emitted to
/// `build/spec-registry/registry-oap.json` as the OAP-specific
/// overlay. `compliance` is no longer a KNOWN_KEYS entry in
/// `open_agentic_spec_types` either — the generic spec compiler
/// treats it as extraFrontmatter passthrough now.
const SPEC_VERSION: &str = "2.0.0";

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    MissingFrontmatter { path: PathBuf },
    InvalidFrontmatter { path: PathBuf, msg: String },
}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

impl From<serde_yaml::Error> for CompileError {
    fn from(e: serde_yaml::Error) -> Self {
        CompileError::Yaml(e)
    }
}

impl From<serde_json::Error> for CompileError {
    fn from(e: serde_json::Error) -> Self {
        CompileError::Json(e)
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "{e}"),
            CompileError::Yaml(e) => write!(f, "{e}"),
            CompileError::Json(e) => write!(f, "{e}"),
            CompileError::MissingFrontmatter { path } => {
                write!(f, "missing YAML frontmatter: {}", path.display())
            }
            CompileError::InvalidFrontmatter { path, msg } => {
                write!(f, "{}: {msg}", path.display())
            }
        }
    }
}

impl std::error::Error for CompileError {}

/// Result of a compile: registry JSON bytes (deterministic) + build-meta JSON bytes (ephemeral).
pub struct CompileOutput {
    pub registry_json: Vec<u8>,
    pub build_meta_json: Vec<u8>,
    pub validation_passed: bool,
}

/// Run compilation from `repo_root` (must be the repository root). Writes to `build/spec-registry/`.
pub fn compile_and_write(repo_root: &Path) -> Result<CompileOutput, CompileError> {
    let out = compile(repo_root)?;
    let out_dir = repo_root.join("build/spec-registry");
    fs::create_dir_all(&out_dir)?;
    fs::write(out_dir.join("registry.json"), &out.registry_json)?;
    fs::write(out_dir.join("build-meta.json"), &out.build_meta_json)?;
    Ok(out)
}

/// Build registry + build-meta without writing (for tests).
pub fn compile(repo_root: &Path) -> Result<CompileOutput, CompileError> {
    let compiler_version = env!("CARGO_PKG_VERSION").to_string();
    let mut violations: Vec<Violation> = Vec::new();

    let spec_paths = discover_spec_paths(repo_root)?;
    for dir in missing_spec_md_dirs(repo_root)? {
        violations.push(Violation {
            code: "V-001".to_string(),
            severity: "error".to_string(),
            message: "spec.md missing for feature directory".to_string(),
            path: Some(normalize_repo_path(repo_root, &dir)),
        });
    }

    yaml_violations(repo_root, &mut violations);

    let mut features: Vec<FeatureRecord> = Vec::new();
    let mut seen_ids: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut alias_owner: BTreeMap<String, (String, String)> = BTreeMap::new();

    for spec_path in &spec_paths {
        let raw = fs::read_to_string(spec_path)?;
        let (yaml_val, body): (serde_yaml::Value, String) = split_frontmatter(&raw, spec_path)?;

        let fm = yaml_val
            .as_mapping()
            .ok_or_else(|| CompileError::InvalidFrontmatter {
                path: spec_path.clone(),
                msg: "frontmatter must be a mapping".into(),
            })?;

        let id = required_str(fm, "id", spec_path)?;
        let title = required_str(fm, "title", spec_path)?;
        let status = required_str(fm, "status", spec_path)?;
        let created = required_str(fm, "created", spec_path)?;
        let summary = required_str(fm, "summary", spec_path)?;

        if let Some(prev) = seen_ids.get(&id) {
            violations.push(Violation {
                code: "V-003".to_string(),
                severity: "error".to_string(),
                message: format!("duplicate feature id {id:?}"),
                path: Some(normalize_repo_path(repo_root, spec_path)),
            });
            violations.push(Violation {
                code: "V-003".to_string(),
                severity: "error".to_string(),
                message: format!("duplicate feature id {id:?} (first occurrence)"),
                path: Some(normalize_repo_path(repo_root, prev)),
            });
            continue;
        }
        seen_ids.insert(id.clone(), spec_path.clone());

        let rel = normalize_repo_path(repo_root, spec_path);
        let authors = optional_string_list(fm, "authors");
        let kind = optional_str(fm, "kind");
        let feature_branch = optional_str(fm, "feature_branch");
        let depends_on = optional_string_list(fm, "depends_on");
        let owner = optional_str(fm, "owner");
        let risk = optional_str(fm, "risk");
        let implementation = optional_str(fm, "implementation");
        if let Some(ref r) = risk {
            if !VALID_RISK_LEVELS.contains(&r.as_str()) {
                violations.push(Violation {
                    code: "V-007".to_string(),
                    severity: "error".to_string(),
                    message: format!(
                        "invalid risk value {r:?}; must be one of: low, medium, high, critical"
                    ),
                    path: Some(normalize_repo_path(repo_root, spec_path)),
                });
            }
        }
        let extra = extra_frontmatter(repo_root, fm, spec_path, &mut violations)?;

        let code_aliases = parse_code_aliases(
            fm,
            &id,
            repo_root,
            spec_path,
            &mut violations,
            &mut alias_owner,
        )?;

        // Spec 132 fields (V-011 input). Stored separately from extra_frontmatter
        // so the V-011 check has typed access without re-parsing.
        let amends = optional_string_list(fm, "amends").unwrap_or_default();
        let amends_sections = optional_string_list(fm, "amends_sections").unwrap_or_default();
        let unamendable = optional_string_list(fm, "unamendable").unwrap_or_default();

        // ── Spec 147 — universal dimensions + governance lifecycle ──
        let shape = optional_str(fm, "shape");
        let category = optional_string_list(fm, "category");
        let supersedes = optional_string_list(fm, "supersedes");
        let superseded_by = optional_str(fm, "superseded_by");
        let retirement_rationale = fm.get("retirement_rationale").and_then(yaml_to_json);
        // ── Spec 147 — per-kind structural fields ──
        let implements = parse_implements(fm);
        let provides = fm.get("provides").and_then(yaml_to_json);
        let selectable_by = optional_str(fm, "selectable_by");
        let composition = fm.get("composition").and_then(yaml_to_json);
        let selector = optional_str(fm, "selector");
        let default_value = optional_str(fm, "default");
        let production_forbidden = optional_string_list(fm, "production_forbidden");
        let member_contract = optional_str(fm, "member_contract");
        let identity = fm.get("identity").and_then(yaml_to_json);
        let selects = fm.get("selects").and_then(yaml_to_json);
        let policy = fm.get("policy").and_then(yaml_to_json);

        // ── V-012 (Spec 147): kind enum membership ──
        // Promoted to error severity in Phase 2 after corpus-wide
        // `kind:` backfill (spec 147 §Migration Phase 2).
        if let Some(ref k) = kind {
            if !VALID_KINDS.contains(&k.as_str()) {
                violations.push(Violation {
                    code: "V-012".to_string(),
                    severity: "error".to_string(),
                    message: format!(
                        "kind value {k:?} is not in the declared enum; expected one of: {}",
                        VALID_KINDS.join(", ")
                    ),
                    path: Some(normalize_repo_path(repo_root, spec_path)),
                });
            }
        }

        // ── V-013 (Spec 147): per-kind required fields ──
        // V-013 is silent on kind: amendment per spec 147 §V-013 prose:
        // amendment required-fields are governed by spec 119's amendment
        // convention (`amends:`, `amends_sections:`), not by per-kind
        // structural validation.
        if let Some(ref k) = kind {
            match k.as_str() {
                "capability" => {
                    let missing = collect_capability_missing(
                        &implements,
                        &provides,
                        &composition,
                        shape.as_deref(),
                    );
                    for m in &missing {
                        violations.push(Violation {
                            code: "V-013".to_string(),
                            severity: "warning".to_string(),
                            message: format!(
                                "kind=capability requires {m}; spec 147 §V-013 governs the required-field set"
                            ),
                            path: Some(normalize_repo_path(repo_root, spec_path)),
                        });
                    }
                }
                "registry" => {
                    if selector.is_none() {
                        violations.push(Violation {
                            code: "V-013".to_string(),
                            severity: "warning".to_string(),
                            message: "kind=registry requires `selector:`".to_string(),
                            path: Some(normalize_repo_path(repo_root, spec_path)),
                        });
                    }
                    if member_contract.is_none() {
                        violations.push(Violation {
                            code: "V-013".to_string(),
                            severity: "warning".to_string(),
                            message: "kind=registry requires `member_contract:`".to_string(),
                            path: Some(normalize_repo_path(repo_root, spec_path)),
                        });
                    }
                }
                "profile" => {
                    if identity.is_none() {
                        violations.push(Violation {
                            code: "V-013".to_string(),
                            severity: "warning".to_string(),
                            message: "kind=profile requires `identity:`".to_string(),
                            path: Some(normalize_repo_path(repo_root, spec_path)),
                        });
                    }
                    if selects.is_none() {
                        violations.push(Violation {
                            code: "V-013".to_string(),
                            severity: "warning".to_string(),
                            message: "kind=profile requires `selects:`".to_string(),
                            path: Some(normalize_repo_path(repo_root, spec_path)),
                        });
                    }
                    if !composition_has_requires(&composition) {
                        violations.push(Violation {
                            code: "V-013".to_string(),
                            severity: "warning".to_string(),
                            message: "kind=profile requires `composition.requires:`".to_string(),
                            path: Some(normalize_repo_path(repo_root, spec_path)),
                        });
                    }
                }
                _ => {}
            }
        }

        // ── V-014 (Spec 147): implements shape consistency ──
        // Scalar form is valid only for kind: capability. List form is
        // valid for every other kind (or no kind).
        if let Some(ref imp) = implements {
            let is_scalar = imp.is_string();
            let is_list = imp.is_array();
            let kind_is_capability = kind.as_deref() == Some("capability");
            if is_scalar && !kind_is_capability {
                violations.push(Violation {
                    code: "V-014".to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "implements: scalar form is reserved for kind=capability; this spec declares kind={:?}",
                        kind.as_deref().unwrap_or("(none)")
                    ),
                    path: Some(normalize_repo_path(repo_root, spec_path)),
                });
            } else if !is_scalar && !is_list {
                violations.push(Violation {
                    code: "V-014".to_string(),
                    severity: "warning".to_string(),
                    message: "implements: must be a scalar string (kind=capability) or a list of {path, primary?} items".to_string(),
                    path: Some(normalize_repo_path(repo_root, spec_path)),
                });
            }
        }

        // ── V-018 (Spec 147): retirement_rationale presence when status=retired ──
        // Promoted to error severity in Phase 4 — `retirement_rationale:`
        // is now a KNOWN_KEY top-level field, and any spec carrying
        // `status: retired` must declare it.
        if status == "retired" && retirement_rationale.is_none() {
            violations.push(Violation {
                code: "V-018".to_string(),
                severity: "error".to_string(),
                message: "status=retired requires `retirement_rationale:` frontmatter".to_string(),
                path: Some(normalize_repo_path(repo_root, spec_path)),
            });
        }

        let headings = extract_headings(&body, &title);

        features.push(FeatureRecord {
            amends,
            amends_sections,
            unamendable,
            id,
            title,
            status,
            created,
            summary,
            spec_path: rel,
            section_headings: headings,
            authors,
            kind,
            feature_branch,
            code_aliases,
            depends_on,
            owner,
            risk,
            implementation,
            shape,
            category,
            supersedes,
            superseded_by,
            retirement_rationale,
            implements,
            provides,
            selectable_by,
            composition,
            selector,
            default: default_value,
            production_forbidden,
            member_contract,
            identity,
            selects,
            policy,
            extra_frontmatter: extra,
        });
    }

    // V-002: required keys checked above; extra invalid types add violations in extra_frontmatter

    features.sort_by(|a, b| a.id.cmp(&b.id));

    // ── V-008: validate depends_on references resolve to existing IDs (102 FR-028) ──
    // depends_on may use short numeric prefixes (e.g. "089") or full slugs ("089-governed-convergence-plan").
    let all_ids: BTreeSet<String> = seen_ids.keys().cloned().collect();
    for f in &features {
        if let Some(ref deps) = f.depends_on {
            for dep in deps {
                let found = all_ids.contains(dep)
                    || all_ids.iter().any(|id| id.starts_with(&format!("{dep}-")));
                if !found {
                    violations.push(Violation {
                        code: "V-008".to_string(),
                        severity: "warning".to_string(),
                        message: format!("depends_on references non-existent spec id {dep:?}"),
                        path: Some(f.spec_path.clone()),
                    });
                }
            }
        }
    }

    // ── V-011: amends_sections must not overlap an amended spec's `unamendable` (spec 132) ──
    //
    // For every spec X with `amends: [Y, ...]` and a non-empty
    // `amends_sections`, look up each Y by id (or numeric prefix) and
    // check that the section anchors X claims to amend are not declared
    // unamendable in Y. An attempt to amend a frozen anchor is a hard
    // error: the amending spec must instead retire the amended spec
    // entirely (status: superseded) and replace it.
    {
        // Build a quick id-prefix map: short ids (e.g. "000") and full slugs
        // both resolve to the FeatureRecord owning the unamendable list.
        let mut by_id: BTreeMap<String, &FeatureRecord> = BTreeMap::new();
        for f in &features {
            by_id.insert(f.id.clone(), f);
            // Also index by leading numeric prefix (e.g. "000") so
            // `amends: ["000"]` resolves the same as `amends: ["000-bootstrap-spec-system"]`.
            if let Some((prefix, _)) = f.id.split_once('-') {
                if prefix.len() == 3 && prefix.chars().all(|c| c.is_ascii_digit()) {
                    by_id.entry(prefix.to_string()).or_insert(f);
                }
            }
        }
        for f in &features {
            if f.amends.is_empty() || f.amends_sections.is_empty() {
                continue;
            }
            for amended_id in &f.amends {
                let Some(amended) = by_id.get(amended_id.as_str()) else {
                    continue; // V-008 would have flagged this elsewhere
                };
                if amended.unamendable.is_empty() {
                    continue;
                }
                let frozen: BTreeSet<&String> = amended.unamendable.iter().collect();
                for section in &f.amends_sections {
                    if frozen.contains(section) {
                        violations.push(Violation {
                            code: "V-011".to_string(),
                            severity: "error".to_string(),
                            message: format!(
                                "spec {:?} amends section {:?} of spec {:?}, but that anchor is in {:?}'s unamendable list; \
                                 amending an unamendable section requires retiring the spec (status: superseded) and \
                                 replacing it with a successor",
                                f.id, section, amended.id, amended.id
                            ),
                            path: Some(f.spec_path.clone()),
                        });
                    }
                }
            }
        }
    }

    // ── Spec 147 — cross-spec validators (V-015, V-016, V-017, V-019) ──
    //
    // These follow the V-011 pattern: build an id-prefix index once,
    // then iterate features and emit diagnostics against the corpus.
    // V-015, V-016, V-017 are at warning severity in Phase 1 (and
    // remain so until a follow-on amendment exercises the new-kind
    // contract). V-019 was promoted to error severity in Phase 4
    // per spec 147 §Migration (the 4 superseded specs already carry
    // valid `superseded_by:` pointers).
    {
        let mut by_id: BTreeMap<String, &FeatureRecord> = BTreeMap::new();
        for f in &features {
            by_id.insert(f.id.clone(), f);
            if let Some((prefix, _)) = f.id.split_once('-') {
                if prefix.len() == 3 && prefix.chars().all(|c| c.is_ascii_digit()) {
                    by_id.entry(prefix.to_string()).or_insert(f);
                }
            }
        }
        let resolve = |id: &str| -> Option<&FeatureRecord> { by_id.get(id).copied() };

        // ── V-015: capability/registry link integrity (+ selectable_by equality) ──
        for f in &features {
            if f.kind.as_deref() != Some("capability") {
                continue;
            }
            let Some(Value::String(target_id)) = f.implements.as_ref() else {
                continue; // V-013/V-014 already covered missing or wrong shape
            };
            let Some(target) = resolve(target_id) else {
                violations.push(Violation {
                    code: "V-015".to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "kind=capability implements {target_id:?}, which does not resolve to an existing spec id"
                    ),
                    path: Some(f.spec_path.clone()),
                });
                continue;
            };
            if target.kind.as_deref() != Some("registry") {
                violations.push(Violation {
                    code: "V-015".to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "kind=capability implements {target_id:?}, whose kind={:?} (must be kind=registry)",
                        target.kind.as_deref().unwrap_or("(none)")
                    ),
                    path: Some(f.spec_path.clone()),
                });
                continue;
            }
            if let (Some(sb), Some(sel)) = (f.selectable_by.as_deref(), target.selector.as_deref()) {
                if sb != sel {
                    violations.push(Violation {
                        code: "V-015".to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "capability declares selectable_by={sb:?} but target registry {target_id:?} declares selector={sel:?}; values must match"
                        ),
                        path: Some(f.spec_path.clone()),
                    });
                }
            }
        }

        // ── V-016: corpus-wide primary-flag uniqueness ──
        // For any given path appearing under any spec's `implements:` list,
        // at most one spec across the corpus may declare `primary: true`
        // for it. Resolves spec 130 OQ-1 by making primary ownership a
        // typed, corpus-wide question with a deterministic answer.
        {
            let mut primary_owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for f in &features {
                let Some(Value::Array(items)) = f.implements.as_ref() else {
                    continue;
                };
                for item in items {
                    let Some(obj) = item.as_object() else {
                        continue;
                    };
                    let Some(path) = obj.get("path").and_then(|p| p.as_str()) else {
                        continue;
                    };
                    let is_primary = obj
                        .get("primary")
                        .and_then(|p| p.as_bool())
                        .unwrap_or(false);
                    if is_primary {
                        primary_owners
                            .entry(path.to_string())
                            .or_default()
                            .push(f.id.clone());
                    }
                }
            }
            for (path, owners) in &primary_owners {
                if owners.len() > 1 {
                    let mut sorted_owners = owners.clone();
                    sorted_owners.sort();
                    let joined = sorted_owners.join(", ");
                    for owner_id in &sorted_owners {
                        let owner_spec = match resolve(owner_id) {
                            Some(o) => o,
                            None => continue,
                        };
                        violations.push(Violation {
                            code: "V-016".to_string(),
                            severity: "warning".to_string(),
                            message: format!(
                                "path {path:?} has primary: true declared by multiple specs ({joined}); spec 147 V-016 requires at most one corpus-wide primary owner per path"
                            ),
                            path: Some(owner_spec.spec_path.clone()),
                        });
                    }
                }
            }
        }

        // ── V-017: profile selects-target validity ──
        for f in &features {
            if f.kind.as_deref() != Some("profile") {
                continue;
            }
            let Some(Value::Object(map)) = f.selects.as_ref() else {
                continue;
            };
            for (registry_id, cap_value) in map {
                let cap_id = match cap_value.as_str() {
                    Some(s) => s,
                    None => {
                        violations.push(Violation {
                            code: "V-017".to_string(),
                            severity: "warning".to_string(),
                            message: format!(
                                "profile selects[{registry_id:?}] must be a spec id string"
                            ),
                            path: Some(f.spec_path.clone()),
                        });
                        continue;
                    }
                };
                let registry_spec = resolve(registry_id);
                let cap_spec = resolve(cap_id);
                let registry_ok = registry_spec
                    .map(|r| r.kind.as_deref() == Some("registry"))
                    .unwrap_or(false);
                let cap_ok = cap_spec
                    .map(|c| c.kind.as_deref() == Some("capability"))
                    .unwrap_or(false);
                if !registry_ok {
                    violations.push(Violation {
                        code: "V-017".to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "profile selects key {registry_id:?} does not resolve to a kind=registry spec"
                        ),
                        path: Some(f.spec_path.clone()),
                    });
                    continue;
                }
                if !cap_ok {
                    violations.push(Violation {
                        code: "V-017".to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "profile selects[{registry_id:?}] = {cap_id:?} does not resolve to a kind=capability spec"
                        ),
                        path: Some(f.spec_path.clone()),
                    });
                    continue;
                }
                // Verify the capability implements the registry it's being
                // selected for (its implements: scalar matches the registry id
                // or its 3-digit prefix).
                let cap = cap_spec.expect("cap_ok implies Some");
                let cap_target = cap
                    .implements
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let registry_prefix = registry_id
                    .split_once('-')
                    .map(|(p, _)| p)
                    .unwrap_or(registry_id);
                let target_prefix = cap_target
                    .split_once('-')
                    .map(|(p, _)| p)
                    .unwrap_or(cap_target);
                let matches = cap_target == registry_id
                    || cap_target == registry_prefix
                    || target_prefix == registry_prefix;
                if !matches {
                    violations.push(Violation {
                        code: "V-017".to_string(),
                        severity: "warning".to_string(),
                        message: format!(
                            "profile selects[{registry_id:?}] = {cap_id:?}, but that capability implements {cap_target:?} (must implement the selected registry)"
                        ),
                        path: Some(f.spec_path.clone()),
                    });
                }
            }
        }

        // ── V-019: supersession back-link presence and resolution ──
        // Promoted to error severity in Phase 4 — the 4 superseded specs
        // (038, 040, 044, 088) already declare valid `superseded_by:`
        // pointers, and `superseded_by:` is now a KNOWN_KEY top-level
        // field. Any new spec carrying `status: superseded` must declare
        // it and the value must resolve to a corpus spec id.
        for f in &features {
            if f.status != "superseded" {
                continue;
            }
            match f.superseded_by.as_deref() {
                None => violations.push(Violation {
                    code: "V-019".to_string(),
                    severity: "error".to_string(),
                    message: "status=superseded requires `superseded_by:` frontmatter".to_string(),
                    path: Some(f.spec_path.clone()),
                }),
                Some(target_id) => {
                    if resolve(target_id).is_none() {
                        violations.push(Violation {
                            code: "V-019".to_string(),
                            severity: "error".to_string(),
                            message: format!(
                                "superseded_by {target_id:?} does not resolve to an existing spec id"
                            ),
                            path: Some(f.spec_path.clone()),
                        });
                    }
                }
            }
        }
    }

    // Cut D W-06c: Factory Build Spec discovery and OAP compliance
    // emission moved to `tools/oap-registry-enrich`. The generic spec
    // compiler no longer carries OAP-specific concepts.

    let passed = !violations.iter().any(|v| v.severity == "error");

    let content_hash = compute_content_hash(repo_root, &spec_paths)?;

    let registry_value = build_registry_value(
        &compiler_version,
        content_hash,
        &features,
        passed,
        &violations,
    )?;

    let registry_json = canonical_json_bytes(&registry_value)?;

    let built_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let build_meta = json!({
        "builtAt": built_at,
        "compilerId": COMPILER_ID,
        "compilerVersion": compiler_version,
    });
    let build_meta_json = canonical_json_bytes(&build_meta)?;

    Ok(CompileOutput {
        registry_json,
        build_meta_json,
        validation_passed: passed,
    })
}

#[derive(Serialize)]
struct FeatureRecord {
    id: String,
    title: String,
    status: String,
    created: String,
    summary: String,
    #[serde(rename = "specPath")]
    spec_path: String,
    #[serde(rename = "sectionHeadings")]
    section_headings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(rename = "featureBranch", skip_serializing_if = "Option::is_none")]
    feature_branch: Option<String>,
    #[serde(rename = "codeAliases", skip_serializing_if = "Option::is_none")]
    code_aliases: Option<Vec<String>>,
    #[serde(rename = "dependsOn", skip_serializing_if = "Option::is_none")]
    depends_on: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    risk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    implementation: Option<String>,
    /// Spec 132 — section anchors this spec amends in the spec(s) named
    /// in `amends:`. Validated against the amended spec's `unamendable`
    /// list (V-011). Empty → no amendment claim.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    amends: Vec<String>,
    #[serde(
        rename = "amendsSections",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    amends_sections: Vec<String>,
    /// Spec 132 — section anchors that future amendments cannot touch.
    /// Used by V-011: when another spec carries `amends: [<this id>]`
    /// AND `amends_sections:` overlaps this set, the amendment is
    /// rejected.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    unamendable: Vec<String>,
    // ── Spec 147 — kind-grammar universal dimensions ────────────────
    /// Optional kind-refinement; validated against `SHAPE_TABLE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<String>,
    /// Optional cross-cutting tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<Vec<String>>,
    /// Optional list of spec ids this spec replaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    supersedes: Option<Vec<String>>,
    /// Optional successor id; required by V-019 when status=superseded.
    #[serde(rename = "supersededBy", skip_serializing_if = "Option::is_none")]
    superseded_by: Option<String>,
    /// Optional structured retirement record; required by V-018 when status=retired.
    #[serde(rename = "retirementRationale", skip_serializing_if = "Option::is_none")]
    retirement_rationale: Option<Value>,
    // ── Spec 147 — implements promotion (serialized; shape disambiguated) ──
    /// Either a scalar registry-id (kind=capability) or a list of code-path claims.
    /// V-014 enforces shape consistency; V-015 enforces registry resolution;
    /// V-016 enforces corpus-wide `primary:` uniqueness.
    #[serde(skip_serializing_if = "Option::is_none")]
    implements: Option<Value>,
    // ── Spec 147 — capability/registry/profile per-kind structure ───
    #[serde(skip_serializing_if = "Option::is_none")]
    provides: Option<Value>,
    #[serde(rename = "selectableBy", skip_serializing_if = "Option::is_none")]
    selectable_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    composition: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
    #[serde(rename = "productionForbidden", skip_serializing_if = "Option::is_none")]
    production_forbidden: Option<Vec<String>>,
    #[serde(rename = "memberContract", skip_serializing_if = "Option::is_none")]
    member_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selects: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<Value>,
    #[serde(rename = "extraFrontmatter", skip_serializing_if = "Option::is_none")]
    extra_frontmatter: Option<Map<String, Value>>,
}

#[derive(Clone, Debug, Serialize)]
struct Violation {
    code: String,
    severity: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

fn build_registry_value(
    compiler_version: &str,
    content_hash: String,
    features: &[FeatureRecord],
    passed: bool,
    violations: &[Violation],
) -> Result<Value, CompileError> {
    let mut viol: Vec<Violation> = violations.to_vec();
    viol.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| a.message.cmp(&b.message))
            .then_with(|| a.path.as_deref().cmp(&b.path.as_deref()))
    });

    let features_val = serde_json::to_value(features)?;
    let viol_val = serde_json::to_value(&viol)?;

    // Cut D W-06c: factoryProjects + per-feature compliance lifted out
    // to oap-registry-enrich. The generic registry is now strictly
    // spec-corpus-derived; OAP-specific overlays live in
    // build/spec-registry/registry-oap.json.
    let registry = json!({
        "specVersion": SPEC_VERSION,
        "build": {
            "compilerId": COMPILER_ID,
            "compilerVersion": compiler_version,
            "inputRoot": ".",
            "contentHash": content_hash,
        },
        "features": features_val,
        "validation": {
            "passed": passed,
            "violations": viol_val,
        }
    });

    Ok(registry)
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, CompileError> {
    let sorted = sort_json_value(value.clone());
    let s = serde_json::to_string(&sorted)?;
    Ok(s.into_bytes())
}

fn sort_json_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out: BTreeMap<String, Value> = BTreeMap::new();
            for (k, val) in map {
                out.insert(k, sort_json_value(val));
            }
            let mut m = Map::new();
            for (k, v) in out {
                m.insert(k, v);
            }
            Value::Object(m)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sort_json_value).collect()),
        other => other,
    }
}

fn compute_content_hash(repo_root: &Path, spec_paths: &[PathBuf]) -> Result<String, CompileError> {
    let mut pieces: Vec<(String, Vec<u8>)> = Vec::new();
    for p in spec_paths {
        let raw = fs::read_to_string(p)?;
        let normalized = normalize_text(&raw);
        let rel = normalize_repo_path(repo_root, p);
        let mut buf = rel.as_bytes().to_vec();
        buf.push(0);
        buf.extend_from_slice(&normalized);
        pieces.push((rel, buf));
    }
    pieces.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (_, buf) in pieces {
        hasher.update(&buf);
    }
    Ok(hex_lower(&hasher.finalize()))
}

fn normalize_text(s: &str) -> Vec<u8> {
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    s.into_bytes()
}

fn normalize_repo_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `specs/<NNN>-<kebab>/` directory names per Feature 000 (three digits, hyphen, rest).
fn is_specs_feature_directory(name: &str) -> bool {
    let b = name.as_bytes();
    if b.len() < 5 {
        return false;
    }
    if !b[..3].iter().all(|u| u.is_ascii_digit()) {
        return false;
    }
    b[3] == b'-'
}

fn discover_spec_paths(repo_root: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let specs = repo_root.join("specs");
    if !specs.is_dir() {
        return Ok(vec![]);
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for ent in fs::read_dir(&specs)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !is_specs_feature_directory(name) {
            continue;
        }
        let spec_md = p.join("spec.md");
        if spec_md.is_file() {
            paths.push(spec_md);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Directories under `specs/<NNN>-<kebab>/` that exist but lack spec.md (V-001).
fn missing_spec_md_dirs(repo_root: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let specs = repo_root.join("specs");
    if !specs.is_dir() {
        return Ok(vec![]);
    }
    let mut missing = Vec::new();
    for ent in fs::read_dir(&specs)? {
        let p = ent?.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !is_specs_feature_directory(name) {
            continue;
        }
        if !p.join("spec.md").is_file() {
            missing.push(p);
        }
    }
    missing.sort();
    Ok(missing)
}

/// Standalone `.yaml` / `.yml` under the repo are rejected (V-004). Skipped path
/// components include `.git`, `.github` (CI workflows), build artifacts, etc.; see
/// Feature 001 research R6. Consolidated product/vendor trees (`apps/`, `crates/`, …)
/// are excluded from this scan — they are not the authored spec surface (V-004 targets
/// repo-authored YAML, not imported third-party or lockfile material).
fn yaml_violations(repo_root: &Path, violations: &mut Vec<Violation>) {
    let skip_dir_name = |name: &str| {
        matches!(
            name,
            ".git"
                | ".github"
                | "build"
                | "node_modules"
                | "vendor"
                | "target"
                | ".idea"
                | // Consolidated OPC / monorepo trees (not spec authoring surface)
                "apps"
                | "crates"
                | "factory"
                | "grammars"
                | "packages"
                | "platform"
                | "standards"
                | "_tmp"
        )
    };
    for ent in WalkDir::new(repo_root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            !skip_dir_name(name)
        })
        .filter_map(|e| e.ok())
    {
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        if v004_yaml_scan_exempt(repo_root, p) {
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "yaml" || ext == "yml" {
            violations.push(Violation {
                code: "V-004".to_string(),
                severity: "error".to_string(),
                message: "standalone authored YAML file is forbidden".to_string(),
                path: Some(normalize_repo_path(repo_root, p)),
            });
        }
    }
}

/// Lockfiles, workspace manifests, and tool config files at the repository root (e.g.
/// pnpm, SOPS) are not "standalone authored YAML" in the sense of V-004; they are
/// package-manager output, workspace glue, or tool-format config consumed by an
/// external CLI, not parallel spec registries. Spec 151 plan.md §"Constitution check"
/// records the rationale for `.sops.yaml` in particular.
fn v004_yaml_scan_exempt(repo_root: &Path, p: &Path) -> bool {
    // Files inside `.factory/` directories are indexed by factory scanning (074 FR-007).
    for ancestor in p.ancestors() {
        if let Some(name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if name == ".factory" {
                return true;
            }
        }
    }
    let Some(parent) = p.parent() else {
        return false;
    };
    if parent != repo_root {
        return false;
    }
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    matches!(
        name,
        "pnpm-workspace.yaml" | "pnpm-lock.yaml" | ".sops.yaml"
    )
}

// ── Factory Build Spec discovery (074 FR-007) ───────────────────────────────

fn split_frontmatter(raw: &str, path: &Path) -> Result<(serde_yaml::Value, String), CompileError> {
    split_frontmatter_required(raw).map_err(|err| match err {
        FrontmatterError::MissingFrontmatter => CompileError::MissingFrontmatter {
            path: path.to_path_buf(),
        },
        FrontmatterError::Yaml(e) => CompileError::Yaml(e),
    })
}

fn required_str(m: &serde_yaml::Mapping, key: &str, path: &Path) -> Result<String, CompileError> {
    let v = m.get(key).ok_or_else(|| CompileError::InvalidFrontmatter {
        path: path.to_path_buf(),
        msg: format!("missing required key {key:?}"),
    })?;
    v.as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| CompileError::InvalidFrontmatter {
            path: path.to_path_buf(),
            msg: format!("key {key:?} must be a string"),
        })
}

fn optional_str(m: &serde_yaml::Mapping, key: &str) -> Option<String> {
    m.get(key)?.as_str().map(|s| s.to_string())
}

fn optional_string_list(m: &serde_yaml::Mapping, key: &str) -> Option<Vec<String>> {
    let v = m.get(key)?;
    let arr = v.as_sequence()?;
    let mut out = Vec::new();
    for x in arr {
        out.push(x.as_str()?.to_string());
    }
    Some(out)
}

/// Token shape aligned with `featuregraph` / `registry.schema.json` `codeAliases` items.
fn is_valid_code_alias(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 3 || b.len() > 64 {
        return false;
    }
    if !b[0].is_ascii_uppercase() {
        return false;
    }
    b[1..]
        .iter()
        .all(|&c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_')
}

fn parse_code_aliases(
    fm: &serde_yaml::Mapping,
    feature_id: &str,
    repo_root: &Path,
    spec_path: &Path,
    violations: &mut Vec<Violation>,
    alias_owner: &mut BTreeMap<String, (String, String)>,
) -> Result<Option<Vec<String>>, CompileError> {
    let Some(raw) = fm.get("code_aliases") else {
        return Ok(None);
    };
    let Some(seq) = raw.as_sequence() else {
        violations.push(Violation {
            code: "V-002".to_string(),
            severity: "error".to_string(),
            message: "code_aliases must be a list of strings".into(),
            path: Some(normalize_repo_path(repo_root, spec_path)),
        });
        return Ok(None);
    };
    if seq.is_empty() {
        return Ok(None);
    }

    let mut seen_in_feature: BTreeSet<String> = BTreeSet::new();
    let mut out: Vec<String> = Vec::new();

    for entry in seq {
        let Some(s) = entry.as_str() else {
            violations.push(Violation {
                code: "V-002".to_string(),
                severity: "error".to_string(),
                message: "code_aliases must be a list of strings".into(),
                path: Some(normalize_repo_path(repo_root, spec_path)),
            });
            continue;
        };
        if !is_valid_code_alias(s) {
            violations.push(Violation {
                code: "V-006".to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "code_aliases entry {s:?} does not match pattern ^[A-Z][A-Z0-9_]{{2,63}}$"
                ),
                path: Some(normalize_repo_path(repo_root, spec_path)),
            });
            continue;
        }
        if !seen_in_feature.insert(s.to_string()) {
            continue;
        }
        if let Some((prev_id, prev_path)) = alias_owner.get(s) {
            if prev_id != feature_id {
                violations.push(Violation {
                    code: "V-005".to_string(),
                    severity: "error".to_string(),
                    message: format!("code alias {s:?} is already claimed by feature {prev_id:?}"),
                    path: Some(normalize_repo_path(repo_root, spec_path)),
                });
                violations.push(Violation {
                    code: "V-005".to_string(),
                    severity: "error".to_string(),
                    message: format!(
                        "code alias {s:?} in feature {prev_id:?} is duplicated by feature {feature_id:?}"
                    ),
                    path: Some(prev_path.clone()),
                });
                continue;
            }
        } else {
            alias_owner.insert(
                s.to_string(),
                (
                    feature_id.to_string(),
                    normalize_repo_path(repo_root, spec_path),
                ),
            );
        }
        out.push(s.to_string());
    }

    if out.is_empty() {
        Ok(None)
    } else {
        out.sort();
        Ok(Some(out))
    }
}

fn extra_frontmatter(
    repo_root: &Path,
    m: &serde_yaml::Mapping,
    path: &Path,
    violations: &mut Vec<Violation>,
) -> Result<Option<Map<String, Value>>, CompileError> {
    let mut extra = Map::new();
    for (k, v) in m.iter() {
        let key = k.as_str().ok_or_else(|| CompileError::InvalidFrontmatter {
            path: path.to_path_buf(),
            msg: "frontmatter keys must be strings".into(),
        })?;
        if KNOWN_KEYS.contains(&key) {
            continue;
        }
        match yaml_scalar_to_json(v) {
            Some(j) => {
                extra.insert(key.to_string(), j);
            }
            None => {
                violations.push(Violation {
                    code: "V-002".to_string(),
                    severity: "error".to_string(),
                    message: format!(
                        "frontmatter key {key:?} has a value that cannot be represented in extraFrontmatter"
                    ),
                    path: Some(normalize_repo_path(repo_root, path)),
                });
            }
        }
    }
    if extra.len() > 8 {
        violations.push(Violation {
            code: "V-002".to_string(),
            severity: "error".to_string(),
            message: "extraFrontmatter exceeds maxProperties (8)".into(),
            path: Some(normalize_repo_path(repo_root, path)),
        });
    }
    if extra.is_empty() {
        Ok(None)
    } else {
        Ok(Some(extra))
    }
}

/// Spec 147 — return the list of missing required fields for a
/// `kind: capability` spec. Implements V-013's capability path including
/// the web-snippet shape linkage (capability + shape: web-snippet
/// requires at least one `provides.registrations[].kind: web-snippet`).
fn collect_capability_missing(
    implements: &Option<Value>,
    provides: &Option<Value>,
    composition: &Option<Value>,
    shape: Option<&str>,
) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    if implements.is_none() {
        missing.push("`implements:` (registry-id scalar)".into());
    }
    if provides.is_none() {
        missing.push("`provides:`".into());
    }
    if !composition_has_requires(composition) {
        missing.push("`composition.requires:`".into());
    }
    if shape == Some("web-snippet")
        && !provides_has_registration_kind(provides, "web-snippet")
    {
        missing.push(
            "at least one `provides.registrations[].kind: web-snippet` when shape=web-snippet"
                .into(),
        );
    }
    missing
}

/// True when `composition.requires` exists and is a non-empty array.
fn composition_has_requires(composition: &Option<Value>) -> bool {
    composition
        .as_ref()
        .and_then(|c| c.get("requires"))
        .and_then(|r| r.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

/// True when `provides.registrations[]` contains an entry with `kind == target`.
fn provides_has_registration_kind(provides: &Option<Value>, target: &str) -> bool {
    let Some(p) = provides else {
        return false;
    };
    let Some(regs) = p.get("registrations").and_then(|r| r.as_array()) else {
        return false;
    };
    regs.iter().any(|r| {
        r.get("kind").and_then(|k| k.as_str()) == Some(target)
    })
}

/// Spec 147 — recursive YAML → JSON conversion for nested frontmatter
/// fields (`provides`, `composition`, `identity`, `retirement_rationale`,
/// `implements` list form, `selects`, `policy`). Unlike `yaml_scalar_to_json`
/// this descends into mappings and heterogeneous sequences. Returns
/// `None` if a value can't be represented (e.g. tagged YAML).
fn yaml_to_json(v: &serde_yaml::Value) -> Option<Value> {
    match v {
        serde_yaml::Value::Null => Some(Value::Null),
        serde_yaml::Value::Bool(b) => Some(Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Some(Value::Number(i.into()));
            }
            if let Some(u) = n.as_u64() {
                return Some(Value::Number(u.into()));
            }
            let f = n.as_f64()?;
            serde_json::Number::from_f64(f).map(Value::Number)
        }
        serde_yaml::Value::String(s) => Some(Value::String(s.clone())),
        serde_yaml::Value::Sequence(seq) => {
            let mut arr = Vec::with_capacity(seq.len());
            for x in seq {
                arr.push(yaml_to_json(x)?);
            }
            Some(Value::Array(arr))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = Map::new();
            for (k, val) in map {
                let key = k.as_str()?.to_string();
                obj.insert(key, yaml_to_json(val)?);
            }
            Some(Value::Object(obj))
        }
        serde_yaml::Value::Tagged(_) => None,
    }
}

/// Spec 147 — parse the optional `implements:` field, preserving scalar
/// vs list distinction. Scalar form is reserved for `kind: capability`
/// (V-014); list form carries `{path, primary?}` items.
fn parse_implements(m: &serde_yaml::Mapping) -> Option<Value> {
    yaml_to_json(m.get("implements")?)
}

fn yaml_scalar_to_json(v: &serde_yaml::Value) -> Option<Value> {
    match v {
        serde_yaml::Value::String(s) => Some(Value::String(s.clone())),
        serde_yaml::Value::Bool(b) => Some(Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Some(Value::Number(i.into()));
            }
            let f = n.as_f64()?;
            Some(Value::Number(serde_json::Number::from_f64(f)?))
        }
        serde_yaml::Value::Null => Some(Value::Null),
        serde_yaml::Value::Sequence(seq) => {
            let mut arr = Vec::new();
            for x in seq {
                arr.push(x.as_str()?.to_string());
            }
            if arr.len() > 64 {
                return None;
            }
            Some(Value::Array(arr.into_iter().map(Value::String).collect()))
        }
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Tagged(_) => None,
    }
}

/// ATX `#` / `##` headings only; first heading equal to `title` is dropped (see README).
pub fn extract_headings(body: &str, title: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim_start();
        if let Some(h) = atx_h2(t) {
            out.push(h.to_string());
            continue;
        }
        if let Some(h) = atx_h1(t) {
            out.push(h.to_string());
        }
    }
    if let Some(first) = out.first() {
        if first.trim() == title.trim() {
            out.remove(0);
        }
    }
    out
}

fn atx_h1(line: &str) -> Option<&str> {
    if !line.starts_with('#') {
        return None;
    }
    if line.starts_with("##") {
        return None;
    }
    line.strip_prefix("# ").map(str::trim_end)
}

fn atx_h2(line: &str) -> Option<&str> {
    if !line.starts_with("##") {
        return None;
    }
    if line.starts_with("###") {
        return None;
    }
    line.strip_prefix("## ").map(str::trim_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_skip_title_duplicate() {
        let body = "# Feature X\n\n## A\n## B\n";
        let h = extract_headings(body, "Feature X");
        assert_eq!(h, vec!["A", "B"]);
    }

    #[test]
    fn feature_dir_name_matches_feature_000() {
        assert!(is_specs_feature_directory("000-bootstrap-spec-system"));
        assert!(is_specs_feature_directory("001-spec-compiler-mvp"));
        assert!(!is_specs_feature_directory("001"));
        assert!(!is_specs_feature_directory("docs"));
        assert!(!is_specs_feature_directory("00a-x"));
    }

    // Cut D W-06c: factory-project parser and scan-skip-dir unit
    // tests moved with the factory_projects integration test to
    // tools/oap-registry-enrich/tests/. The generic spec-compiler no
    // longer carries .factory/ scanning logic.
}
