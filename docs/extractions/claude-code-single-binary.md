---
source: claude-code-single-binary
source_path: ~/Dev2/stagecraft-ing/claude-code-single-binary
status: extracted
---

## Summary

This project packages Anthropic's official `@anthropic-ai/claude-code` npm package (v1.0.41) into standalone single-binary executables using Bun's `--compile` feature. It contains build scripts that embed all assets (yoga.wasm, ripgrep binaries, VS Code extension) into the binary, patch minified code at build time to resolve platform-specific issues (Windows import.meta, POSIX shell requirements, file URL generation), and produce cross-platform executables for 15 target combinations (Linux glibc/musl, macOS Intel/ARM, Windows x64). The core `cli.js` is Anthropic's minified proprietary code; the original value is entirely in the build/packaging layer and the cross-platform workarounds.

## Extractions

### [Build/CI/Packaging]: Bun Single-Binary Compilation Pipeline

- **What**: A complete multi-target build system that compiles a Node.js CLI application into standalone executables using `bun build --compile`. Includes a target matrix of 15 platform/arch/libc combinations, automated asset embedding via Bun's `import ... with { type: "file" }` syntax, and a two-phase build (non-Windows then Windows with extra patches). The orchestrator script (`build-executables.js`) handles target selection, sequential builds, temp file cleanup, and error-tolerant per-target builds.
- **Where in source**: `scripts/build/build-executables.js`, `scripts/build/prepare-bundle-native.js`
- **Integration target in OAP**: `tools/` CLI tools (spec-compiler, spec-lint, registry-consumer) or any Rust/Node hybrid CLI that needs single-binary distribution. Could inform a future `scripts/build-single-binary.js` in OAP root, or be adapted for the desktop app's sidecar binaries.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: Bun Asset Embedding Pattern (WASM + Native Modules)

- **What**: Technique for embedding WASM files and platform-specific native `.node` modules into a Bun-compiled binary. Assets are declared as `import X from "./path" with { type: "file" }` at the top of the bundle, then referenced at runtime via a mapping object (`__embeddedFiles`). This eliminates runtime filesystem dependencies for tools like ripgrep.
- **Where in source**: `scripts/build/prepare-bundle-native.js` (lines building `embeddedImports` and `embeddedFilesMapping` arrays)
- **Integration target in OAP**: If OAP ever ships Node.js/Bun-based CLI tools as single binaries, this pattern is the reference implementation for embedding WASM grammars (tree-sitter) or native search modules.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Runtime Source Patching for Minified Code

- **What**: A systematic approach to patching minified/bundled JavaScript at build time using regex-based find-and-replace on specific code patterns. The scripts locate minified variable names and function calls (e.g., `var k81=await nUA(await VP9(...))`) and replace them with embedded-asset-aware equivalents. Includes fallback patterns when exact matches fail. This is fragile but effective for wrapping third-party minified code without access to source.
- **Where in source**: `scripts/build/prepare-bundle-native.js` (yoga.wasm patching, ripgrep patching, shell bypass patching), `scripts/build/prepare-windows-bundle.js` (import.meta patching)
- **Integration target in OAP**: Not directly applicable (OAP controls its own source), but the pattern of "patch third-party code at build time with fallback strategies" is useful knowledge if OAP ever needs to bundle/redistribute third-party CLI tools.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: Windows import.meta Compatibility Layer

- **What**: A complete solution for running ES module code in contexts where `import.meta` is unavailable (e.g., Bun's Windows CJS wrapper). Includes: (1) absolute path resolution from `process.argv[1]` with fallbacks, (2) a `__toFileURL()` helper that handles Windows drive letters (`file:///C:/...` vs `file:///path`), (3) systematic replacement of `import.meta.url` and `fileURLToPath(import.meta.url)` patterns, (4) `shell-quote` module override for cmd.exe-compatible quoting.
- **Where in source**: `scripts/build/prepare-windows-bundle.js` (`applyWindowsImportMetaFixes` function)
- **Integration target in OAP**: `apps/desktop` (Tauri) if it ever bundles JS-based sidecar processes for Windows, or any future Windows packaging of Node.js tools.
- **Action**: capture-as-idea
- **Priority**: P2

### [Architecture Patterns]: POSIX Shell Fallback / Bypass

- **What**: Pattern for making a CLI tool work on Windows without requiring a POSIX shell. Instead of throwing an error when no POSIX shell is found, the code falls back to `cmd.exe` on Windows or `/bin/sh` on Unix. Includes POSIX-to-Windows command translation: `/dev/null` to `NUL`, `source` to `REM`, `eval` to direct execution, `pwd -P` to `cd`.
- **Where in source**: `scripts/build/prepare-bundle-native.js` (shell bypass section), `scripts/build/prepare-windows-bundle.js` (POSIX-to-cmd.exe translations)
- **Integration target in OAP**: Any OAP tool that spawns shell commands cross-platform. Relevant to `crates/run` or `crates/agent` if they ever need to execute shell commands on Windows without assuming bash.
- **Action**: capture-as-idea
- **Priority**: P2

### [MCP/Tool Integrations]: Claude Code SDK TypeScript Interface

- **What**: The `sdk.mjs` and `sdk.d.ts` files define a clean TypeScript SDK for programmatically driving Claude Code as a subprocess. Key features: (1) async generator pattern (`query()` yields `SDKMessage` objects), (2) streaming JSON protocol over stdout, (3) MCP server configuration passthrough, (4) permission modes (default/acceptEdits/bypassPermissions/plan), (5) tool allow/disallow lists, (6) abort controller integration, (7) multi-turn conversation support (continue/resume). The message types (`SDKSystemMessage`, `SDKAssistantMessage`, `SDKResultMessage`) define a structured protocol for agent communication.
- **Where in source**: `sdk.mjs`, `sdk.d.ts`
- **Integration target in OAP**: `crates/agent` or a new `packages/claude-code-sdk` wrapper. The streaming JSON protocol and message types could inform OAP's own agent communication protocol. The permission mode concept maps to OAP's governance model.
- **Action**: outline-spec
- **Priority**: P1

### [Agent/Skill Definitions]: Claude Code SDK Message Protocol

- **What**: A well-defined message protocol for agent communication with four message types: `SDKSystemMessage` (init with tools, MCP servers, model, permission mode), `SDKUserMessage` (user input with parent tool use tracking), `SDKAssistantMessage` (model responses), and `SDKResultMessage` (success/error with usage stats including cost, duration, turns). The `SDKResultMessage` includes `total_cost_usd` and `NonNullableUsage` tracking, which are useful for governance/cost control.
- **Where in source**: `sdk.d.ts`
- **Integration target in OAP**: Spec for agent message protocol in `specs/`. The cost tracking and usage reporting pattern should inform OAP's agent observability/governance layer.
- **Action**: outline-spec
- **Priority**: P1

### [MCP/Tool Integrations]: MCP Server Configuration Types

- **What**: TypeScript types for three MCP server connection modes: `McpStdioServerConfig` (command + args + env), `McpSSEServerConfig` (url + headers), `McpHttpServerConfig` (url + headers). These are passed through to Claude Code via `--mcp-config` as JSON. This is a clean, minimal type definition for MCP server discovery/configuration.
- **Where in source**: `sdk.d.ts` (lines defining `McpServerConfig` union type)
- **Integration target in OAP**: Already partially covered by OAP's MCP integration, but the three-transport union type pattern is a good reference. Validate that OAP's MCP types cover stdio/SSE/HTTP equally.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: Cross-Platform Target Matrix

- **What**: A comprehensive 15-target distribution matrix covering: Linux x64 (glibc default/modern/baseline), Linux ARM64 (glibc), Linux x64 musl (default/modern/baseline), Linux ARM64 musl, macOS x64 (default/modern/baseline), macOS ARM64, Windows x64 (default/modern/baseline). Includes guidance on CPU feature detection (AVX2 for modern, SSE2 for baseline), libc detection, and universal binary creation via `lipo`.
- **Where in source**: `scripts/build/build-executables.js` (`PLATFORMS` object), `README.md`
- **Integration target in OAP**: `build/` or CI workflows. OAP's existing release workflows for gitctx-mcp could benefit from this target matrix pattern, especially the modern/baseline CPU tier distinction and musl variants for Alpine/Docker.
- **Action**: capture-as-idea
- **Priority**: P2

### [Build/CI/Packaging]: Code Signing and Security Considerations

- **What**: Documentation of macOS code signing (`codesign --force --deep --sign`), Windows security warning handling for unsigned executables, Windows Defender considerations for packed binaries, and supply chain security principles (verified sources, reproducible builds, no runtime downloads).
- **Where in source**: `README.md` (Code Signing section, Security Considerations section)
- **Integration target in OAP**: `apps/desktop` Tauri app signing workflow and any future binary distribution of CLI tools.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: Preinstall Platform Gate

- **What**: A `preinstall.js` hook that blocks `npm install` on Windows with a colored error message and links to documentation/WSL instructions. Simple but effective UX pattern for platform-gated packages.
- **Where in source**: `scripts/preinstall.js`
- **Integration target in OAP**: Any npm package in `packages/` that has platform restrictions.
- **Action**: capture-as-idea
- **Priority**: P2

### [Ideas Only]: Embedded Ripgrep for CLI Tools

- **What**: Pattern of bundling platform-specific ripgrep binaries (both the `rg` CLI and `ripgrep.node` native module) with platform detection at runtime to select the correct binary. Includes a safe platform detection helper with defensive fallbacks.
- **Where in source**: `scripts/build/prepare-bundle-native.js` (ripgrep embedding section), `vendor/ripgrep/` directory structure
- **Integration target in OAP**: `crates/xray` or `crates/gitctx` if they need fast file search. The `vendor/ripgrep/` directory layout (arch-platform subdirectories) is a clean pattern for multi-platform native binary distribution.
- **Action**: capture-as-idea
- **Priority**: P2

## No-value items

| Item | Reason |
|------|--------|
| `cli.js` (7.3MB minified) | Anthropic's proprietary minified code. Cannot be reused, only patched. No intellectual property value for OAP. |
| `Claude_Code_LICENSE.md` | Anthropic's commercial terms. Not applicable to OAP. |
| `Claude_Code_README.md` | Anthropic's official product README. Standard install/usage docs, no novel content. |
| `yoga.wasm` | Facebook's Yoga layout engine binary. Available from npm; no modification or novel usage here. |
| `vendor/ripgrep/*` | Standard ripgrep binaries from BurntSushi. Available from upstream releases. |
| `vendor/claude-code.vsix` | Anthropic's VS Code extension binary. Not usable by OAP. |
| `vendor/claude-code-jetbrains-plugin/*` | Anthropic's JetBrains plugin JARs. Not usable by OAP. |
| `scripts/test/*.bat`, `*.ps1` | Windows test scripts specific to the binary patching fixes. Testing patterns are standard; no reusable test infrastructure. |
| `scripts/fixes/*` | Windows shell wrappers and launchers. Specific to Claude Code Windows distribution workarounds; superseded by the POSIX bypass patch. |
| `scripts/debug/*` | Windows ARM64 diagnostic scripts. Specific to debugging Bun x64 emulation on ARM64 Windows. |
| `scripts/test/test-file-url-replacements.*` | Test harnesses for the file URL fix. Useful as documentation of the problem but not reusable code. |
| `scripts/test/test-windows-file-url-fix.cjs` | Same as above -- diagnostic, not reusable. |
| `.gitignore` | Standard Node.js gitignore. No novel entries. |
| `package.json` | Package metadata for `@anthropic-ai/claude-code`. The `optionalDependencies` on `@img/sharp-*` is notable but standard for image processing. |

## Safe-to-delete confirmation

- [x] All valuable content extracted or documented above
