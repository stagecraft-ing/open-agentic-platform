---
source: research
source_path: ~/Dev2/stagecraft-ing/research
status: extracted
---

## Summary

This project is an empty Zola static site blog skeleton for "Asterisk Research" (research.asterisk.so), branded as an "AI+Security Research Blog." It uses a customized fork of the Duckquill theme (MIT licensed, by David Lapshin). The site contains zero actual blog posts -- only the theme scaffolding, templates, SCSS styling, i18n files, and a landing page with an ASCII-art Asterisk logo. There is no original research content, no custom code, no agent definitions, no build pipelines, and no OAP-relevant logic.

## Extractions

No items meet the threshold for extraction into OAP. Full analysis by category follows.

### 1. Directly portable code

None. The project contains only third-party Zola theme code (Duckquill v4.1.0) and standard static-site boilerplate. No Rust, TypeScript, or application code exists.

### 2. Architecture patterns

None. Standard Zola SSG layout (templates/content/sass/static/public). No novel patterns.

### 3. Agent/skill definitions

None present.

### 4. MCP/tool integrations

None present.

### 5. UI components/features

None relevant. The Duckquill theme includes client-side search (elasticlunr), copy-code-button, dark/light syntax themes, CRT shortcode, alert shortcodes, and i18n support, but these are all generic third-party blog theme features with no applicability to the OAP Tauri desktop app.

### 6. Spec/governance ideas

None. The blog was intended to publish AI+Security research but contains no actual content.

### 7. Build/CI/packaging

None. No CI config, Dockerfile, or build scripts exist -- only Zola's built-in `zola build` is implied.

### 8. Ideas only

None worth capturing. The concept of a research blog for the project could be revisited if OAP needs a public-facing research/changelog site, but that is already a well-understood pattern and this skeleton adds no intellectual value beyond what `zola init` provides.

## No-value items

| Item | Reason skipped |
|---|---|
| Duckquill theme (templates/, sass/, static/) | Third-party MIT theme, available upstream at codeberg.org/daudix/duckquill. No customization beyond branding colors and nav links. |
| content/_index.md, content/posts/_index.md | Empty blog scaffolding with ASCII logo; no research content |
| config.toml | Standard Zola config with Asterisk branding; no reusable config patterns |
| public/ (built output) | Generated HTML/CSS/JS from the empty site; no value |
| i18n/ (en.toml, ar.toml, ru.toml) | Stock Duckquill translation strings |
| LICENSE | MIT license for Duckquill theme, not OAP-relevant |
| Brand assets (logo.png, card.png, favicon.png, gradient-*.png) | Asterisk-specific branding, not OAP assets |
| elasticlunr.min.js | Third-party search library, available via npm |
| screenshot.png | Theme screenshot |

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
- [x] Zero blog posts exist; no research content to preserve
- [x] Theme is third-party (Duckquill) and available upstream
- [x] No custom code, scripts, or configurations of value
