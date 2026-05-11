---
source: claude-code
source_path: ~/Dev2/stagecraft-ing/claude-code
status: extracted
---

## Summary

This is the official Anthropic Claude Code community repository containing the CHANGELOG (documenting 2.1.x feature evolution), 13 official plugins demonstrating the Claude Code plugin system (agents, skills, commands, hooks), GitHub issue management automation (triage, dedup, lifecycle), devcontainer configuration with network sandboxing, enterprise settings examples, and the `.claude-plugin/marketplace.json` manifest. It is a reference implementation for Claude Code's extensibility surface.

## Extractions

### [Agent/Skill Definitions]: PR Review Toolkit Agents
- **What**: Six specialized review agents (code-reviewer, silent-failure-hunter, code-simplifier, comment-analyzer, pr-test-analyzer, type-design-analyzer) each with detailed system prompts, model specification (sonnet), and focused review mandates. The `/review-pr` command coordinates them with optional aspect selection.
- **Where in source**: `plugins/pr-review-toolkit/agents/*.md`, `plugins/pr-review-toolkit/commands/review-pr.md`
- **Integration target in OAP**: `packages/agents/review/` -- adapt as spec-aware review agents that check conformance alongside code quality
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Feature Development Workflow
- **What**: A structured 7-phase feature dev workflow with three agents (code-explorer, code-architect, code-reviewer) and a slash command that orchestrates codebase analysis, architecture design, implementation, and quality review.
- **Where in source**: `plugins/feature-dev/agents/*.md`, `plugins/feature-dev/commands/feature-dev.md`
- **Integration target in OAP**: `packages/agents/feature-dev/` -- could become spec-driven feature lifecycle agents
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Plugin Development Toolkit
- **What**: Comprehensive plugin development kit with 7 skills (hook-development, mcp-integration, command-development, agent-development, skill-development, plugin-structure, plugin-settings), 3 agents (agent-creator, plugin-validator, skill-reviewer), and detailed reference docs for each. Covers frontmatter specs, testing strategies, MCP server types (stdio/SSE/HTTP), and validation scripts.
- **Where in source**: `plugins/plugin-dev/` (entire tree)
- **Integration target in OAP**: Reference material for OAP's own plugin/extension system design
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Hookify -- Declarative Hook Rule Engine
- **What**: A Python-based hook rule engine that intercepts Claude Code events (PreToolUse, PostToolUse, UserPromptSubmit, Stop) using markdown-defined rules with YAML frontmatter. Rules specify matchers (tool names, patterns), conditions (field contains/matches), and actions (block/warn/modify). Includes `config_loader.py`, `rule_engine.py`, and event-specific handlers.
- **Where in source**: `plugins/hookify/` (core/, hooks/, matchers/, examples/)
- **Integration target in OAP**: `packages/governance/hooks/` -- adapt as a spec-driven governance hook system for OAP's conformance layer
- **Action**: outline-spec
- **Priority**: P1

### [MCP/Tool Integrations]: MCP Integration Reference
- **What**: Detailed reference docs for integrating MCP servers via stdio, SSE, and HTTP transports. Covers authentication patterns (OAuth, API key, headers), server configuration JSON schemas, and connection lifecycle. Includes examples for Asana, GitHub, and custom servers.
- **Where in source**: `plugins/plugin-dev/skills/mcp-integration/`
- **Integration target in OAP**: `docs/reference/mcp-integration.md` -- reference for OAP's MCP server integrations (gitctx-mcp and future servers)
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: DevContainer with Network Sandboxing
- **What**: Complete devcontainer setup with Dockerfile (Node 20, zsh, git-delta, Claude Code pre-installed) and `init-firewall.sh` that creates an iptables/ipset firewall allowing only GitHub, npm, Anthropic API, Sentry, and VS Code Marketplace traffic. Uses DNS resolution and IP aggregation for GitHub ranges.
- **Where in source**: `.devcontainer/` (Dockerfile, devcontainer.json, init-firewall.sh)
- **Integration target in OAP**: `.devcontainer/` -- adapt for OAP's own sandboxed development environment
- **Action**: integrate-now
- **Priority**: P1

### [Build/CI/Packaging]: GitHub Issue Automation Suite
- **What**: Complete issue lifecycle management: Claude-powered triage (`triage-issue.md` command), duplicate detection (`dedupe.md` command with parallel search agents), auto-close duplicates (TypeScript), lifecycle comments, sweep stale issues, lock closed issues. Includes 11 GitHub Actions workflows.
- **Where in source**: `scripts/` (auto-close-duplicates.ts, sweep.ts, lifecycle-comment.ts, issue-lifecycle.ts), `.github/workflows/`, `.claude/commands/`
- **Integration target in OAP**: `.github/workflows/` -- adapt triage and lifecycle patterns for OAP's issue management
- **Action**: capture-as-idea
- **Priority**: P2

### [Spec/Governance]: Enterprise Settings Examples
- **What**: Three example settings configurations (lax, strict, bash-sandbox) demonstrating Claude Code's managed settings hierarchy: disabling bypass permissions, blocking plugin marketplaces, requiring managed hooks/permission rules only, sandboxing bash execution with network restrictions.
- **Where in source**: `examples/settings/` (settings-lax.json, settings-strict.json, settings-bash-sandbox.json)
- **Integration target in OAP**: `docs/governance/` -- reference for OAP's own governance settings hierarchy
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Ralph Wiggum -- Self-Referential Iteration Loop
- **What**: A plugin that enables Claude to work on the same task repeatedly, seeing its previous work, until completion. Uses a stop hook to intercept exit attempts and continue iteration, with commands to start/cancel the loop. Novel pattern for autonomous iterative refinement.
- **Where in source**: `plugins/ralph-wiggum/`
- **Integration target in OAP**: `packages/agents/iteration/` -- pattern for autonomous spec refinement loops
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Security Guidance Hook
- **What**: Python-based PreToolUse hook monitoring 9 security patterns (command injection, XSS, eval usage, dangerous HTML, pickle deserialization, os.system calls) when editing files. Returns warnings to Claude as context.
- **Where in source**: `plugins/security-guidance/hooks/security_reminder_hook.py`
- **Integration target in OAP**: `packages/governance/security/` -- adapt as security conformance check in OAP's hook system
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Patterns]: Bash Command Validator Hook
- **What**: Python hook that intercepts Bash tool calls and validates commands against regex rules (e.g., suggesting `rg` over `grep`, `rg --files` over `find -name`). Clean pattern for tool-use governance.
- **Where in source**: `examples/hooks/bash_command_validator_example.py`
- **Integration target in OAP**: `packages/governance/hooks/` -- exemplar for OAP's tool-use governance hooks
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Marketplace Manifest Schema
- **What**: `.claude-plugin/marketplace.json` defining a plugin bundle with categorized plugins (development, productivity, learning, security), each with name, description, version, author, source path, and category.
- **Where in source**: `.claude-plugin/marketplace.json`
- **Integration target in OAP**: `packages/registry/` -- reference for OAP's own extension registry schema
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: CHANGELOG as Feature Intelligence
- **What**: The CHANGELOG documents ~90 releases of Claude Code 2.1.x, providing a detailed record of every feature, fix, and behavioral change. Contains intelligence on: streaming patterns, MCP protocol evolution, permission systems, hook lifecycle, plugin architecture, model routing, cowork dispatch, bare mode, and more.
- **Where in source**: `CHANGELOG.md` (180KB)
- **Integration target in OAP**: Reference material for understanding Claude Code SDK capabilities and limitations
- **Action**: capture-as-idea
- **Priority**: P2

### [Directly Portable Code]: PowerShell DevContainer Runner
- **What**: Cross-platform PowerShell script supporting Docker and Podman backends for spinning up the devcontainer, finding the container ID, and exec-ing into it with Claude pre-launched.
- **Where in source**: `Script/run_devcontainer_claude_code.ps1`
- **Integration target in OAP**: `scripts/` -- useful for Windows dev onboarding
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

- `LICENSE.md` -- Anthropic proprietary license, not applicable to OAP (MIT)
- `.gitattributes` -- simple line ending config, already handled in OAP
- `.gitignore` -- standard ignores, OAP has its own
- `.DS_Store` -- macOS artifact
- `.git/` -- repository metadata
- `demo.gif` -- marketing asset, not present in repo clone
- `plugins/explanatory-output-style/` -- session-start hook injecting educational context; niche style preference, low OAP value
- `plugins/learning-output-style/` -- similar session-start hook for learning mode; niche
- `plugins/claude-opus-4-5-migration/` -- model migration skill specific to Anthropic version transitions; ephemeral
- `plugins/frontend-design/` -- frontend design skill with aesthetic guidance; too opinionated for OAP
- `plugins/agent-sdk-dev/` -- Agent SDK verification agents (Python/TS); OAP already has its own SDK integration
- `plugins/commit-commands/` -- simple git workflow commands (commit, push, PR); OAP has its own commit patterns
- `plugins/code-review/` -- simplified version of pr-review-toolkit; redundant with the more comprehensive toolkit
- `.github/ISSUE_TEMPLATE/` -- bug report, feature request, model behavior, documentation templates; OAP has its own

## Safe-to-delete confirmation
- [x] All valuable content extracted or documented above
