---
source: asterisk-mcp-server
source_path: ~/Dev2/stagecraft-ing/asterisk-mcp-server
status: extracted
---

## Summary

Asterisk MCP Server is a Python-based Model Context Protocol (MCP) server that acts as a middleware between IDE/code-editor AI assistants and the Asterisk security vulnerability scanning API. It exposes three scanning tools (snippet scan, codebase scan, change verification) plus a settings tool over MCP using FastMCP, supports stdio and SSE transports, includes a Dear PyGui native settings UI, and ships as a `pipx`/`uvx`-installable package via PyPI. The codebase is small (~650 lines of Python across 5 source files), Apache-2.0 licensed, with no tests.

## Extractions

### [MCP/tool integrations]: Security scanning MCP tool pattern

- **What**: Three MCP tools (`scan_snippet`, `scan_codebase`, `verify`) that proxy code to an external security API and return markdown-formatted vulnerability reports. The pattern is: MCP tool receives code/file-paths from IDE agent, reads local files if needed, adds line numbers, POST to upstream API with API key auth, parses JSON response with `markdown_content` field, returns rich markdown to the agent. Includes structured error handling for connection errors, timeouts, HTTP 401, HTTP 429, and generic failures -- each returning user-friendly markdown error reports.
- **Where in source**: `asterisk_mcp_server/server.py` -- `_handle_scan_snippet`, `_handle_scan_codebase`, `_handle_verify`
- **Integration target in OAP**: Could inform a security-scan MCP server in `packages/mcp-servers/` or be referenced when building any API-proxying MCP tool pattern. The structured error-handling-to-markdown pattern is reusable for any MCP tool that calls external APIs.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: MCP middleware with hot-reloadable config

- **What**: `AsteriskMCPMiddleware` wraps `FastMCP` and calls `self.config.load()` (re-reads JSON from disk) before every API request via `_get_current_config()`. This allows settings changes (e.g., from the settings UI or manual file edit) to take effect without restarting the server. The `Config` class provides a clean load/save/update/get interface backed by a JSON file with sensible defaults.
- **Where in source**: `asterisk_mcp_server/server.py` lines 29-72 (`Config` class), lines 94-154 (`AsteriskMCPMiddleware.__init__`, `_get_current_config`)
- **Integration target in OAP**: The hot-reload config pattern is relevant to any MCP server in OAP that needs runtime-configurable settings without restart. Could be adapted for Rust-based MCP servers or the desktop app's MCP server management.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture patterns]: FastMCP tool registration via decorator + class method dispatch

- **What**: Tools are registered as inner async functions inside `_register_tools()` using the `@self.mcp.tool()` decorator, but each delegates to a private `_handle_*` method on the class. This keeps tool registration clean (docstrings serve as MCP tool descriptions visible to agents) while business logic lives in testable class methods.
- **Where in source**: `asterisk_mcp_server/server.py` -- `_register_tools` method
- **Integration target in OAP**: Reference pattern when building Python-based MCP servers for OAP. The docstring-as-tool-description convention is worth standardizing.
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/tool integrations]: In-chat settings UI trigger via MCP tool

- **What**: A `settings` MCP tool that triggers when a user types `/asterisk` in chat. It launches a native GUI (Dear PyGui) for configuration. The tool returns a success message after the GUI closes. This is an interesting pattern for MCP servers that need interactive configuration -- the agent can invoke a tool that opens a native window.
- **Where in source**: `asterisk_mcp_server/server.py` (`settings` tool in `_register_tools`), `asterisk_mcp_server/ui/settings.py`
- **Integration target in OAP**: OAP's desktop app (Tauri) already has a native UI; the pattern of an MCP tool triggering a configuration UI could be adapted for OAP's MCP servers to open Tauri settings panels. The slash-command-to-tool-invocation convention could be standardized.
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/skill definitions]: Security verification of accumulated chat changes

- **What**: The `verify` tool concept -- after a long coding session with an AI assistant, the user can ask to verify all accumulated code changes for security vulnerabilities. The tool accepts a list of `{file_path, code_snippet}` dicts representing changes made during the chat. This is a useful agent workflow pattern: accumulated-change-verification as a post-hoc quality gate.
- **Where in source**: `asterisk_mcp_server/server.py` -- `verify` tool and `_handle_verify`
- **Integration target in OAP**: This maps to OAP's conformance/governance layer. A "verify accumulated changes" tool pattern could be generalized beyond security to spec conformance, lint, or any post-session quality check. Could be a first-class concept in the spec spine.
- **Action**: outline-spec
- **Priority**: P1

### [Architecture patterns]: Markdown-formatted tool responses with structured sections

- **What**: All tool responses (including errors) return well-structured markdown with `# Title`, `## Section` headers, and `## Recommendations` blocks. Error responses are categorized (connection, timeout, auth, rate-limit, generic) with specific remediation guidance. This makes tool output immediately presentable in any markdown-capable UI.
- **Where in source**: Throughout `asterisk_mcp_server/server.py` -- all `_handle_*` methods
- **Integration target in OAP**: Establish as a convention for all OAP MCP tool responses. The error taxonomy (connection, timeout, 401, 429, generic) with markdown formatting could be a shared utility or trait.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/packaging]: Python MCP server packaging with hatchling + pipx

- **What**: Uses `hatchling` build backend, `pyproject.toml` with `[project.scripts]` entry point (`asterisk-mcp`), designed for `pipx run` / `uvx` invocation. The `scripts/publish.py` automates clean-build-upload to PyPI/TestPyPI using `build` + `twine`. The `uv.lock` provides reproducible dependency resolution.
- **Where in source**: `pyproject.toml`, `scripts/publish.py`, `uv.lock`
- **Integration target in OAP**: If OAP ships Python-based MCP servers, this packaging pattern (hatchling + pipx-runnable entry point) is the standard approach. The publish script pattern could be adapted.
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/governance ideas]: Post-session security gate as governance primitive

- **What**: The `verify` tool embodies a governance idea: after AI-assisted code changes, run a security verification pass before considering work complete. This is a "gate" concept that aligns with OAP's spec-governed delivery model. The gate could be mandatory (spec-enforced) or advisory.
- **Where in source**: Conceptual, derived from `verify` tool in `server.py`
- **Integration target in OAP**: Spec spine could define a `post-session-gate` or `change-verification-gate` concept where registered verifiers (security, conformance, lint) must pass before changes are marked as delivered. This would be a new spec feature.
- **Action**: outline-spec
- **Priority**: P1

### [Ideas only]: Line-number injection for codebase scanning

- **What**: The `add_line_numbers()` utility prepends formatted line numbers (`"  1 | code"`) to file contents before sending to the API. This helps the scanning API reference specific lines in its reports. Simple but useful for any tool that needs to reference code positions.
- **Where in source**: `asterisk_mcp_server/server.py` -- `add_line_numbers` function
- **Integration target in OAP**: Trivial utility, but the pattern of preprocessing code before sending to analysis APIs is worth noting. Could live in a shared utils module if multiple MCP tools need it.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

| Item | Reason |
|---|---|
| `LICENSE` (Apache 2.0 full text) | Standard license file, no unique content |
| `.gitignore` | Standard Python gitignore, nothing OAP-specific |
| `uv.lock` | Dependency lock file for this specific project's Python deps (httpx, mcp, dearpygui); not portable |
| `asterisk_mcp_server/__init__.py` | Single line version string |
| `asterisk_mcp_server/ui/__init__.py` | Single line docstring |
| `asterisk_mcp_server/ui/settings.py` (Dear PyGui UI code) | Uses Dear PyGui which is a niche Python GUI toolkit; OAP uses Tauri/web UI. The specific widget code is not portable. The *concept* of MCP-tool-triggered config UI is extracted above |
| `asterisk_mcp_server/cli.py` | Standard argparse CLI boilerplate; the pattern of CLI-args-override-config is common and not novel enough to extract |
| `scripts/publish.py` | Simple build+twine publish script; OAP uses GitHub Actions for releases |
| `.DS_Store` | macOS artifact |
| Asterisk API specifics (endpoints, payload formats) | Vendor-specific to asterisk.so; not portable |
| Duplicated error handling blocks | Each of the three `_handle_*` methods has nearly identical error handling (~80 lines each). This is a code smell, not a pattern to replicate. If adopted, this should be refactored into a shared error handler |

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
