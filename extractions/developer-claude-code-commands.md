---
source: developer-claude-code-commands
source_path: ~/Dev2/stagecraft-ing/developer-claude-code-commands
status: extracted
date: 2026-03-29
---

## Summary

A collection of 14 Claude Code slash commands (markdown files) designed for developer workflows, created by Nimbalyst. The commands cover planning, implementation with progress tracking, code review (with parallel sub-agent orchestration), branch review, commit message formatting, release workflows (internal + public), mockup generation, sub-agent creation, validation/fix pipelines, and utility commands (analyze-code, write-tests, mychanges). The project is pure prompt engineering -- no executable code, no tests, no build system. Value to OAP lies in the prompt patterns, agent orchestration strategies, and governance-adjacent review checklists that can be adapted to OAP's spec-governed workflow.

## Extractions

### Agent/Skill Definitions: Code Review Multi-Agent Orchestration

- **What**: `code-review.md` defines a sophisticated parallel sub-agent pattern: a coordinator performs impact assessment, then spawns up to 6 concurrent review agents (Architecture, Code Quality, Security, Performance, Testing, Documentation) via the `Task` tool. Includes pre-review impact analysis, post-review cross-pattern consolidation with alternative-hypothesis thinking, structured scoring (X/10 per aspect), and a consolidated report format with severity tiers (CRITICAL/HIGH/MEDIUM). The adaptive agent selection based on file types (docs-only changes skip performance review, etc.) is particularly well-designed.
- **Where in source**: `code-review.md`
- **Integration target in OAP**: `.claude/commands/code-review.md` or as a spec-governed agent in `crates/agent/` definitions. The structured output format aligns with OAP's conformance-lint philosophy -- review scores could feed into spec verification gates.
- **Action**: integrate-now
- **Priority**: P0

### Agent/Skill Definitions: Validate-and-Fix Pipeline

- **What**: `validate-and-fix.md` defines a systematic quality-gate pipeline: (1) discover available validation commands from project config, (2) run all checks in parallel, (3) categorize findings by severity (CRITICAL/HIGH/MEDIUM/LOW), (4) execute fixes in phased order (safe quick wins first, then functionality, then critical with user confirmation), (5) verify after each phase. Includes git-stash rollback capability, dependency mapping between issues, and partial-success handling. The pattern of "discover what validation tools exist, then run them" is project-agnostic.
- **Where in source**: `validate-and-fix.md`
- **Integration target in OAP**: `.claude/commands/validate-and-fix.md` directly, and the phased-fix-with-rollback pattern should inform `crates/agent/` governed execution (spec 035). The severity categorization maps naturally to OAP's safety-tier governance (spec 036).
- **Action**: integrate-now
- **Priority**: P0

### Agent/Skill Definitions: Branch Review Checklist

- **What**: `review-branch.md` provides an exhaustive read-only branch review with a structured checklist covering: database changes, security issues, performance concerns, cross-platform compatibility (file paths, keyboard shortcuts, env vars, line endings, permissions, path length limits), dependencies, logging, type safety, potential bugs, junk/cleanup, analytics events, and CLAUDE.md documentation needs. The cross-platform compatibility section is particularly thorough with 10 specific categories. Output is a structured table + detailed findings + file-by-file analysis.
- **Where in source**: `review-branch.md`
- **Integration target in OAP**: `.claude/commands/review-branch.md`. The structured checklist format could also generate spec-lint rules -- e.g., cross-platform checks could become conformance-lint violations.
- **Action**: integrate-now
- **Priority**: P1

### Architecture Patterns: Plan-to-Implementation Progress Tracking

- **What**: `implement.md` defines a workflow where a plan document (with YAML frontmatter including planId, status, priority, progress percentage, timestamps) is read, tasks are extracted from acceptance criteria, a checkbox list is inserted into the document, and progress is tracked by checking off items and updating frontmatter fields (status transitions: draft -> ready-for-development -> in-development -> in-review -> completed, with progress 0-100%). The plan stays synchronized with implementation.
- **Where in source**: `implement.md` + `plan.md`
- **Integration target in OAP**: This pattern maps to OAP's spec lifecycle. The frontmatter schema (planId, status, planType, priority, progress, stakeholders, tags) could inform a "work-item" layer above specs. The status state machine and progress tracking could enhance the desktop app's spec visualization.
- **Action**: outline-spec
- **Priority**: P1

### Architecture Patterns: Plan Document Schema

- **What**: `plan.md` defines a structured planning document format with YAML frontmatter (planId, title, status with defined values, planType enum, priority, owner, stakeholders, tags, created/updated timestamps, progress percentage, dueDate). Enforces separation between WHAT/WHY (plan) and HOW (implementation). Status values: draft, ready-for-development, in-development, in-review, completed, rejected, blocked. Plan types: feature, bug-fix, refactor, system-design, research.
- **Where in source**: `plan.md`
- **Integration target in OAP**: The status state machine and plan types could extend OAP's spec frontmatter. OAP specs currently use a different lifecycle but the concepts of progress tracking, stakeholders, and due dates could be added. The "plans are WHAT/WHY, not HOW" principle aligns with OAP's spec philosophy.
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Sub-Agent Creation Framework

- **What**: `create-subagent.md` provides a comprehensive framework for creating domain-expert agents with: YAML frontmatter schema (name, description, tools, model, category, color, displayName, bundle), delegation-first patterns (delegate to specialist or escalate to parent), environment detection steps, quality criteria ("Would I pay $5/month for this?", "Would someone put this on their resume?"), naming conventions (kebab-case, avoid verb-noun like "fix-circular-deps"), and a domain coverage assessment (5-15 related problems minimum, otherwise use a slash command). Includes example agent templates.
- **Where in source**: `create-subagent.md`
- **Integration target in OAP**: The agent schema and quality criteria could inform `crates/agent/` agent definitions. The delegation-first pattern and hierarchical expert model (broad expert -> sub-domain specialist) maps to OAP's governed execution model. The "5-15 problems" heuristic and resume test are useful design principles.
- **Action**: outline-spec
- **Priority**: P1

### Agent/Skill Definitions: Command Creation Meta-Command

- **What**: `create-command.md` is a meta-command for creating new Claude Code slash commands. Documents the full frontmatter schema (description, allowed-tools with granular security, argument-hint, model, category), features ($ARGUMENTS placeholders, bash execution with `!` prefix, file references with `@` prefix, namespacing with `:` for subdirectories), and includes templates for simple and complex commands.
- **Where in source**: `create-command.md`
- **Integration target in OAP**: `.claude/commands/create-command.md` directly usable. The security model (allowed-tools with glob patterns for bash) is relevant to OAP's safety-tier governance.
- **Action**: integrate-now
- **Priority**: P1

### Spec/Governance Ideas: Commit Message Governance

- **What**: `commit.md` enforces conventional commit prefixes (feat/fix/refactor/docs/test/chore), impact-focused titles (GOOD: "fix: OpenAI/LMStudio diffs now persist across app restarts" vs BAD: "feat: add pre-edit tagging for non-agentic AI providers"), 72-char line limit, issue linking patterns (Linear NIM-XXX, GitHub #XXX), and explicit prohibitions (no Co-Authored-By, no marketing taglines).
- **Where in source**: `commit.md`
- **Integration target in OAP**: `.claude/commands/commit.md` adapted with OAP-specific conventions. The impact-focused-over-implementation-focused title rule is a good standard. Could also become a git hook or conformance-lint rule.
- **Action**: integrate-now
- **Priority**: P1

### Build/CI/Packaging: Release Workflow (Internal + Public)

- **What**: `release-internal.md` defines a two-version release notes workflow: developer CHANGELOG (technical, all changes) + public release notes (user-facing only, marketing language, present tense). Includes auto mode for unattended releases. `release-public.md` defines a publish workflow that fetches the last public release via GitHub API, generates cumulative notes across versions, creates PUBLIC_RELEASE_NOTES.md, and triggers a GitHub Actions workflow via `gh workflow run`.
- **Where in source**: `release-internal.md`, `release-public.md`
- **Integration target in OAP**: The dual-notes pattern (developer changelog vs user-facing notes) is relevant for OAP's release workflows (spec 806ec33). The cumulative cross-version notes aggregation in release-public is useful. These are Nimbalyst-specific in detail but the patterns can be adapted.
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Patterns: Mockup Generation with Style Guide Caching

- **What**: `mockup.md` defines a workflow for creating HTML mockups: (1) detect new vs modification, (2) for new screens: auto-generate a style guide by crawling the codebase for CSS variables/themes/patterns and caching it, (3) for modifications: create pixel-perfect HTML replica of existing screen, cache it, then copy-and-modify. Includes annotation detection via screenshot capture, file naming conventions, and a sub-agent pattern for visual verification.
- **Where in source**: `mockup.md`
- **Integration target in OAP**: The style-guide-caching and existing-screen-replica patterns could inform OAP's desktop app development workflow. The mockup-as-HTML approach could be useful for OAP's Tauri UI development.
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Standup Summary Generator

- **What**: `mychanges.md` generates standup-style summaries from git history with time period parsing (1d, 2d, 1w), author filtering, grouping related commits into achievements, and conversational output format.
- **Where in source**: `mychanges.md`
- **Integration target in OAP**: `.claude/commands/mychanges.md` directly usable as-is. Low-effort, high-convenience utility.
- **Action**: integrate-now
- **Priority**: P2

### Agent/Skill Definitions: Analyze Code (Stub)

- **What**: `analyze-code.md` is a brief stub that describes code analysis (quality score, issues with severity, suggestions, best practices references) but provides no detailed prompt engineering -- it is more of a feature description than a working command.
- **Where in source**: `analyze-code.md`
- **Integration target in OAP**: Not worth integrating as-is. The code-review.md command covers this territory far more thoroughly.
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Write Tests (Stub)

- **What**: `write-tests.md` is a brief stub describing test generation (happy path, edge cases, errors, existing framework detection) but provides no detailed prompt engineering.
- **Where in source**: `write-tests.md`
- **Integration target in OAP**: Not worth integrating as-is. A proper test-generation command for OAP would need to understand the spec-test relationship.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

| Item | Reason |
|------|--------|
| `README.md` | Marketing/installation docs for Nimbalyst product. No technical value beyond what is extracted from individual commands. |
| `analyze-code.md` | Stub with no real prompt engineering; fully subsumed by code-review.md patterns. |
| `write-tests.md` | Stub with no real prompt engineering; too generic to add value without customization. |
| `.git/` directory | Repository history, not relevant. |
| Nimbalyst-specific references | Multiple commands reference `nimbalyst-local/plans/`, `nimbalyst-local/mockups/`, Nimbalyst MCP tools (`mcp__nimbalyst-mcp__capture_mockup_screenshot`), and Nimbalyst-specific paths. These need to be stripped/adapted on integration. |
| `release-internal.md` / `release-public.md` details | The specific script paths (`./scripts/release.sh`), repo URLs (`nimbalyst/nimbalyst-code`), and workflow names are Nimbalyst-specific. Only the patterns are valuable. |

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
- [x] Every file in the repository has been read and evaluated
- [x] Integration-worthy items identified with target locations in OAP
- [x] Nimbalyst-specific details noted for stripping during integration
