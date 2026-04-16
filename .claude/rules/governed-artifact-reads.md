# Governed Artifact Reads

These rules apply to every orchestrated workflow in this project — commands under `.claude/commands/**`, agents under `.claude/agents/**`, and the init protocol in `AGENTS.md`. Interactive, exploratory tool use answering a user question is not bound by this file.

> Governed by spec **`103-init-protocol-governed-reads`**. Extends constitution Principle II (compiler-owned JSON machine truth) from authoring to reads.

## Principle

Compiled artifacts under `build/**` MUST be read by orchestrated workflows through their designated consumer binaries. Ad-hoc parsers over `build/**/*.json` in a workflow step are a workflow violation.

## Consumer Binaries

| Artifact | Consumer | Common subcommands |
|----------|----------|---------------------|
| `build/spec-registry/registry.json` | `registry-consumer` | `list`, `list --ids-only`, `list --json`, `show`, `status-report --json`, `compliance-report` |
| `build/codebase-index/index.json` | `codebase-indexer` | `compile`, `check`, `render` |
| `build/codebase-index/CODEBASE-INDEX.md` | read directly (already a governed human-shaped view) | — |

If a consumer subcommand is missing for a legitimate workflow query, add the subcommand under the consumer's spec — do not work around it with `python`, `jq`, `awk`, `sed`, or similar.

## Bad pattern

```bash
# Reaches past the consumer layer, guesses the shape, breaks on drift.
python3 -c "import json; d=json.load(open('build/codebase-index/index.json')); print(len(d['inventory']))"
```

## Good pattern

```bash
# Governed read. Typed at the tool boundary. Fails loudly on schema drift.
codebase-indexer check                                    # staleness gate
codebase-indexer render                                   # refresh markdown view
cat build/codebase-index/CODEBASE-INDEX.md                # human-shaped summary
registry-consumer status-report --json --nonzero-only     # typed lifecycle counts
```

## Exceptions

- A consumer binary IS allowed to parse its own artifact (`serde_json::from_reader`). That is what makes it the consumer.
- A human running `jq` at the shell to inspect an artifact interactively is not an orchestrated workflow. The rule binds repeatable protocol steps, not debugging.
- If a binary is unbuilt, workflows MUST instruct the user to `cargo build --release --manifest-path tools/<name>/Cargo.toml` — NOT silently fall back to ad-hoc parsing.

## Enforcement (MVP)

Enforcement is by review. A future spec may add an automated lint that rejects commands or agents which spawn `python`/`jq`/`awk`/`sed` against `build/**/*.json`.
