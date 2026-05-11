---
source: product-manager-claude-code-commands
source_path: ~/Dev2/stagecraft-ing/product-manager-claude-code-commands
status: extracted
---

## Summary

A collection of 19 Claude Code custom slash commands (markdown prompt templates) designed for Product Managers, created by Nimbalyst. Each file is a structured prompt that defines an agent persona, output format, file naming conventions, and interaction flow for a specific PM workflow (PRDs, competitive analysis, bug reports, triage, research, mockups, etc.). The project contains zero executable code -- it is entirely markdown-based prompt engineering with some MCP integration references (HubSpot, Linear) and one GitHub Actions workflow for zipping markdown into a release artifact. Licensed MIT.

## Extractions

### Agent/Skill Definitions: Structured Slash-Command Prompt Pattern

- **What**: Every file follows a consistent pattern: role preamble, file location/naming conventions, structured templates with placeholders, usage examples, best practices, and "what to ask" clarification sections. This is a proven format for making Claude Code commands reliable and self-contained. OAP's `.claude/commands/` directory is currently empty -- this pattern could seed it.
- **Where in source**: All 17 `.md` files in the root directory (excluding README.md and LICENSE)
- **Integration target in OAP**: `.claude/commands/` directory or a command registry in the spec spine
- **Action**: capture-as-idea
- **Priority**: P1

### Agent/Skill Definitions: Research Command with Parallel Subagent Orchestration

- **What**: `research.md` implements a sophisticated multi-agent research pattern: query classification (breadth-first / depth-first / simple factual), parallel subagent spawning with depth-mode trigger phrases ("Deep dive:", "Quick check:", "Investigate:"), filesystem-based artifact passing (write to `/tmp/research_*.md`, return file path + summary to reduce token usage), and a synthesis phase that reads all artifacts and produces a consolidated report. This is the most architecturally interesting file in the repo. The pattern of writing intermediate results to filesystem to avoid token bloat is directly applicable to OAP agent orchestration.
- **Where in source**: `research.md`
- **Integration target in OAP**: Agent orchestration patterns in desktop app or as a reference architecture for multi-agent workflows in the spec
- **Action**: outline-spec
- **Priority**: P0

### Agent/Skill Definitions: Bug Report Interactive Investigation Flow

- **What**: `bug-report.md` defines a multi-turn conversational investigation pattern: read initial report, analyze codebase, identify ambiguities, ask one clarifying question at a time, then generate a structured bug report. Includes a "Human Sourced" section that preserves all original human text verbatim with AI/Human turn markers. This pattern of progressive clarification before action is useful for any OAP agent that needs to gather context before executing.
- **Where in source**: `bug-report.md`
- **Integration target in OAP**: Agent interaction patterns, possibly a `/bug-report` command for OAP development itself
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Customer Interview Simulator

- **What**: `customer-interview-simulate.md` flips the AI role -- instead of being an assistant, it role-plays as a customer with realistic pain points, budget constraints, and emotional responses. The PM practices interview skills against it. Post-interview, it generates a structured summary. This "AI-as-interviewee" pattern is novel and could be adapted for OAP user testing (simulate a developer using the governed platform to find UX friction).
- **Where in source**: `customer-interview-simulate.md`
- **Integration target in OAP**: Could be adapted as a conformance testing agent that role-plays as a developer trying to violate spec constraints
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Triage/Deduplication with Linear Integration

- **What**: `triage-requests.md` defines a structured triage workflow: summarize raw requests, deduplicate, categorize by type/area/segment/theme, score using Impact x Frequency / Effort, recommend action (Build Now / Roadmap / Parking Lot / Decline), and optionally create Linear issues via MCP. Includes decline response templates. The prioritization framework (RICE scoring, t-shirt sizing) and the deduplication pattern are directly useful for OAP issue management.
- **Where in source**: `triage-requests.md`
- **Integration target in OAP**: Could become a `/triage` command for OAP backlog management
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Edge Case Analysis Checklist

- **What**: `edge-cases.md` contains a comprehensive, category-organized checklist for edge case analysis: input validation, data states (empty/extreme), system states (loading/offline/timeout/concurrent edits), permissions, user flow interruptions, integration failures. Each category has specific items. This is directly usable as a conformance or QA checklist template for OAP features.
- **Where in source**: `edge-cases.md`
- **Integration target in OAP**: Spec compiler test generation or as a QA command
- **Action**: capture-as-idea
- **Priority**: P2

### Agent/Skill Definitions: Mockup Generation with Style Guide Extraction

- **What**: `mockup.md` defines a two-path workflow for UI mockups: (1) new screens -- first extract a style guide from the codebase (colors, typography, spacing, components), then build standalone HTML mockups matching it; (2) modifications -- create pixel-perfect HTML replicas of existing screens, copy to mockups dir, then apply changes. Uses sub-agents for code analysis and screenshot verification. The style-guide-extraction-from-codebase pattern is interesting for OAP's desktop app development.
- **Where in source**: `mockup.md`
- **Integration target in OAP**: Desktop app (Tauri) development workflow
- **Action**: capture-as-idea
- **Priority**: P2

### Architecture Patterns: Plan Document with YAML Frontmatter and Status Machine

- **What**: `plan.md` defines a plan document format with structured YAML frontmatter including: planId, title, status (draft/ready-for-development/in-development/in-review/completed/rejected/blocked), planType (feature/bug-fix/refactor/system-design/research), priority, owner, stakeholders, tags, timestamps, progress percentage, and dates. The explicit "WHAT and WHY, not HOW" principle and the status state machine are directly relevant to OAP's spec spine governance model.
- **Where in source**: `plan.md`
- **Integration target in OAP**: Spec spine document format or ADR template enrichment
- **Action**: capture-as-idea
- **Priority**: P1

### Architecture Patterns: Feedback Analysis Framework

- **What**: `feedback-analyze.md` defines a structured framework for processing customer feedback: categorize into Pain Points / Blockers / Feature Requests / Praise / Confusion / Bugs, score each by Frequency x Severity, extract representative quotes, and produce executive summaries. The framework of "one complaint is noise, ten is a signal" and segmented analysis (free vs paid, new vs returning) is useful methodology.
- **Where in source**: `feedback-analyze.md`
- **Integration target in OAP**: Could inform how OAP processes its own user feedback / GitHub issues
- **Action**: capture-as-idea
- **Priority**: P2

### MCP/Tool Integrations: HubSpot and Linear MCP References

- **What**: `customer-interview-simulate.md` references `mcp__Hubspot__hubspot-create-engagement` for logging interview notes as CRM engagements. `triage-requests.md` references `mcp__linear__create_issue` and `mcp__linear__list_teams` for creating triaged feature requests as Linear issues. These are concrete examples of MCP tool invocations from slash commands -- the pattern (not the specific tools) is relevant to OAP's MCP integration story.
- **Where in source**: `customer-interview-simulate.md`, `triage-requests.md`
- **Integration target in OAP**: MCP integration patterns documentation
- **Action**: capture-as-idea
- **Priority**: P2

### MCP/Tool Integrations: Screenshot Capture for Mockup Verification

- **What**: `mockup.md` references `mcp__nimbalyst-mcp__capture_mockup_screenshot` for capturing and verifying mockup screenshots in a feedback loop (capture, analyze, fix, re-capture). This verify-via-screenshot loop pattern could be useful for OAP desktop app UI testing.
- **Where in source**: `mockup.md`
- **Integration target in OAP**: Desktop app testing workflows
- **Action**: capture-as-idea
- **Priority**: P2

### Build/CI/Packaging: GitHub Actions Markdown Zip Release

- **What**: `.github/workflows/zip.yml` defines a manual-dispatch workflow that zips all tracked markdown files (`git ls-files -z -- '*.md' '*.MD'`) into a release artifact using `softprops/action-gh-release@v2`. Uses `git ls-files` to avoid including untracked files. Simple but the pattern of bundling markdown documentation as a release artifact is a minor reference.
- **Where in source**: `.github/workflows/zip.yml`
- **Integration target in OAP**: No direct integration needed
- **Action**: capture-as-idea
- **Priority**: P2

### Spec/Governance Ideas: PRD Template Structure

- **What**: `prd.md` defines a comprehensive PRD template: Problem Statement (Who/What/Why), Goals (User/Business/Success Metrics), Non-Goals, User Stories, Requirements prioritized as P0/P1/P2, UX flows, Technical Considerations, Dependencies, Risks & Mitigations table, Open Questions, Timeline, and Appendix. The P0/P1/P2 prioritization and the Risks & Mitigations table format could inform OAP spec document templates.
- **Where in source**: `prd.md`
- **Integration target in OAP**: Spec spine document templates
- **Action**: capture-as-idea
- **Priority**: P2

### Spec/Governance Ideas: Strategy Memo Template

- **What**: `strategy.md` defines a strategy memo format: TL;DR, Context (Current State / Market Forces / Opportunity), Recommendation, Rationale (Strategic Alignment / User Value / Business Impact / Competitive Positioning / Risks), Alternatives Considered with pros/cons/decision, Implementation Plan with phases, and Success Metrics. The "Alternatives Considered" section pattern is particularly useful for ADRs.
- **Where in source**: `strategy.md`
- **Integration target in OAP**: ADR template enrichment (alternatives-considered section)
- **Action**: capture-as-idea
- **Priority**: P2

### Ideas Only: Understand-Feature Codebase Explainer

- **What**: `understand-feature.md` defines a command that deep-inspects a codebase and explains features at different levels (PM, Designer, CS, Engineering). Uses a structured output format: Feature Overview, User Journey Trace with file:line references, Data Flow diagrams (ASCII), Dependencies map, and Failure Points. The audience-adaptive explanation style (same feature explained differently for PM vs Engineer) is a good UX pattern.
- **Where in source**: `understand-feature.md`
- **Integration target in OAP**: Could be a useful developer onboarding command for OAP itself
- **Action**: capture-as-idea
- **Priority**: P2

### Ideas Only: GitHub Status Summarizer

- **What**: `github-status.md` defines templates for summarizing GitHub activity at repo, team, feature, and sprint levels. Includes stale-work detection (issues with no activity in 30+ days, PRs awaiting review >5 days). The stale-work detection concept could be automated in OAP CI.
- **Where in source**: `github-status.md`
- **Integration target in OAP**: CI/automation for stale issue detection
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

| Item | Reason skipped |
|------|---------------|
| `README.md` | Project overview and Nimbalyst marketing copy; no technical content beyond what the individual commands contain |
| `LICENSE` | MIT license from Nimbalyst; not portable (OAP has its own licensing) |
| `.DS_Store` | macOS metadata artifact |
| `launch.md` | Launch announcement templates (internal/external/release notes); standard PM templates with no OAP-specific value |
| `sales-enablement.md` | Sales battlecard, objection handling, and marketing copy templates; extensive but entirely focused on B2B SaaS sales motions irrelevant to OAP |
| `documentation.md` | Generic documentation-writing command; very thin (mostly writing guidelines); OAP already has stronger doc conventions |
| `status.md` | Executive status update templates; standard format with no novel patterns |
| `competitive.md` | Competitive analysis templates (SWOT, comparison matrices, pricing research); standard PM toolkit, not technically interesting |
| `customer-interview.md` | Real customer interview prep command; standard interview guide format |
| `strategy.md` (content) | The template structure is noted above; the actual content is generic strategy memo guidance |
| `.github/workflows/zip.yml` (content) | Trivially simple workflow; noted above but no real integration value |

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
- [x] The research.md parallel-subagent pattern (P0) is fully documented
- [x] The plan.md frontmatter/status-machine pattern (P1) is fully documented
- [x] The slash-command prompt engineering pattern (P1) is fully documented
- [x] All MCP integration references captured
- [x] All 17 command files, README, LICENSE, and CI workflow reviewed
- [x] No executable code exists in this project (markdown only)
