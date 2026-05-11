---
source: gitctx
source_path: ~/Dev2/stagecraft-ing/gitctx
status: extracted
---

## Summary

gitctx is a Rust-based MCP server (23 tools) that provides GitHub repository exploration capabilities to AI coding agents over stdio transport. It wraps the GitHub REST and GraphQL APIs (via octocrab and reqwest) to expose repository navigation (find_repo, list_dir, read_file, read_files, search_code, get_tree, switch_branch), issue/PR exploration with filters, commit/blame/compare operations, release analysis, contributor/dependency-graph stats, and a stateful context resource tracking the current repo/branch/path. It features in-memory TTL+LRU caching, XML-formatted tool output optimized for LLM consumption, multi-source token resolution (GITHUB_TOKEN, GH_TOKEN, config file), and interactive PAT provisioning with browser open + scope validation. The full codebase (~3,500 lines of Rust across 19 source files) has already been imported verbatim into OAP at `crates/gitctx/`. The source and OAP copies are byte-identical (only Cargo.lock has minor dependency version bumps in OAP). OAP also already has CI workflows (`build-gitctx-mcp.yml`) and desktop app sidecar integration for this binary.

## Extractions

### [Directly portable code]: Entire gitctx crate (already ported)

- **What**: All 19 source files, Cargo.toml, README.md, LICENSE, .gitignore, and Cargo.lock have already been imported into OAP at `crates/gitctx/`. A byte-level diff of every `.rs` file and `Cargo.toml` shows zero differences between `~/Dev2/stagecraft-ing/gitctx/src/` and `~/Dev2/open-agentic-platform/crates/gitctx/src/`. The only difference is Cargo.lock dependency version bumps (OAP copy is slightly newer).
- **Where in source**: entire project
- **Integration target in OAP**: `crates/gitctx/` (already there)
- **Action**: integrate-now (already complete)
- **Priority**: P0

### [MCP/tool integrations]: 23-tool GitHub exploration MCP server

- **What**: A comprehensive MCP tool surface covering six categories: (1) Code navigation -- find_repo, list_dir, read_file, read_files, search_code, get_tree, switch_branch; (2) Issues -- search_issues, get_issue, list_issue_comments; (3) Pull requests -- search_prs, get_pr, list_pr_comments; (4) Commits -- list_commits, get_commit, compare_commits, blame_file; (5) Releases -- list_releases, get_release, compare_releases; (6) Insights -- get_contributors, get_repo_stats, get_dependency_graph. Each tool uses the `#[tool]` macro from rmcp with parameter structs deriving `schemars::JsonSchema` for automatic schema generation. All tools share a stateful `GitHubContext` and return XML-formatted output via `xml_format::to_xml()`.
- **Where in source**: `src/mcp/tools.rs` (1516 lines), `src/mcp/server.rs`, `src/mcp/mod.rs`
- **Integration target in OAP**: `crates/gitctx/` (already integrated)
- **Action**: integrate-now (already complete)
- **Priority**: P0

### [Architecture patterns]: Stateful MCP context with shared client

- **What**: `GitHubContext` uses `Arc<RwLock<ContextInner>>` to maintain shared state (current repo, branch, path, token, Octocrab client) across all tool invocations. The pattern of "select a target first, then operate on it" (call `find_repo` before any other tool) creates a session-like experience without actual session management. The Octocrab client is created once and shared via `Arc<Octocrab>` inside the context, avoiding per-call client creation overhead. Branch switches reset the working path. This is a clean pattern for any MCP server that needs cross-tool state.
- **Where in source**: `src/context.rs`
- **Integration target in OAP**: Already in `crates/gitctx/`. The pattern could be documented as a reference architecture for other stateful MCP servers in OAP.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: In-memory TTL+LRU cache for API responses

- **What**: Generic `ApiCache<T: Clone>` using `HashMap<String, CacheEntry<T>>` wrapped in `Arc<RwLock>` with 5-minute TTL, 1000-entry max, and LRU-style eviction (removes expired entries first, then earliest-expiration entries). Used as `Lazy<ApiCache<String>>` statics for directory listings and file contents. Cache keys are structured as `file:{owner}:{repo}:{branch}:{path}` and `dir:{owner}:{repo}:{branch}:{path}`. This is a self-contained, dependency-free cache implementation suitable for any MCP server making repeated API calls.
- **Where in source**: `src/cache.rs`
- **Integration target in OAP**: Already in `crates/gitctx/`. Could be extracted into a shared utility crate if other MCP servers need similar caching.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: XML output formatting for LLM consumption

- **What**: `xml_format::to_xml()` converts `serde_json::Value` trees to XML-tagged format optimized for LLMs. Handles objects, arrays (with automatic singularization of tag names: "files" -> "file", "entries" -> "entry", "matches" -> "match"), nulls (self-closing tags), and code content fields ("content", "patch", "diff", "diff_hunk", "body") which get newline-separated formatting. Includes XML escaping for non-code strings. This is a reusable utility for any MCP tool that wants structured output more parseable than JSON for LLMs.
- **Where in source**: `src/xml_format.rs`
- **Integration target in OAP**: Already in `crates/gitctx/`. Worth considering as a shared utility for other MCP servers that return structured data.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: Multi-source token resolution with interactive PAT flow

- **What**: Three-tier token resolution: (1) GITHUB_TOKEN env var, (2) GH_TOKEN env var (gh CLI compatibility), (3) `~/.config/gitctx/token.json` config file. Interactive PAT provisioning opens browser to GitHub's token creation page with pre-filled scopes, reads token from stdin, validates via GitHub `/user` API, checks `x-oauth-scopes` header for required scopes (`repo`, `read:org`, `read:user`), and saves to config file with 0600 Unix permissions. Includes `get_auth_status()` returning source and timestamp metadata.
- **Where in source**: `src/auth/github.rs`, `src/auth/mod.rs`
- **Integration target in OAP**: Already in `crates/gitctx/`. The interactive PAT flow pattern could be reused for other API token provisioning in the desktop app.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: Retry with exponential backoff

- **What**: Generic `retry_with_backoff` async function accepting any closure returning `anyhow::Result<T>`, with configurable max retries and delays starting at 100ms doubling each attempt (100ms, 200ms, 400ms...). Logs retries to stderr. Simple, dependency-free implementation useful for any external API call.
- **Where in source**: `src/github/mod.rs`
- **Integration target in OAP**: Already in `crates/gitctx/`. Could be extracted to a shared utility crate.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: GraphQL blame via reqwest (bypassing octocrab)

- **What**: `get_blame()` in `src/github/commits.rs` uses raw reqwest POST to GitHub's GraphQL API to fetch line-by-line blame information, since the REST API lacks a blame endpoint. The function fetches both blame ranges and file content in a single GraphQL query, then merges them into line-level blame data. This is a useful pattern for when octocrab doesn't cover a needed API -- fall back to raw HTTP with manual GraphQL.
- **Where in source**: `src/github/commits.rs` -- `get_blame()` function
- **Integration target in OAP**: Already in `crates/gitctx/`.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: Repo reference parsing (multiple URL formats)

- **What**: `parse_repo_reference()` handles `owner/repo`, `https://github.com/owner/repo`, `https://github.com/owner/repo.git`, and `git@github.com:owner/repo.git` formats, normalizing all to `(owner, name)` tuple. Well-tested with unit tests.
- **Where in source**: `src/github/search.rs`
- **Integration target in OAP**: Already in `crates/gitctx/`. Reusable utility for any tool that accepts GitHub repository references.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: Tree formatting with box-drawing characters

- **What**: `format_tree_output()` builds a hierarchical tree structure from flat path entries and renders it using Unicode box-drawing characters (like the Unix `tree` command), with directories sorted before files. Handles nested structures, base path filtering, and root naming.
- **Where in source**: `src/mcp/tools.rs` -- `format_tree_output()` function (bottom ~100 lines)
- **Integration target in OAP**: Already in `crates/gitctx/`.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/packaging]: Cross-platform CI build workflow

- **What**: OAP already has `.github/workflows/build-gitctx-mcp.yml` building for aarch64-apple-darwin, x86_64-apple-darwin, x86_64-unknown-linux-gnu, and x86_64-pc-windows-msvc. The source project at `stagecraft-ing/gitctx` had no CI -- OAP added this.
- **Where in source**: N/A (OAP addition)
- **Integration target in OAP**: `.github/workflows/build-gitctx-mcp.yml` (already there)
- **Action**: integrate-now (already complete)
- **Priority**: P0

### [Ideas only]: README prompt engineering examples

- **What**: The README contains extensive prompt engineering examples organized by use-case category (PR/issue analysis, codebase understanding, change/regression analysis, dependency/release checks, security/reliability, compound one-shot prompts). These are well-crafted agent workflow prompts that demonstrate how to compose multiple MCP tool calls. Could be repurposed as agent skill templates or documentation for OAP users.
- **Where in source**: `README.md` -- "Use Cases" section
- **Integration target in OAP**: Could feed into agent skill definitions, prompt library, or user documentation for the gitctx MCP integration in the desktop app.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas only]: Multi-agent-client setup documentation

- **What**: README has setup instructions for Claude Code, OpenAI Codex, Cursor, and Amp, covering both CLI and config-file approaches for each. This cross-client compatibility documentation pattern is valuable for any MCP server OAP ships.
- **Where in source**: `README.md` -- "Coding Agents Setup" section
- **Integration target in OAP**: Documentation patterns for MCP server setup across different clients.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas only]: SBOM-based dependency graph tool

- **What**: `get_dependency_graph` tool uses GitHub's SBOM (Software Bill of Materials) API endpoint to retrieve the full dependency graph with package URLs, scope (runtime/development), and relationship (direct/indirect) metadata. This is a useful capability for security and compliance workflows.
- **Where in source**: `src/github/stats.rs` -- `get_dependency_graph()`, `src/mcp/tools.rs` -- `get_dependency_graph` tool
- **Integration target in OAP**: Already integrated. The SBOM consumption pattern could inform future governance/compliance features in the spec spine.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- **Cargo.lock**: Only difference from OAP is slightly older dependency versions; OAP's is already newer. No value.
- **LICENSE**: MIT license, identical to OAP copy. No action needed.
- **.gitignore**: Standard Rust gitignore, identical to OAP copy. No action needed.
- **.DS_Store**: macOS artifact. No value.
- **Git history**: Only 3 commits (initial commit, init project, add screenshot). No development history worth preserving beyond the code itself.
- **Git remote** (`stagecraft-ing/gitctx`): Separate GitHub org/repo. The code has been fully imported to OAP already.

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
- [x] Source code is byte-identical to OAP's `crates/gitctx/` (verified via recursive diff)
- [x] OAP already has CI workflows, desktop app sidecar integration, and release workflows for this binary
- [x] No unique content in source that is absent from OAP -- the source is a strict subset
