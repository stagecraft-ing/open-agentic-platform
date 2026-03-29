---
source: skills
source_path: ~/Dev2/stagecraft-ing/skills
status: extracted
---

## Summary

This project is a collection of 25 Claude Code/Claude.ai "skills" -- reusable prompt packages organized as SKILL.md files in category directories (dev/, product/, research/, content/, one-step-better/). Each skill is a markdown file with YAML frontmatter (name, description) and detailed instructions that Claude uses for specialized workflows. The skills cover development workflows (code review, commit messages, testing), product management (PRDs, triage, strategy memos), research (competitive analysis, deep multi-agent research), content creation (blogs, docs, slides, sales enablement), and a sophisticated workflow-improvement advisor. The project is MIT licensed under "Nimbalyst." There is no executable code -- only prompt engineering in markdown files, a README, a CONTRIBUTING guide, and a LICENSE.

## Extractions

### [Agent/Skill Definitions]: Skill Manifest Format (YAML frontmatter + markdown body)

- **What**: Every skill uses a consistent format: YAML frontmatter with `name` (kebab-case identifier) and `description` (activation trigger text), followed by a markdown body with instructions, templates, and best practices. The CONTRIBUTING.md documents this as a formal spec with quality checklist. This is a lightweight, portable skill definition format that could become OAP's standard for agent skill packaging.
- **Where in source**: `CONTRIBUTING.md`, every `SKILL.md` file
- **Integration target in OAP**: `specs/` -- define a skill-manifest spec (e.g., `S-SKILL-001`); `packages/` or `tools/` -- skill loader that reads this format
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Deep Researcher -- Parallel Multi-Agent Research Pattern

- **What**: A sophisticated multi-agent orchestration pattern: (1) classify query as breadth-first/depth-first/simple, (2) spawn 1-10 parallel subagents with mode indicators ("Deep dive:", "Quick check:", "Investigate:"), (3) each subagent writes to /tmp filesystem artifacts and returns only summary + path, (4) orchestrator synthesizes from files. Achieves 90% token reduction via filesystem artifact passing. Includes effort scaling rules and example execution patterns.
- **Where in source**: `skills/research/deep-researcher/SKILL.md`
- **Integration target in OAP**: `specs/` -- could inform a spec for multi-agent orchestration patterns; `tools/` or `packages/` -- reference implementation for agent-to-agent delegation with artifact passing
- **Action**: outline-spec
- **Priority**: P0

### [Agent/Skill Definitions]: Branch Reviewer -- Parallel Code Review with Sub-Agents

- **What**: A comprehensive code review skill that gathers git diff, then spawns 7 parallel sub-agents (security, performance, cross-platform, type safety/DRY, bugs/cleanup, analytics/docs, Jotai patterns). Produces structured report with quick-review checklist table, file-by-file analysis, and numbered action items users can select. Includes detailed checklists for cross-platform compatibility (path separators, keyboard shortcuts, env vars, line endings, etc.) and analytics event guidelines.
- **Where in source**: `skills/dev/branch-reviewer/SKILL.md`
- **Integration target in OAP**: `tools/` -- adapt as OAP's conformance-lint or pre-merge review tool; the parallel sub-agent pattern and cross-platform checklist are directly reusable
- **Action**: integrate-now
- **Priority**: P0

### [Agent/Skill Definitions]: Plan Implementer -- Plan-to-Code Execution with Progress Tracking

- **What**: Reads a plan document (with YAML frontmatter including status, progress fields), extracts acceptance criteria as checkboxes, updates frontmatter status through lifecycle (draft -> in-development -> in-review), calculates progress percentage, and keeps plan doc synchronized with implementation progress. Defines clear error handling for missing/completed/blocked plans.
- **Where in source**: `skills/dev/plan-implementer/SKILL.md`
- **Integration target in OAP**: `specs/` -- aligns with OAP's spec-driven development; the plan frontmatter lifecycle (status, progress, startDate, updated) could be adopted by spec-compiler for tracking spec implementation
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: One-Step-Better -- Personalized Workflow Advisor with Knowledge Base

- **What**: The most complex skill (549 lines). Two modes: (1) queue-based recommendation delivery from pre-scored queue, (2) deep analysis that profiles user (role, experience, goals), runs /insights for usage data, performs web searches across 25+ influencer sources for latest tips, analyzes project setup (CLAUDE.md, custom skills, MCPs, hooks, visual files), then generates 8-12 scored recommendations. Scoring formula: `impact * 10 + role_fit * 50 + ease * 5 + priority_boost + insights_bonus`. Includes a 50+ item knowledge base of Claude Code best practices with per-item metadata (priority, role relevance scores, experience level, impact/ease ratings, steps, success criteria, sources). Validates against user's existing setup to avoid redundant recommendations.
- **Where in source**: `skills/one-step-better/one-step-better-at-cc/SKILL.md`
- **Integration target in OAP**: `tools/` or `packages/` -- the recommendation engine pattern (profile -> analyze -> score -> queue -> deliver) could power an OAP onboarding/optimization advisor; the knowledge base format (ID, priority, role scores, impact/ease) is a reusable pattern for any scored-recommendation system
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Edge Case Analyzer

- **What**: Systematic edge-case identification framework covering input validation, data states (empty/extreme), system states (loading/offline/timeout/concurrent), permissions, user flow interruptions, integration failures, and accessibility. Includes comprehensive checklists and output template (scenario, frequency, impact, current/desired behavior, UX spec, engineering notes).
- **Where in source**: `skills/product/edge-case-analyzer/SKILL.md`
- **Integration target in OAP**: `specs/` -- the edge-case checklist categories could be formalized as a conformance-lint rule set; `tools/` -- integrate into spec review workflow
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: PRD Writer

- **What**: Structured PRD template with Problem Statement, Goals (user/business/metrics), Non-Goals, User Stories, prioritized Requirements (P0/P1/P2 with checkboxes), UX flows, Technical Considerations, Dependencies, Risks table, Open Questions, Timeline, and Appendix. Includes format variants for different contexts (new features, improvements, bug fixes, experiments).
- **Where in source**: `skills/product/prd-writer/SKILL.md`
- **Integration target in OAP**: `specs/` -- the PRD structure maps well to OAP's spec format; the P0/P1/P2 prioritization and risk table patterns could be adopted
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Request Triager with Priority Scoring

- **What**: Feature request triage workflow: summarize, deduplicate, categorize (type/area/segment/theme), prioritize using `Priority Score = (Impact x Frequency) / Effort`, and recommend action (Build Now / Roadmap / Parking Lot / Decline). Includes RICE scoring variant, deduplication report format, and decline response templates. Also includes Linear integration patterns for creating issues from triaged requests.
- **Where in source**: `skills/product/request-triager/SKILL.md`
- **Integration target in OAP**: `tools/` -- the prioritization formula and triage workflow could power an OAP feature-request processing pipeline
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Commit Helper -- Impact-Focused Commit Messages

- **What**: Enforces commit message discipline: type prefix (feat/fix/refactor/docs/test/chore), focus on impact/WHY not implementation, issue linking (Linear NIM-XXX, GitHub #XXX). Key insight: "Lead with the problem solved or capability added, not the technique used." Explicitly bans Co-Authored-By lines, marketing taglines.
- **Where in source**: `skills/dev/commit-helper/SKILL.md`
- **Integration target in OAP**: `.claude/` or `CLAUDE.md` -- adopt the commit message guidelines directly; could inform a pre-commit hook
- **Action**: integrate-now
- **Priority**: P1

### [Agent/Skill Definitions]: Pre-Commit Reviewer

- **What**: Reviews git diff to comment out (not delete) inappropriate logging, flags leftover TODOs/FIXME/HACK/TEMP markers, identifies dead code (commented-out blocks, unused imports, unreachable code). Also checks plan document frontmatter status if plans are being committed. Clear rules on what to keep (error/warning/analytics logging) vs. comment out.
- **Where in source**: `skills/dev/pre-commit-reviewer/SKILL.md`
- **Integration target in OAP**: `tools/` -- adapt as a conformance-lint pre-commit check
- **Action**: capture-as-idea
- **Priority**: P2

### [Agent/Skill Definitions]: Claude.md Refactorer

- **What**: Systematic approach to modularizing large CLAUDE.md files: identify extraction candidates (cross-cutting patterns, component-specific rules), create `docs/[NAME].md` files, create `.claude/rules/[name].md` with glob-based path-scoped rules and `@imports`, update main CLAUDE.md with brief references. Defines what to extract vs. keep, and reports size reduction metrics.
- **Where in source**: `skills/dev/claude-md-refactorer/SKILL.md`
- **Integration target in OAP**: `docs/` or `.claude/` -- directly applicable to managing OAP's own CLAUDE.md as the project grows; the glob-scoped rules pattern is reusable
- **Action**: integrate-now
- **Priority**: P1

### [Architecture Patterns]: Skill Category Taxonomy

- **What**: Five-category taxonomy for organizing AI skills: dev (code review, commits, testing, debugging), product (PRDs, strategy, triage, status), research (competitive, user research, deep dives), content (blogs, docs, slides, marketing), and meta (workflow optimization, skill creation). Each category has clear boundaries and naming conventions (kebab-case).
- **Where in source**: `README.md`, `CONTRIBUTING.md`, directory structure
- **Integration target in OAP**: `specs/` -- if OAP adopts a skill registry, this taxonomy provides a proven categorization model
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Filesystem Artifact Passing Between Agents

- **What**: Pattern where sub-agents write full reports to `/tmp/research_[timestamp]_[topic].md` and return only file path + 2-3 sentence summary to the orchestrator. Orchestrator then reads files selectively for synthesis. Claimed 90% token reduction vs. passing full content through conversation context.
- **Where in source**: `skills/research/deep-researcher/SKILL.md` (Phase 2-3)
- **Integration target in OAP**: `specs/` or `docs/` -- document as a recommended pattern for multi-agent workflows in OAP; relevant to MCP tool design
- **Action**: outline-spec
- **Priority**: P1

### [Architecture Patterns]: Mode-Indicator Trigger Phrases for Sub-Agent Depth Control

- **What**: Sub-agents receive depth instructions via trigger phrases at the start of their prompt: "Quick check:" (3-5 searches), "Investigate:" (5-10 searches), "Deep dive:" (10-15 searches). Simple mechanism for controlling agent effort/cost without complex configuration.
- **Where in source**: `skills/research/deep-researcher/SKILL.md` (Phase 2)
- **Integration target in OAP**: `docs/` -- document as a pattern for agent orchestration; could inform reasoning-effort or budget controls
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Parallel Sub-Agent Code Review

- **What**: Spawn N specialized sub-agents (security, performance, compatibility, etc.) in a single message for true parallelization, each analyzing the same diff from a different angle. Synthesize findings into unified report. The branch-reviewer spawns 7 concurrent analysis agents.
- **Where in source**: `skills/dev/branch-reviewer/SKILL.md`
- **Integration target in OAP**: `tools/` -- pattern for conformance-lint parallelization; each lint rule category could be a parallel agent
- **Action**: outline-spec
- **Priority**: P1

### [Spec/Governance Ideas]: Plan Document Lifecycle States

- **What**: Plan documents use YAML frontmatter with lifecycle states: draft -> ready-for-development -> in-development -> in-review -> completed (plus "blocked"). Frontmatter includes: status, progress (0-100), startDate, updated timestamp. The plan-implementer skill manages these transitions automatically as work progresses.
- **Where in source**: `skills/dev/plan-implementer/SKILL.md`
- **Integration target in OAP**: `specs/` -- directly maps to OAP's spec lifecycle; the spec-compiler could adopt these status fields and progress tracking
- **Action**: outline-spec
- **Priority**: P1

### [Spec/Governance Ideas]: Recommendation Scoring Formula

- **What**: Formula for scoring and prioritizing recommendations: `impact * 10 + role_fit * 50 + ease * 5 + priority_boost + insights_bonus`. Each recommendation has structured metadata: priority (P0-P2), role relevance scores (dev/design/pm/personal as 0.0-1.0), experience level (Beginner/Intermediate/Advanced), impact (1-10), ease (1-10). Validation rules prevent recommending things the user already does.
- **Where in source**: `skills/one-step-better/one-step-better-at-cc/SKILL.md`
- **Integration target in OAP**: `specs/` or `tools/` -- the scoring model could be adapted for spec prioritization or conformance issue severity ranking
- **Action**: capture-as-idea
- **Priority**: P2

### [UI Components/Features]: HTML Slide Deck Design System

- **What**: Complete HTML slide deck system with 10 templates (title, section divider, header+text, text+image, feature cards, screenshot gallery, two-column split, testimonials, questions, closing), design tokens (colors, gradients, typography in pt units, spacing), navigation system with prev/next links and slide counter, and workflow for parallel sub-agent slide generation. Fixed 720x405pt (16:9) dimensions, self-contained HTML with no external dependencies.
- **Where in source**: `skills/content/slide-deck-creator/SKILL.md`
- **Integration target in OAP**: Not directly applicable -- OAP is not a presentation tool. However, the design-token-driven template system is a useful pattern if OAP ever needs generated HTML reports or dashboards.
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/Tool Integrations]: HubSpot Integration Pattern

- **What**: Pattern for logging customer interview insights to HubSpot via MCP: create engagement notes on contacts with `mcp__Hubspot__hubspot-create-engagement` with type "NOTE", owner ID, contact associations, and metadata body.
- **Where in source**: `skills/research/customer-interview-sim/SKILL.md`
- **Integration target in OAP**: `tools/` -- example of MCP integration pattern; relevant if OAP builds CRM connectors
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/Tool Integrations]: Linear Integration for Issue Creation

- **What**: Pattern for creating Linear issues from triaged feature requests via MCP: uses `mcp__linear__create_issue` with title, team, priority mapping (High Impact=2, Medium=3, Low=4), labels, description, and project association. Also uses `mcp__linear__list_teams` for team discovery.
- **Where in source**: `skills/product/request-triager/SKILL.md`
- **Integration target in OAP**: `tools/` -- example MCP integration; relevant if OAP builds project-management connectors
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: Skill Installation Model

- **What**: Three distribution channels: (1) Claude Code via `claude skills add /path/to/skill`, (2) Claude.ai via uploading SKILL.md as project file, (3) Anthropic API via including SKILL.md content in system prompt. Single markdown file serves all three channels with no transformation needed.
- **Where in source**: `README.md`
- **Integration target in OAP**: `specs/` -- if OAP defines a skill format, this multi-channel distribution model (CLI install, web upload, API system prompt) is worth considering
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: Customer Interview Simulation

- **What**: Claude role-plays as a realistic customer persona (with defined industry, role, company size, current solution, pain point) for PM interview practice. Stays in character until explicitly stopped. Includes guidelines for realistic behavior (budget constraints, unclear articulation, emotional responses).
- **Where in source**: `skills/research/customer-interview-sim/SKILL.md`
- **Integration target in OAP**: Not applicable to OAP's core mission.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: Curated Influencer/Source List for AI Coding Tips

- **What**: The one-step-better skill maintains a curated list of 25+ sources/influencers for Claude Code tips, organized by role (PM vs. Dev). Includes agenticcoding.substack.com, ccforpms.com, YK Dojo, and named experts (Karpathy, Mollick, Rachitsky, etc.) with search query templates.
- **Where in source**: `skills/one-step-better/one-step-better-at-cc/SKILL.md` (section 2b-3)
- **Integration target in OAP**: `docs/` -- useful as a reference for staying current on AI coding practices
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

Items reviewed and intentionally skipped:

- **LICENSE** (MIT): Standard license file. OAP already has its own license. No extraction needed.
- **content/blog-writer/SKILL.md**: Blog writing style guide and SEO templates specific to Nimbalyst's marketing voice. Not applicable to OAP (a governance platform, not a CMS).
- **content/doc-writer/SKILL.md**: Generic documentation writing guidelines. Too generic to add value beyond what OAP already has in its own docs workflow.
- **content/launch-announcer/SKILL.md**: Product launch announcement templates (internal, external, release notes). Marketing-specific; not relevant to OAP's technical scope.
- **content/sales-enablement/SKILL.md**: Extensive sales collateral templates (battlecards, demo scripts, objection handling, one-pagers). Purely GTM-focused; no OAP relevance.
- **product/status-updater/SKILL.md**: Executive status update templates with emoji indicators. Generic PM reporting; OAP has its own governance reporting via specs.
- **product/github-status/SKILL.md**: GitHub activity summarizer for PMs. Generic; no novel patterns beyond standard `git log` / `gh` usage.
- **product/feature-explainer/SKILL.md**: Code-to-PM explanation framework. Generic audience-adaptation pattern; nothing OAP-specific.
- **product/feedback-analyzer/SKILL.md**: Customer feedback analysis templates (NPS, surveys, reviews). Generic PM workflow; not relevant to OAP.
- **product/work-tracker/SKILL.md**: Markdown-based task/bug/idea tracker with ULID-based IDs. Lightweight and functional but OAP already has spec-driven tracking.
- **product/strategy-memo/SKILL.md**: Strategy memo templates. Generic management communication; not relevant to OAP.
- **dev/standup-summary/SKILL.md**: Git commit summarizer for standups. Trivial `git log` wrapper; no novel patterns.
- **dev/code-analyzer/SKILL.md**: Minimal code analysis stub (only 20 lines of content). Too thin to extract.
- **dev/test-writer/SKILL.md**: Minimal test generation stub (only 18 lines of content). Too thin to extract.
- **dev/bug-reporter/SKILL.md**: Guided bug report creation through conversational Q&A. Well-designed but Nimbalyst-specific (references Nimbalyst UI modes, file tree, tab manager). The general approach is standard.
- **dev/lib-updater/SKILL.md**: Dependency update workflow for three specific Nimbalyst packages (claude-agent-sdk, MCP SDK, codex SDK). Too product-specific; the general pattern (check current -> fetch latest -> show changelog -> update -> verify) is standard.
- **research/competitive-analyst/SKILL.md**: Competitive analysis templates (SWOT, comparison matrices). Generic PM workflow; not relevant to OAP.
- **research/user-research-doc/SKILL.md**: User research document template. Minimal (40 lines), generic.

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
