//! CI parity drift-check (spec 104).
//!
//! For every enforcing GitHub Actions workflow, extract significant command
//! tokens from each step's `run:` block and assert they appear in the root
//! Makefile. Drift means the Makefile has fallen behind CI — a gate exists
//! in CI that `make ci` does not mirror.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Workflow {
    jobs: BTreeMap<String, Job>,
}

#[derive(Debug, Deserialize, Default)]
struct Job {
    #[serde(default)]
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize, Default)]
struct Step {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    run: Option<String>,
}

/// Workflows whose gates `make ci` must mirror. Order is stable for reporting.
/// Keep in sync with spec 104 §2.2.
pub const ENFORCING_WORKFLOWS: &[&str] = &[
    "ci-axiomregent.yml",
    "ci-codebase-index.yml",
    "ci-crates.yml",
    "ci-deployd-api-rs.yml",
    "ci-desktop.yml",
    "ci-orchestrator.yml",
    "ci-policy-kernel.yml",
    "ci-spec-code-coupling.yml",
    "ci-stagecraft.yml",
    "ci-supply-chain.yml",
    "spec-conformance.yml",
];

/// Lines that appear in an enforcing workflow but have no local analogue.
/// Each entry MUST carry a one-line rationale. If this grows past ~5 entries,
/// spec 104 must be revisited (FR-06, SC-04).
const ALLOW_LIST: &[&str] = &[
    // ci-desktop.yml creates a sidecar binary stub named for the CI runner's
    // host triple (aarch64-apple-darwin on macOS runners). The Makefile's
    // ci-desktop target creates the same stub using the local host triple
    // detected at runtime — not byte-identical, but equivalent in intent.
    "axiomregent-aarch64-apple-darwin",
];

#[derive(Debug, Clone)]
pub struct Drift {
    pub workflow: String,
    pub job: String,
    pub step: String,
    pub missing_token: String,
    pub source_line: String,
}

/// Run the parity check against `repo_root`. Returns `Ok(vec![])` when the
/// Makefile mirrors every significant `run:` token across every enforcing
/// workflow.
pub fn check_parity(repo_root: &Path) -> Result<Vec<Drift>, String> {
    let makefile = fs::read_to_string(repo_root.join("Makefile"))
        .map_err(|e| format!("reading Makefile: {e}"))?;
    let workflows_dir = repo_root.join(".github").join("workflows");

    let mut drifts = Vec::new();
    for wf_name in ENFORCING_WORKFLOWS {
        let wf_path = workflows_dir.join(wf_name);
        let content = fs::read_to_string(&wf_path)
            .map_err(|e| format!("reading {}: {e}", wf_path.display()))?;
        let wf: Workflow = serde_yaml::from_str(&content)
            .map_err(|e| format!("parsing {}: {e}", wf_path.display()))?;

        for (job_name, job) in &wf.jobs {
            for step in &job.steps {
                let Some(run) = &step.run else { continue };
                let step_name = step.name.clone().unwrap_or_else(|| "<unnamed>".to_string());

                for raw_line in run.lines() {
                    let line = normalise(raw_line);
                    if line.is_empty() {
                        continue;
                    }
                    if allow_list_suppresses(&line) {
                        continue;
                    }
                    let tokens = significant_tokens(&line);
                    if tokens.is_empty() {
                        continue;
                    }
                    for token in &tokens {
                        if !makefile.contains(token) {
                            drifts.push(Drift {
                                workflow: (*wf_name).to_string(),
                                job: job_name.clone(),
                                step: step_name.clone(),
                                missing_token: token.clone(),
                                source_line: line.clone(),
                            });
                            break; // report first missing token per line
                        }
                    }
                }
            }
        }
    }
    Ok(drifts)
}

fn allow_list_suppresses(line: &str) -> bool {
    ALLOW_LIST.iter().any(|entry| line.contains(entry))
}

// ─────────────────────────────────────────────────────────────────────────────
// Precondition check (spec 104 fix 4): fresh-clone execution parity.
//
// `ci-parity-check` base pass guarantees command equality between workflow
// and Makefile. It does NOT guarantee that running those commands on a
// fresh clone yields the same result as running them in a dev workspace.
//
// The concrete case that prompted this: `spec-conformance.yml` runs
// `codebase-indexer check` as step 11, BEFORE `codebase-indexer compile`
// at step 12. The tool reads `build/codebase-index/index.json`. On a dev
// machine that file exists as a residue of prior runs; on CI's fresh
// clone it doesn't. The step fails with ENOENT on CI while `make ci-tools`
// passes locally.
//
// The rule: any step that invokes a "consumer" of a governed artifact
// under `build/` MUST be preceded (in the same job) by a "producer" of
// that artifact, OR the artifact MUST be tracked in git. Otherwise the
// CI runner has nothing to feed the consumer and will error.
// ─────────────────────────────────────────────────────────────────────────────

struct ConsumerRule {
    pattern: &'static str,
    artifact: &'static str,
}

struct ProducerRule {
    pattern: &'static str,
    artifact: &'static str,
}

/// Commands known to READ a governed artifact under `build/`.
/// Extend as new tools are added to the governed-read surface.
const CONSUMERS: &[ConsumerRule] = &[
    ConsumerRule {
        pattern: "codebase-indexer check",
        artifact: "build/codebase-index/index.json",
    },
    ConsumerRule {
        pattern: "codebase-indexer render",
        artifact: "build/codebase-index/index.json",
    },
    ConsumerRule {
        pattern: "registry-consumer list",
        artifact: "build/spec-registry/registry.json",
    },
    ConsumerRule {
        pattern: "registry-consumer show",
        artifact: "build/spec-registry/registry.json",
    },
    ConsumerRule {
        pattern: "registry-consumer status-report",
        artifact: "build/spec-registry/registry.json",
    },
    ConsumerRule {
        pattern: "registry-consumer compliance-report",
        artifact: "build/spec-registry/registry.json",
    },
];

/// Commands known to WRITE a governed artifact under `build/`.
const PRODUCERS: &[ProducerRule] = &[
    ProducerRule {
        pattern: "spec-compiler compile",
        artifact: "build/spec-registry/registry.json",
    },
    ProducerRule {
        pattern: "codebase-indexer compile",
        artifact: "build/codebase-index/index.json",
    },
    ProducerRule {
        pattern: "adapter-scopes-compiler",
        artifact: "build/adapter-scopes.json",
    },
];

#[derive(Debug, Clone)]
pub struct PreconditionDrift {
    pub workflow: String,
    pub job: String,
    pub step: String,
    pub missing_artifact: String,
    pub consumer_line: String,
}

/// For every enforcing workflow, assert each governed-artifact consumer
/// has its artifact either (a) produced by an earlier step in the same job,
/// or (b) tracked in git.
pub fn check_preconditions(repo_root: &Path) -> Result<Vec<PreconditionDrift>, String> {
    let tracked = load_tracked_files(repo_root)?;
    let workflows_dir = repo_root.join(".github").join("workflows");
    let mut drifts = Vec::new();

    for wf_name in ENFORCING_WORKFLOWS {
        let wf_path = workflows_dir.join(wf_name);
        let content = fs::read_to_string(&wf_path)
            .map_err(|e| format!("reading {}: {e}", wf_path.display()))?;
        let wf: Workflow = serde_yaml::from_str(&content)
            .map_err(|e| format!("parsing {}: {e}", wf_path.display()))?;

        for (job_name, job) in &wf.jobs {
            let mut produced: BTreeSet<String> = BTreeSet::new();
            for step in &job.steps {
                let Some(run) = &step.run else { continue };
                let step_name = step.name.clone().unwrap_or_else(|| "<unnamed>".to_string());

                for raw_line in run.lines() {
                    let line = raw_line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(c) = CONSUMERS.iter().find(|c| line.contains(c.pattern)) {
                        let covered_by_earlier_step = produced.contains(c.artifact);
                        let covered_by_git = tracked.contains(c.artifact);
                        if !covered_by_earlier_step && !covered_by_git {
                            drifts.push(PreconditionDrift {
                                workflow: (*wf_name).to_string(),
                                job: job_name.clone(),
                                step: step_name.clone(),
                                missing_artifact: c.artifact.to_string(),
                                consumer_line: line.to_string(),
                            });
                        }
                    }
                }

                for raw_line in run.lines() {
                    if let Some(p) = PRODUCERS.iter().find(|p| raw_line.contains(p.pattern)) {
                        produced.insert(p.artifact.to_string());
                    }
                }
            }
        }
    }
    Ok(drifts)
}

fn load_tracked_files(repo_root: &Path) -> Result<BTreeSet<String>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["ls-files"])
        .output()
        .map_err(|e| format!("spawn git ls-files: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files exited with status {:?}",
            output.status.code()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect())
}

/// Normalise a raw `run:` line: strip whitespace, line-continuation slash,
/// trailing `# comment`, and obvious shell suffixes.
fn normalise(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if s.ends_with('\\') {
        s.pop();
        s = s.trim_end().to_string();
    }
    if let Some(idx) = s.find(" # ") {
        s.truncate(idx);
    }
    for suffix in [" || true", " 2>&1", " > /dev/null"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
        }
    }
    s.trim().to_string()
}

/// Extract the tokens we expect to find mirrored in the Makefile.
/// Returns empty for lines that aren't validation commands (shell control
/// flow, variable assignments, echo/cd/grep preambles, etc.).
pub fn significant_tokens(line: &str) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }
    let cmd = words[0];
    let significant = matches!(cmd, "cargo" | "pnpm" | "npm" | "npx" | "node")
        || cmd.starts_with("./tools/");
    if !significant {
        return vec![];
    }
    let mut out = vec![cmd.to_string()];
    let mut i = 1usize;
    while i < words.len() {
        let w = words[i];
        // `--` separator — cargo clippy passes flags after it; skip the marker.
        if w == "--" {
            i += 1;
            continue;
        }
        // GitHub Actions matrix/expression substitution (`${{ matrix.x }}`)
        // can't possibly be a literal token in the Makefile. Strip the three
        // whitespace-split fragments together so the Makefile recipe (which
        // uses `$$VAR` or an explicit list) is still considered a valid mirror.
        if w == "${{" {
            // Skip until we see the closing `}}`.
            let mut j = i + 1;
            while j < words.len() && words[j] != "}}" {
                j += 1;
            }
            i = j + 1;
            continue;
        }
        // Flags that consume the next token as their value.
        if matches!(w, "--manifest-path" | "--target" | "--all" | "--filter") {
            if let Some(v) = words.get(i + 1) {
                // If the value is a `${{ … }}` expression, skip the flag entirely
                // — it can't be mirrored as a literal token. The Makefile is
                // expected to provide an equivalent loop or per-target recipe.
                if *v == "${{" {
                    let mut j = i + 2;
                    while j < words.len() && words[j] != "}}" {
                        j += 1;
                    }
                    i = j + 1;
                    continue;
                }
                out.push(w.to_string());
                out.push((*v).to_string());
                i += 2;
                continue;
            }
        }
        // Paired short flags: `-D warnings`, `-A dead_code`.
        if (w == "-D" || w == "-A") && i + 1 < words.len() {
            out.push(format!("{w} {}", words[i + 1]));
            i += 2;
            continue;
        }
        // Shell operators — skip.
        if matches!(w, "|" | "&&" | "||" | ">" | ">>" | "<" | "2>&1") {
            i += 1;
            continue;
        }
        out.push(w.to_string());
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_cargo_test_with_manifest_path_and_filter() {
        let t = significant_tokens(
            "cargo test --manifest-path tools/registry-consumer/Cargo.toml --all readme_",
        );
        assert!(t.contains(&"cargo".into()));
        assert!(t.contains(&"test".into()));
        assert!(t.contains(&"--manifest-path".into()));
        assert!(t.contains(&"tools/registry-consumer/Cargo.toml".into()));
        assert!(t.contains(&"--all".into()));
        assert!(t.contains(&"readme_".into()));
    }

    #[test]
    fn tokens_clippy_paired_flags() {
        let t = significant_tokens(
            "cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml -- -A dead_code -D warnings",
        );
        assert!(t.contains(&"-A dead_code".into()));
        assert!(t.contains(&"-D warnings".into()));
    }

    #[test]
    fn tokens_pnpm_filter() {
        let t = significant_tokens("pnpm --filter @opc/desktop exec tsc --noEmit");
        assert!(t.contains(&"pnpm".into()));
        assert!(t.contains(&"--filter".into()));
        assert!(t.contains(&"@opc/desktop".into()));
        assert!(t.contains(&"--noEmit".into()));
    }

    #[test]
    fn tokens_skip_shell_assignment() {
        // Shell var assignment with $() subshell — not a direct command.
        let t = significant_tokens("CARGO_VERSION=$(grep '^version' apps/desktop/src-tauri/Cargo.toml)");
        assert!(t.is_empty());
    }

    #[test]
    fn tokens_strip_matrix_expression_value() {
        let t = significant_tokens(
            "cargo check --target ${{ matrix.target }} --manifest-path apps/desktop/src-tauri/Cargo.toml",
        );
        assert!(t.contains(&"cargo".into()));
        assert!(t.contains(&"--manifest-path".into()));
        assert!(t.contains(&"apps/desktop/src-tauri/Cargo.toml".into()));
        // The matrix expression and its sigils MUST NOT leak into the output.
        for forbidden in ["${{", "matrix.target", "}}"] {
            assert!(
                !t.iter().any(|tok| tok == forbidden),
                "found forbidden token {forbidden:?} in {t:?}",
            );
        }
    }

    #[test]
    fn tokens_strip_matrix_expression_standalone() {
        let t = significant_tokens(
            "cargo build --release --target ${{ matrix.target }} --manifest-path crates/axiomregent/Cargo.toml",
        );
        for forbidden in ["${{", "matrix.target", "}}"] {
            assert!(!t.iter().any(|tok| tok == forbidden));
        }
        assert!(t.contains(&"crates/axiomregent/Cargo.toml".into()));
    }

    #[test]
    fn tokens_tool_binary_invocation() {
        let t = significant_tokens("./tools/spec-compiler/target/release/spec-compiler compile");
        assert!(t.contains(&"./tools/spec-compiler/target/release/spec-compiler".into()));
        assert!(t.contains(&"compile".into()));
    }

    #[test]
    fn normalise_strips_trailing_inline_comment() {
        assert_eq!(normalise("cargo test # a comment"), "cargo test");
    }

    #[test]
    fn normalise_strips_or_true_suffix() {
        assert_eq!(normalise("./tools/spec-lint/target/release/spec-lint || true"),
                   "./tools/spec-lint/target/release/spec-lint");
    }

    #[test]
    fn allow_list_matches_substring() {
        assert!(allow_list_suppresses(
            "touch apps/desktop/src-tauri/binaries/axiomregent-aarch64-apple-darwin"
        ));
        assert!(!allow_list_suppresses("cargo test --manifest-path crates/agent/Cargo.toml"));
    }

    #[test]
    fn consumer_rules_cover_governed_reads() {
        // Every governed-read consumer we care about MUST resolve to some
        // artifact under build/. If a new consumer is added without a rule,
        // this is the test that should fail.
        let lines = [
            "./tools/codebase-indexer/target/release/codebase-indexer check",
            "./tools/codebase-indexer/target/release/codebase-indexer render",
            "./tools/registry-consumer/target/release/registry-consumer list",
            "./tools/registry-consumer/target/release/registry-consumer status-report --json",
        ];
        for line in lines {
            assert!(
                CONSUMERS.iter().any(|c| line.contains(c.pattern)),
                "no consumer rule matched line: {line}"
            );
        }
    }

    #[test]
    fn producer_rules_cover_governed_writes() {
        let lines = [
            "./tools/spec-compiler/target/release/spec-compiler compile",
            "./tools/codebase-indexer/target/release/codebase-indexer compile",
            "./tools/adapter-scopes-compiler/target/release/adapter-scopes-compiler",
        ];
        for line in lines {
            assert!(
                PRODUCERS.iter().any(|p| line.contains(p.pattern)),
                "no producer rule matched line: {line}"
            );
        }
    }
}
