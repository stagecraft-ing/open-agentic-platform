<div align="center">
  <h1>gitctx</h1>
  <p><strong>High-signal GitHub context for AI coding tools via MCP</strong></p>
</div>

`gitctx` is an MCP server that gives agents and IDE assistants targeted, up-to-date access to GitHub repositories. Code, Issues, PRs, Commits, Releases, everything. This is an open-source alternative to Amp's [Librarian](https://ampcode.com/news/librarian) and was inspired by the same.

![Screenshot 2026-02-14 at 12 48 59 AM](https://github.com/user-attachments/assets/e2a3ccca-3e9c-43e8-8b03-2f44ce69155b)

## Why gitctx

- Focused GitHub exploration primitives instead of generic web scraping
- Stateful repository context across tool calls (repo, branch, path)
- Good defaults for iterative code understanding workflows
- Works with any MCP client that supports stdio transport

## Feature Summary

- Repository discovery and selection (`find_repo`)
- Codebase navigation: tree/list/read/search/switch branch
- Issue and PR exploration with filters and metadata
- Commit, blame, and release analysis
- Repository statistics and dependency graph lookups
- MCP resource exposing current selected context
- In-memory API caching to reduce repeated calls

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Coding Agents Setup](#coding-agents-setup)
- [MCP Client Configuration](#mcp-client-configuration)
- [Tool Catalog](#tool-catalog)
- [Resources](#resources)
- [Authentication and Permissions](#authentication-and-permissions)
- [Operational Notes](#operational-notes)
- [Use Cases](#use-cases)
- [FAQ / Troubleshooting](#faq--troubleshooting)
- [Development](#development)
- [Project Structure](#project-structure)
- [Inspiration](#inspiration)
- [License](#license)

## Installation

### From source

```bash
git clone https://github.com/winfunc/gitctx.git
cd gitctx
cargo build --release
./target/release/gitctx-mcp
```

### Cargo

```bash
cargo install gitctx
gitctx-mcp
```

## Quick Start

1. Set a GitHub token (recommended):

```bash
export GITHUB_TOKEN=ghp_xxx
```

2. Start the MCP server:

```bash
gitctx-mcp
```

3. Configure your MCP client to spawn `gitctx-mcp` over stdio.

## Coding Agents Setup

### Claude Code

Add `gitctx` as a local stdio MCP server:

```bash
claude mcp add --transport stdio --env GITHUB_TOKEN=${GITHUB_TOKEN} gitctx -- gitctx-mcp
```

Useful follow-ups:

```bash
claude mcp list
/mcp
```

Notes:

- Claude docs support scoped installs (`local`, `project`, `user`) with `--scope`.
- For team sharing, prefer project-scoped MCP config.

### OpenAI Codex

Add via CLI:

```bash
codex mcp add gitctx --env GITHUB_TOKEN=${GITHUB_TOKEN} -- gitctx-mcp
```

Check status:

```bash
codex mcp --help
```

Alternative `config.toml` setup (`~/.codex/config.toml` or project `.codex/config.toml`):

```toml
[mcp_servers.gitctx]
command = "gitctx-mcp"
env_vars = ["GITHUB_TOKEN", "GH_TOKEN"]
```

### Cursor

Create either project config `.cursor/mcp.json` or global config `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "gitctx": {
      "type": "stdio",
      "command": "gitctx-mcp",
      "env": {
        "GITHUB_TOKEN": "${env:GITHUB_TOKEN}"
      }
    }
  }
}
```

Notes:

- Cursor docs describe `mcp.json`-based config for stdio/remote servers.
- Project config is best for repo-local sharing; global config is best for personal defaults.

### Amp

Add `gitctx` through Amp CLI:

```bash
amp mcp add gitctx -- gitctx-mcp
```

If you manage MCP servers in Amp config (`~/.config/amp/settings.json`), use the documented `amp.mcpServers` shape:

```json
{
  "amp.mcpServers": {
    "gitctx": {
      "command": "gitctx-mcp",
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    }
  }
}
```

If added in workspace settings, Amp may require explicit approval:

```bash
amp mcp approve gitctx
```

## MCP Client Configuration

If your client supports MCP over stdio but is not listed above, use this generic shape:

```json
{
  "mcpServers": {
    "gitctx": {
      "type": "stdio",
      "command": "gitctx-mcp",
      "env": {
        "GITHUB_TOKEN": "YOUR_GITHUB_TOKEN"
      }
    }
  }
}
```

Minimum requirements:

- stdio transport
- command `gitctx-mcp`
- environment variable forwarding for `GITHUB_TOKEN` (or `GH_TOKEN`)

## Tool Catalog

`gitctx` currently exposes 23 MCP tools.

| Category | Tools |
|---|---|
| Repository context | `find_repo`, `switch_branch`, `get_tree` |
| Code navigation | `list_dir`, `read_file`, `read_files`, `search_code` |
| Issues | `search_issues`, `get_issue`, `list_issue_comments` |
| Pull requests | `search_prs`, `get_pr`, `list_pr_comments` |
| Commits | `list_commits`, `get_commit`, `compare_commits`, `blame_file` |
| Releases | `list_releases`, `get_release`, `compare_releases` |
| Insights | `get_contributors`, `get_repo_stats`, `get_dependency_graph` |

### Typical workflow

1. Call `find_repo` first to select a repository.
2. Use `list_dir`, `get_tree`, and `read_file`/`read_files` to establish structure and context.
3. Use focused tools (`search_code`, `search_issues`, `search_prs`, `list_commits`) to answer specific questions.
4. Use details tools (`get_pr`, `get_commit`, `blame_file`, `get_release`) for deeper inspection.

## Resources

The server exposes one MCP resource:

- `gitctx://context/current`: current repository/session context (selected repo, branch, path, auth status)

## Authentication and Permissions

Token resolution order:

1. `GITHUB_TOKEN`
2. `GH_TOKEN`
3. `~/.config/gitctx/token.json`

Recommended token scopes for full functionality:

- `repo`
- `read:org`
- `read:user`

Without a token, public repositories still work, but rate limits are lower.

## Operational Notes

- Transport: stdio
- Logging: stderr (stdout is reserved for MCP protocol messages)
- Caching: in-memory TTL cache for repeated API requests
- Search behavior: code search is for code patterns/literals, not natural-language semantic search

## Use Cases

`gitctx` is designed for coding agents. You describe the task in natural language, and the agent decides which MCP tools to call.

### Prompt examples

#### Pull requests and issues

- "Find all open PRs related to authentication and summarize risk areas."
- "Show open issues labeled `bug` that mention rate limiting."
- "List merged PRs from the last 30 days that touched auth or session code."
- "Summarize unresolved review feedback on PR #123."

#### Codebase understanding

- "Map the authentication flow end-to-end with file references."
- "Find where OAuth callbacks are handled and explain error paths."
- "Show the main entry points and startup sequence for this repo."
- "Locate all places where this project talks to Redis."

#### Change and regression analysis

- "What changed between `v1.8.0` and `v1.9.0` that could affect login?"
- "Identify commits in the last 2 weeks that touched `src/auth`."
- "Blame the lines around this function and summarize recent ownership changes."
- "Compare `main` and `release/1.2` for API-breaking differences."

#### Dependency and release checks

- "List top dependencies and flag potentially high-risk transitive ones."
- "Show dependency changes introduced after the latest release."
- "Summarize release cadence and notable release-note themes."
- "Find where `jsonwebtoken` is used and how tokens are validated."

#### Security and reliability

- "Find hardcoded secrets, tokens, or suspicious credential patterns."
- "Identify endpoints missing authorization checks."
- "Search for weak cryptography patterns and summarize findings."
- "Locate TODO/FIXME comments related to security or reliability."

### Prompting tips

- Mention repository explicitly when possible (for example: `owner/repo`).
- Give scope boundaries (path, branch, tag, date range, PR number).
- Ask for structured output when needed (for example: summary + file references + risks).
- Prefer concrete intent: "find and summarize" works better than vague "investigate."

### Compound one-shot prompts

These are intentionally broad prompts where the coding agent should orchestrate many MCP tools behind the scenes and return one integrated answer.

- "In `owner/repo`, produce a release-readiness brief for authentication: analyze current auth code paths, open auth-related issues, merged PRs since the last release, commit churn in auth files, and release notes deltas. Return top risks, confidence level, and exact file/PR/issue references."
- "For `owner/repo`, investigate whether a recent login regression was introduced between `v1.9.0` and `main`: compare releases, inspect auth commits, review relevant PR discussions, correlate with open bug reports, and identify the most likely root-cause commits with rationale."
- "Create a security posture snapshot for `owner/repo`: find sensitive auth/session/token code, check recent security-related commits and PR comments, summarize unresolved high-priority issues, and map dependency risk from the graph. End with a prioritized remediation list."
- "In one pass, summarize what changed in data-access behavior over the last 60 days for `owner/repo`: code-level diffs, key PRs, linked issues, notable releases, and maintainers touching critical files. Provide a migration-impact score and supporting evidence."

## FAQ / Troubleshooting

### The MCP client starts, but no tools are available

- Confirm the command points to `gitctx-mcp`.
- Make sure the client uses stdio transport.
- Start manually in terminal to verify it boots:

```bash
gitctx-mcp
```

### I get errors saying no repository is selected

- This is expected until context is set.
- Call `find_repo` first, then run other tools.

### GitHub API requests are rate-limited

- Export a token before launching the MCP client:

```bash
export GITHUB_TOKEN=ghp_xxx
```

- For full access, include scopes: `repo`, `read:org`, `read:user`.

### Private repositories are not accessible

- Ensure token scopes include `repo`.
- Confirm the token belongs to an account with access to the target repo/org.
- Restart the MCP client after updating environment variables.

### `search_code` returns unexpected results

- `search_code` expects literal code patterns, not natural-language queries.
- Use specific identifiers, symbols, or strings (for example function/class names).

### Server appears silent when debugging

- Logs go to stderr by design.
- Run with explicit logging:

```bash
RUST_LOG=gitctx_mcp=debug,rmcp=warn gitctx-mcp
```

### `gitctx-mcp` command is not found

- If installed with Cargo, ensure Cargo bin path is in your shell PATH.
- Typical path:

```bash
export PATH=\"$HOME/.cargo/bin:$PATH\"
```

## Development

```bash
cargo check
cargo clippy
cargo test --lib --bins
```

Run server locally:

```bash
RUST_LOG=gitctx_mcp=debug,rmcp=warn gitctx-mcp
```

## Project Structure

```text
gitctx/
├── src/
│   ├── mcp/              # MCP server, tool router, resources
│   ├── github/           # GitHub API integrations
│   ├── auth/             # Token loading and validation
│   ├── cache.rs          # In-memory API cache
│   ├── context.rs        # Shared exploration context
│   ├── xml_format.rs     # Structured tool output formatting
│   └── mcp_main.rs       # MCP binary entrypoint
├── Cargo.toml
└── README.md
```

## Inspiration

`gitctx` is modeled after the workflow category popularized by Amp Code’s Librarian: a specialized agent capability for searching and understanding GitHub codebases quickly, including across repositories and dependencies.

Reference:

- Amp Chronicle: “The Librarian” (October 20, 2025): https://ampcode.com/news/librarian

## License

MIT License. See [LICENSE](LICENSE).
