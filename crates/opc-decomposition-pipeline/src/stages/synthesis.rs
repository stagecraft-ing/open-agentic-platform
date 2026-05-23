// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
//
// Stage 6 — deterministic baseline synthesiser. Consumes the outputs
// of stages 2 (fingerprint) and 3 (clusters) and emits one draft
// spec.md per cluster under s6-synthesis/specs/<slug>/spec.md.
//
// Every emitted spec satisfies:
//
//   - FR-005 (spec 147 kind grammar): declared `kind: capability`.
//   - FR-006 (spec 154 logical-unit grammar): each `establishes:`
//     entry carries `{ unit: { kind, path } }`.
//   - FR-004 (spec 161 emission contract): exactly one
//     `references:` entry with `role: decomposition-origin` and a
//     `provenance:` block bearing `kind: code-fingerprint`, the
//     stage-2 fingerprint hash as `source:`, and `derived_at:` set
//     to the stage's started_at.
//
// The LLM swap (follow-up F-001) replaces this module's `synthesise`
// function while keeping the on-disk shape identical.

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};

use crate::error::PipelineError;
use crate::persistence::{RunDirectory, hash_file, hash_stage_dir};
use crate::stages::clustering;
use crate::stages::fingerprint as fp;
use crate::types::{
    Cluster, DegradedReason, DraftSpecRef, PipelineConfig, StageId, StageRecord, StageStatus,
};

pub struct SynthesisOutput {
    pub record: StageRecord,
    pub emitted: Vec<DraftSpecRef>,
}

pub fn run(
    _config: &PipelineConfig,
    run_dir: &RunDirectory,
) -> Result<SynthesisOutput, PipelineError> {
    let started_at = Utc::now();
    let stage_dir = run_dir.stage_dir(StageId::Synthesis);
    let specs_dir = run_dir.synthesis_specs_dir();
    fs::create_dir_all(&specs_dir).map_err(|e| PipelineError::io(&specs_dir, e))?;

    let fingerprint = fp::load_fingerprint(run_dir)?;
    let clusters = clustering::load_clusters(run_dir)?;

    let mut emitted: Vec<DraftSpecRef> = Vec::new();
    let mut degraded: Option<DegradedReason> = None;
    if clusters.clusters.is_empty() {
        degraded = Some(DegradedReason::EmptyProjectTree);
    }

    for cluster in &clusters.clusters {
        if cluster.paths.is_empty() {
            continue;
        }
        let slug = slug_for(cluster);
        let dir = specs_dir.join(&slug);
        fs::create_dir_all(&dir).map_err(|e| PipelineError::io(&dir, e))?;
        let spec_path = dir.join("spec.md");
        let body = render_spec(cluster, &fingerprint.hash, started_at);
        fs::write(&spec_path, &body).map_err(|e| PipelineError::io(&spec_path, e))?;

        let content_hash = hash_file(&spec_path)?;
        let relpath = make_relpath(&specs_dir, &spec_path, &stage_dir);
        emitted.push(DraftSpecRef {
            slug,
            relpath,
            content_hash,
        });
    }

    let content_hash = hash_stage_dir(&stage_dir)?;
    let status = if emitted.is_empty() {
        StageStatus::Degraded
    } else if degraded.is_some() {
        StageStatus::Degraded
    } else {
        StageStatus::Complete
    };
    let record = StageRecord {
        id: StageId::Synthesis,
        status,
        content_hash,
        output_relpath: StageId::Synthesis.dir_name(),
        started_at,
        completed_at: Utc::now(),
        degraded,
    };
    Ok(SynthesisOutput { record, emitted })
}

fn make_relpath(_specs_dir: &std::path::Path, spec_path: &std::path::Path, stage_dir: &std::path::Path) -> String {
    spec_path
        .strip_prefix(stage_dir)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| spec_path.to_string_lossy().into_owned())
}

fn slug_for(cluster: &Cluster) -> String {
    // Use a 999- prefix to flag "needs renumbering at promotion". Slug
    // body kebab-cases the cluster root so a human glancing at staging
    // can match emitted specs back to clusters without opening files.
    let sanitised = sanitise(&cluster.root_dir);
    format!("999-decomposed-{}-{}", sanitised, cluster.id)
}

fn sanitise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("root");
    }
    out
}

fn render_spec(cluster: &Cluster, fingerprint_hash: &str, started_at: DateTime<Utc>) -> String {
    let slug = slug_for(cluster);
    let id = slug.clone();
    let derived_at = started_at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let created = started_at.format("%Y-%m-%d").to_string();

    let mut establishes = String::new();
    for path in &cluster.paths {
        establishes.push_str("  - unit: { kind: file, path: ");
        establishes.push_str(&yaml_path(path));
        establishes.push_str(" }\n");
    }

    let mut sources_md = String::new();
    for path in &cluster.paths {
        sources_md.push_str("- `");
        sources_md.push_str(path);
        sources_md.push_str("`\n");
    }

    format!(
        "---\n\
id: \"{id}\"\n\
slug: {slug}\n\
title: \"Decomposed unit from cluster {cluster_id} rooted at {root}\"\n\
status: draft\n\
implementation: pending\n\
owner: opc-decomposition-pipeline\n\
created: \"{created}\"\n\
kind: capability\n\
risk: medium\n\
origin:\n  retroactive: true\n\
summary: >\n  Draft spec emitted by the OPC decomposition pipeline (spec 165)\n\
  from cluster {cluster_id} rooted at {root}. Synthesised from\n\
  {n_files} file(s) using the deterministic baseline synthesiser; an\n\
  LLM swap (spec 165 follow-up F-001) will replace the prose without\n\
  changing the contract. Review, rename, and renumber before promotion.\n\
establishes:\n{establishes}\
references:\n  - role: decomposition-origin\n    provenance:\n      kind: code-fingerprint\n      source: \"{fingerprint_hash}\"\n      derived_at: \"{derived_at}\"\n\
---\n\n\
# {id} — Decomposed unit from cluster {cluster_id}\n\n\
This draft spec was synthesised by the OPC decomposition pipeline\n\
(spec 165) from the following project artifacts. The deterministic\n\
baseline synthesiser produces this scaffold so a developer can\n\
review, refine, and promote it into the project's spec spine.\n\n\
## Cluster\n\n\
- **ID:** {cluster_id}\n\
- **Root:** `{root}`\n\
- **Summary (from stage 3):** {cluster_summary}\n\
- **Files:** {n_files}\n\n\
## Source files (logical units)\n\n\
{sources_md}\n\
## Provenance\n\n\
The `references:` edge above carries `role: decomposition-origin`\n\
with `provenance.kind: code-fingerprint` per spec 161 §2.1. The\n\
`source:` field is the xray structural fingerprint of the project\n\
at synthesis time (stage 2 of the spec-165 pipeline); see\n\
`crates/xray::fingerprint`.\n\n\
## Next steps\n\n\
1. Rename the spec to a meaningful slug.\n\
2. Renumber to fit the project's spec sequence.\n\
3. Replace the auto-generated summary with intent-first prose.\n\
4. Confirm or narrow the `establishes:` paths.\n\
5. Add `kind: capability` refinements (`shape:`, `category:`) per\n\
   spec 147 if the project's grammar uses them.\n",
        id = id,
        slug = slug,
        cluster_id = cluster.id,
        root = cluster.root_dir,
        n_files = cluster.paths.len(),
        cluster_summary = cluster.summary,
        fingerprint_hash = fingerprint_hash,
        derived_at = derived_at,
        created = created,
        establishes = establishes,
        sources_md = sources_md,
    )
}

/// Wrap a path in double quotes if it contains characters that would
/// confuse the YAML flow scalar `unit: { kind: file, path: <here> }`.
fn yaml_path(p: &str) -> String {
    let needs_quotes = p.chars().any(|c| matches!(c, ',' | '{' | '}' | '[' | ']' | ':' | '#' | '"'));
    if needs_quotes {
        format!("\"{}\"", p.replace('"', "\\\""))
    } else {
        p.to_string()
    }
}

#[allow(dead_code)] // used for testing in integration tests
pub fn list_emitted_specs(run_dir: &RunDirectory) -> Result<Vec<PathBuf>, PipelineError> {
    let specs_dir = run_dir.synthesis_specs_dir();
    let mut out = Vec::new();
    if !specs_dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(&specs_dir).map_err(|e| PipelineError::io(&specs_dir, e))? {
        let entry = entry.map_err(|e| PipelineError::io(&specs_dir, e))?;
        let spec_md = entry.path().join("spec.md");
        if spec_md.is_file() {
            out.push(spec_md);
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use crate::stages::{clustering, fingerprint};
    use crate::types::{PipelineConfig, RunId};

    fn fresh_run_dir(out: &std::path::Path) -> RunDirectory {
        let rid = RunId(String::from("test-synth"));
        let d = RunDirectory::new(out, rid);
        d.ensure().unwrap();
        d
    }

    fn prep(project: &std::path::Path, out: &std::path::Path) -> (PipelineConfig, RunDirectory) {
        let crates_a = project.join("crates").join("a");
        let tools = project.join("tools");
        fs::create_dir_all(&crates_a).unwrap();
        fs::create_dir_all(&tools).unwrap();
        fs::write(crates_a.join("lib.rs"), "fn a(){}\n").unwrap();
        fs::write(tools.join("main.rs"), "fn main(){}\n").unwrap();

        let cfg = PipelineConfig::new(project);
        let rd = fresh_run_dir(out);
        fingerprint::run(&cfg, &rd).unwrap();
        clustering::run(&cfg, &rd).unwrap();
        (cfg, rd)
    }

    #[test]
    fn synthesises_one_spec_per_cluster() {
        let project = tempdir().unwrap();
        let out = tempdir().unwrap();
        let (cfg, rd) = prep(project.path(), out.path());
        let synth = run(&cfg, &rd).unwrap();
        assert!(!synth.emitted.is_empty());
        for r in &synth.emitted {
            assert!(r.slug.starts_with("999-decomposed-"));
        }
        let files = list_emitted_specs(&rd).unwrap();
        assert!(!files.is_empty());
        let body = fs::read_to_string(&files[0]).unwrap();
        assert!(body.contains("role: decomposition-origin"));
        assert!(body.contains("kind: code-fingerprint"));
        assert!(body.contains("kind: capability"));
        assert!(body.contains("retroactive: true"));
        assert!(body.contains("establishes:"));
        assert!(body.contains("- unit: { kind: file, path:"));
    }

    #[test]
    fn handles_empty_project_tree() {
        let project = tempdir().unwrap();
        // Project has nothing — fingerprint will report file_count=0,
        // clustering will produce no clusters, synthesis will produce no specs.
        let out = tempdir().unwrap();
        let cfg = PipelineConfig::new(project.path());
        let rd = fresh_run_dir(out.path());
        fingerprint::run(&cfg, &rd).unwrap();
        clustering::run(&cfg, &rd).unwrap();

        let synth = run(&cfg, &rd).unwrap();
        assert!(synth.emitted.is_empty());
        assert_eq!(synth.record.status, StageStatus::Degraded);
    }
}
