# Spec-Spine Cut D — Architectural Review

**Branch under review:** `cut-d/autonomous-run-20260519-025506`
**Scope:** Architectural shape of the post-Cut-D crate set and its
patterns. Not verification, not risk-surface.
**Read at:** 2026-05-19, `cargo test --workspace` clean per Fix 1–5
disclosure (see `spec-spine-cut-d-verification.md` +
`spec-spine-cut-d-run-report-corrigendum.md`; cross-referenced as
claims, not as instructions).

## Verdict table

| Question | Sub | Verdict |
|---|---|---|
| Q1. Crate boundaries, naming, placement | Q1a — shared-types cohesion | sound-with-reservations |
| | Q1b — naming triples for the typed-reader crate | sound-with-reservations |
| | Q1c — featuregraph placement | sound-with-reservations |
| Q2. Patterns introduced by Cut D | Q2a — producer-also-hosts-reader | sound-with-reservations |
| | Q2b — enricher pattern | sound-with-reservations |
| | Q2c — schema-version dispatch | sound-with-reservations |
| Q3. Hidden semantic decisions | Q3a — KNOWN_KEYS allowlist-vs-emitted factoring | sound-with-reservations |
| | Q3b — parallel-pipelines design | sound |
| | Q3c — G-2 emitter-side placement | sound |
| Q4. Split-readiness | (classified list inline) | sound-with-reservations |
| Q5. External contract + grammar | (sub-verdicts inline) | sound-with-reservations |

**Final architectural verdict: SOUND-FOR-NOW.**

---

## Q1. Crate boundaries, naming, placement

### Q1a — shared-types cohesion

`tools/shared/spec-types/src/lib.rs` (read end-to-end, 264 lines)
combines three responsibilities under one Cargo identifier:

- **(a) Frontmatter parsing helpers** (lib.rs:18–56): `FrontmatterError`,
  `split_frontmatter_required`, `split_frontmatter_optional`. These were
  absorbed in W-01 from `tools/shared/frontmatter/` and have **no
  semantic dependency** on (b) or (c). They parse `---` blocks; nothing
  more.
- **(b) Spec-format vocabularies** (lib.rs:58–175): `KNOWN_KEYS`,
  `VALID_RISK_LEVELS`, `VALID_KINDS`, `SHAPE_TABLE`,
  `CONVENTIONAL_CATEGORIES`. The compiler-consumed grammar of the
  authored corpus.
- **(c) Diagnostic-code registries** (lib.rs:177–237): `Severity`,
  `ViolationCode`, `V_001..V_019`, `W_001..W_007`, `W_130..W_132`.

The leaf-discipline argument *is* preserved at the **dep level** — the
manifest at `tools/shared/spec-types/Cargo.toml` carries only `serde` +
`serde_yaml`. But responsibility-level cohesion is partial. (a) is a
file-format parser; (b) is a corpus grammar; (c) is a runtime
diagnostic vocabulary. A change to any one of (b)'s constants
(`KNOWN_KEYS` growing for spec 147 — lib.rs:95–112) does not require
touching (a) or (c), and vice-versa; the three concerns evolve on their
own cadence.

The doc comment at lib.rs:6–9 acknowledges this seam ("they have no
semantic dependency on the vocabularies but ship from the same leaf
crate so every spec-spine producer takes exactly one foundational
dep"). That is an honest rationale — single-dep simplicity — but it is
a packaging convenience, not a cohesion claim.

A hypothetical third-party tool that wants only to *read*
registry.json: post-Cut-D it depends on
`open_agentic_spec_registry_reader` (which depends only on serde +
serde_json — registry-consumer/Cargo.toml:17–21) and **does not pull
shared-types at all**. So the "what does shared-types pull in for a
read-only consumer" question is moot today: no read-only consumer
imports it. The crates that do depend on shared-types (spec-compiler,
spec-lint, codebase-indexer, oap-registry-enrich, oap-code-index-enrich,
policy-compiler — confirmed via `grep -rn "open_agentic_spec_types"`)
are all producers/linters/enrichers, where exposure to all three
concerns is acceptable.

In a post-split world where (b)'s vocabularies are the spec format and
(a)'s parsing helpers are a markdown convention, the two would
plausibly live in distinct crates owned by distinct teams. The current
single-crate shape would not block a split but would surface a
sub-decomposition question at split planning time.

**Verdict: sound-with-reservations.** The crate is internally coherent
as "spec-spine shared low-level surface" but mixes three responsibilities
that have no internal cross-references. The single-dep packaging
benefit is real today; the cohesion seam is visible.

### Q1b — naming triples for the typed-reader crate

Directory: `tools/registry-consumer/` (unchanged from pre-Cut-D —
preserved per W-04 commit body, `756cd87b`).
Cargo `name`: `open_agentic_spec_registry_reader`
(registry-consumer/Cargo.toml:2).
Binary `name`: `registry-consumer` (registry-consumer/Cargo.toml:10),
which is the release-archive filename per release-tools.yml:146
(`for tool in spec-compiler registry-consumer spec-lint codebase-indexer`)
and the file-name expansion logic at release-tools.yml:148–150.

Rationale per W-04 (`756cd87b` commit body, cross-referenced as claim):
binary name preserved for **release-artifact compatibility** — release
archives are named after the `[[bin]]` name, not the `[package]` name.
A comment at release-tools.yml:142–145 documents the
"Binary-name → artifact-name invariant" explicitly.

This produces three signals pointing in two different directions:

- **`open_agentic_spec_registry_reader`** (crate name) describes the
  crate's primary identity as the typed reader for the spec-spine
  registry.json. Internally correct.
- **`registry-consumer`** (binary + directory) is the legacy identity:
  the CLI someone runs to inspect a compiled registry. External
  consumers experience the crate by the binary they download from a
  release.

For a downstream user encountering `registry-consumer-<triple>.tar.gz`
standalone in a release archive, the binary name does signal "consume
the registry" — which is what the CLI does. It is not misleading. But
it is also not aligned with the crate's library-API role as a
typed-reader.

Post-split, "registry-consumer" as the release-artifact name is
defensible (the binary's job is to consume / report on the registry).
The directory rename to `tools/registry-reader/` (or similar) would be a
natural follow-up but the three-way mismatch is mostly cosmetic — the
external surface (binary + archive name) is the single signal external
users see, and it is coherent with the binary's actual function.

The W-04 trade — rename crate identity (library API), preserve binary
identity (release surface) — is sound given the priority on release
compatibility. The cost is that anyone reading the source tree first
encounters `tools/registry-consumer/` and has to discover the crate's
typed-reader role from the source.

**Verdict: sound-with-reservations.** The trade-off is defensible (a
release-archive rename would have larger blast radius than a
directory rename), the three-way mismatch is documented at the right
site (release-tools.yml:142–145), and split-readiness is unaffected.
The directory rename is a near-trivial follow-up if and when split
planning reaches the surface-naming pass.

### Q1c — featuregraph placement

`crates/featuregraph/Cargo.toml` confirms the post-W-05 dep graph:

```
[dependencies]
…
xray = { path = "../xray" }
open_agentic_spec_registry_reader = { path = "../../tools/registry-consumer" }
```

(featuregraph/Cargo.toml:20–21)

So featuregraph now consumes the spec-spine registry through the
typed-reader (good — the registry coupling that the footprint flagged
is resolved at the contract level). But the `xray = { path = "../xray" }`
dependency remains, and `xray` is an OAP-side crate (xray's spec is
`032-opc-inspect-governance-wiring-mvp`, an OPC concern). featuregraph
also remains imported by `axiomregent` and the desktop app (codebase
index Layer 1 dependency column, CODEBASE-INDEX.md:14 for axiomregent,
plus apps/desktop/src-tauri/Cargo.toml uses
`open_agentic_spec_registry_reader` at line 89 — featuregraph itself is
imported by axiomregent which the desktop runs).

featuregraph is therefore bidirectionally entangled with OAP at the
code level, but the entanglement is in the OAP direction
(xray ← featuregraph ← axiomregent / apps/desktop), not back-pressure
into the spec-spine. Its consumption of the spec-spine registry is now
through the typed reader's contract surface, not through ad-hoc JSON
parsing.

Placement in `crates/` rather than `tools/` reflects "this is an OAP
crate, not spec-spine tooling." That is correct under the current
shape. Post-split, featuregraph lives in OAP — it cannot follow
spec-spine because of xray, and it does not need to follow spec-spine
because its registry consumption is via the typed reader (a published
dependency).

The reservation is that featuregraph's *position in the dep graph*
(consumer of spec-spine, with OAP-only inbound imports) is now distinct
from its *crate-tree placement* (alongside other OAP crates in
`crates/`). The placement is correct for the post-split destination but
its current OAP placement reads as "indistinguishable from any other
OAP crate" — there is no architectural marker that it is on the
boundary between the two halves of the eventual split. A split planner
would need to recognise featuregraph as a typed-reader consumer (good)
that lives entirely in OAP (good) — and the absence of any in-tree
marker for that boundary is a documentation gap, not an architectural
one.

**Verdict: sound-with-reservations.** Placement matches the eventual
destination; the dep direction post-W-05 is split-compatible. The
reservation is purely about the discoverability of "this crate consumes
the spec-spine via the published typed reader" — there is no in-tree
breadcrumb beyond the manifest line, and crates/featuregraph's role on
the boundary is not surfaced in any cross-cutting doc.

---

## Q2. Architectural patterns introduced by Cut D

### Q2a — producer-also-hosts-reader

The pattern: each producer crate (`spec-compiler` produces
`registry.json`; `codebase-indexer` produces `index.json`) also exposes
`load(path) -> Result<TypedShape, Error>` as a library API alongside
its binary.

For registry-consumer, the typed-reader API is at
registry-consumer/src/lib.rs:234–541: `Registry`, `Feature`,
`ImplementsField`, `RegistryError`, `load`, plus the impl block
with `find_by_id`, `features_sorted`, `filter`, `status_report`,
`implementation_report`, `authoritative_or_allow_invalid`.

For codebase-indexer, the typed-reader API is at
codebase-indexer/src/lib.rs:69–143: `IndexReaderError`, `load`, plus
the `schema_v1_v2::parse` arm.

The pattern is operationally clean — spec 103's "consumer-binary
exception" applies once per artifact, and the typed reader's load()
function is the consumer-binary entry point for the library API
surface. Producers cannot read past their own contract (they parse the
artifact they emit through serde types they own).

Note though: registry-consumer's typed-reader API is materially richer
than codebase-indexer's. registry-consumer carries 7+ helper methods on
`Registry` (find_by_id, features_sorted, filter with `FeatureFilter`,
status_report, implementation_report, authoritative_or_allow_invalid)
plus the polymorphic `ImplementsField` helper enum with `paths()` and
`as_scalar()`. codebase-indexer's typed reader exposes effectively only
`load()` → `CodebaseIndex` and lets callers traverse the struct
directly; there is no `Index::find_mapping_by_spec_id()` helper or
similar. The asymmetry tracks consumer pressure: registry-consumer has
several callers exercising the filter/report surface (CLI subcommands +
oap-registry-enrich + apps/desktop); codebase-indexer's typed reader
has effectively **one** consumer today (`spec-code-coupling-check` via
the W-11 commit body's own disclosure, `7910af41`).

W-11 was explicitly framed in the plan as "contractual symmetry, not
architectural necessity." Reading the W-11 commit body (cross-referenced
as a claim, not a directive), the rationale is "every governed read
goes through the consumer binary's library API once per artifact." That
is a defensible posture — it makes the spec 103 rule load-bearing for
the index artifact, not just the registry artifact. The cost is API
surface and forward-compat machinery (the `schema_v1_v2` arm, the
`IndexReaderError` enum, four typed-reader tests at lib.rs:584–680)
with no current external consumer pressure.

The plan's choice of in-place restructure over a separate `*-reader`
crate is defensible: a separate crate would have published an empty
extraction-only surface with no logic of its own (the producer already
owns the serde shapes), and would have multiplied crate count for no
cohesion benefit. The realised in-place shape — producer hosts both
binary and library — is durable.

**Verdict: sound-with-reservations.** Pattern is durable as a primitive
(one consumer-binary library API per artifact). The reservation is
specifically about W-11: the cost is paid in code today
(deserialization arm + tests + error enum) but the benefit is preserved
*for future* governed reads of index.json. Whether the symmetry pays
for itself depends on whether consumer pressure ever materialises
beyond `spec-code-coupling-check`. The architecture commits to that
posture; whether it pays out is empirical.

### Q2b — enricher pattern

Two enrichers exist:

- `tools/oap-registry-enrich/`: reads `registry.json` via the typed
  reader (`srr::load`, oap-registry-enrich/src/lib.rs:105), walks
  `specs/*/spec.md` for `compliance:` frontmatter
  (oap-registry-enrich/src/lib.rs:124–149) and the repo tree for
  `.factory/build-spec.yaml` files (oap-registry-enrich/src/lib.rs:151–161,
  295–330), emits `registry-oap.json` as a sibling
  (oap-registry-enrich/src/lib.rs:117–119).
- `tools/oap-code-index-enrich/`: reads `index.json` via the typed
  reader (`load_index`, oap-code-index-enrich/src/lib.rs:79), walks
  `factory/adapters/`, `.claude/{agents,commands,rules}/`,
  `.github/workflows/` (oap-code-index-enrich/src/lib.rs:81–83), emits
  `index-oap.json` (oap-code-index-enrich/src/lib.rs:167–171).

Shape: **read via typed reader → walk additional sources → re-serialise
the original raw value with new fields overlaid → emit as sibling
artifact.** The overlay is done by mutating the raw Value pulled from
the typed reader (oap-registry-enrich/src/lib.rs:170–224;
oap-code-index-enrich/src/lib.rs:91–134). Both enrichers also stamp the
`build` block with `enricherId` + `enricherVersion`
(oap-registry-enrich/src/lib.rs:215–223;
oap-code-index-enrich/src/lib.rs:136–149).

For a hypothetical third overlay (supply-chain attestation, provenance
manifest, stakeholder-attribution): the pattern would absorb it
cleanly as a new enricher reading one of the two generic artifacts (or
both) via their typed readers, walking additional sources, and emitting
a third sibling. The architectural commitment is to a **flat sibling
artifact namespace** (`registry-oap.json`, `index-oap.json`, and any
future `registry-X.json` / `index-X.json` siblings) rather than
**overlay-of-overlays composition** (where a `registry-oap-then-Y.json`
would read `registry-oap.json` and overlay further).

That commitment is intentional but the architecture does not document
the alternative as foreclosed — there is no spec or doc explicitly
saying "overlays do not compose; each overlay reads the generic artifact
directly." A future enricher author would discover the pattern by
imitation, not by stated contract.

The bundle choice — neither enricher is in `release-tools.yml`
(confirmed at release-tools.yml:146 — the loop builds only
`spec-compiler registry-consumer spec-lint codebase-indexer`) — does
not signal "OAP-internal" structurally. The enrichers carry the same
`open_agentic_*` Cargo identifiers as the spec-spine tools; only the
release-bundle absence and the `oap-` prefix in the binary name flag
them as OAP-side. Spec 102 references the
`oap-registry-enrich compliance-report` invocation in the README's
quickstart (README.md:201–207), so the enricher binaries are
externally visible from source builds — they are simply not
release-bundled. That is a tactical bundle choice (smaller release
surface for non-OAP consumers) rather than a structural one.

Same with `policy-compiler`: per W-09 (`f5d85a09`, cross-referenced as
claim), it remains an OAP-internal tool because of the
`open_agentic_policy_kernel = { path = "../../crates/policy-kernel" }`
dependency (policy-compiler/Cargo.toml:22). It is structurally
OAP-coupled, which the bundle removal documents. The two enrichers do
not have a similar structural justification — they consume only
spec-spine surfaces and shared-types — so their bundle absence reads as
"OAP-naming + not released," not "OAP-coupled."

**Verdict: sound-with-reservations.** Pattern is durable for additional
overlays. Reservations: (1) the "no overlay-of-overlays" commitment is
not documented anywhere as architectural intent; a future overlay
author might reasonably propose composition. (2) The "OAP-internal" vs
"genuinely generic" boundary is signalled today by bundle inclusion +
binary-name prefix, not by manifest dependencies or any in-tree
declaration; the spec 103 read-discipline implies a discipline but
doesn't classify the producer set.

### Q2c — schema-version dispatch

The dispatch pattern in both typed readers:

- registry-consumer/src/lib.rs:415–465 — peek `specVersion`, accept
  prefix `1.` or `2.`, dispatch to a single `schema_v1_v2::parse` arm
  whose comment (lib.rs:429–436) explains "the structural shape of the
  Registry/Feature types is unchanged."
- codebase-indexer/src/lib.rs:116–143 — peek `schemaVersion`, accept
  prefix `1.` or `2.`, dispatch to a single `schema_v1_v2::parse` arm
  whose comment (lib.rs:130–137) explains "the 2.x bump removed Layer
  3-5 fields … so a 1.x index with those fields still decodes (serde
  silently ignores unknown fields) and a 2.x index without them decodes
  too."

Both arms work because the 1→2 transition either (a) only removed
fields that were `Option`/`#[serde(default)]` (registry side) or (b)
serde silently ignores unknown fields (index side). A future 3.0.0 or
4.0.0 bump that introduced fundamentally different shapes would need a
new arm. Neither typed reader documents an explicit
"two-major-version compatibility" guarantee — the comments document
the current implementation, not a forward-compat contract.

The Q2 verification failure on the axiomregent fixture (Fix 1, see
`spec-spine-cut-d-verification.md` cross-referenced as claim) — empty
`specVersion` rejected by `RegistryError::UnknownSchemaVersion("")`
— surfaces an undocumented rejection rule. Reading the dispatch code
(registry-consumer/src/lib.rs:418–426): if `specVersion` is absent or
not a string, `unwrap_or("")` produces empty string, the
`starts_with("1.")` / `starts_with("2.")` predicates both fail, the
`UnknownSchemaVersion("")` error path fires. That rejection semantics
is implicit, not documented; a user encountering it sees only the
error message at lib.rs:386 ("unsupported registry specVersion: ").

The same posture applies on the codebase-indexer side (lib.rs:119–127):
empty `schemaVersion` → `UnknownSchemaVersion("")`, same implicit
rejection.

Whether the rejection is correct (a missing specVersion *should* fail
fast — silent fallback to a default version would erode the dispatch's
own contract) is a separate question from whether the rejection is
documented. It is not documented.

**Verdict: sound-with-reservations.** Dispatch scales to additional
arms when the structural shape evolves; the current two-arm-shared-impl
pattern works because of the specific 1→2 transition shape. Two
reservations: (1) no forward-compat statement covers 3.0.0+; (2) the
empty-`specVersion` / missing-`specVersion` rejection is implicit in
the prefix-match logic and not surfaced as a documented contract.

---

## Q3. Hidden semantic decisions surviving Cut D

### Q3a — KNOWN_KEYS factoring as allowlist-vs-emitted

The relevant code: `tools/shared/spec-types/src/lib.rs:63–112`. The
9-line doc comment at lib.rs:63–74 explicitly describes
`KNOWN_KEYS` post-W-06c as a "permitted frontmatter" allowlist, not a
"fields emitted by spec-compiler" list. The text:

> "Cut D W-06c: `compliance` retained in this allowlist but NO LONGER
> emitted by spec-compiler … KNOWN_KEYS is a 'permitted frontmatter'
> allowlist, not a 'fields emitted by spec-compiler' list — the two
> were aligned before W-06c."

The W-06c commit body (`460f5bde`, cross-referenced as claim) carries
the in-code rationale: removing `compliance` from KNOWN_KEYS causes
`yaml_scalar_to_json` to encounter `compliance: [{framework, controls}]`
as the spec extraFrontmatter (because eight specs carry that
frontmatter — 047, 067, 068, 069, 102, 116, 121, 147), and the helper
only converts scalar leaves, firing V-002 on the sequence-of-mappings.

This is a workaround, *not* a redesign. The decision keeps the spec
corpus compileable while migrating `compliance` from emitted-into-
registry.json (pre-W-06c) to overlaid-by-oap-registry-enrich
(post-W-06c). The seam between "what KNOWN_KEYS permits" and "what
FeatureRecord emits" survived Cut D as a deliberate offset of one row
(the `compliance` key). It is documented as a workaround at lib.rs:63–74
and rationalized in the W-06c commit body — but the comment does not
explicitly mark itself "temporary" or "permanent." It describes the
current state.

The architectural question: is the workaround a clean seam, or does it
signal that `yaml_scalar_to_json` should widen?

Reading it as a clean seam: "KNOWN_KEYS is the grammar of permitted
frontmatter; FeatureRecord is the grammar of compiled output." That is
a defensible factoring — the two are conceptually distinct and one row
of offset (`compliance` permitted but not emitted) is a small price
for a clean rule.

Reading it as a workaround: the only reason the seam is offset is that
`yaml_scalar_to_json` (lib.rs:1373 in spec-compiler) does not handle
sequences-of-mappings. Widening that helper would let `compliance` be
removed from KNOWN_KEYS (treating it as extraFrontmatter passthrough),
which is what the plan *literally* called for. The W-06c commit body
acknowledges this gap explicitly ("the plan's *intent* is to drop
compliance from registry.json output, which is fully satisfied by
removing the `FeatureRecord.compliance` field (done)") — so the
factoring is the plan's *intent* achieved by a different mechanism, not
a redesign.

A third-party spec author with their own frontmatter extension (e.g., a
`my_org: { framework: x, controls: [a, b] }` field) cannot reach
extraFrontmatter passthrough today: `yaml_scalar_to_json` would refuse
the nested mapping. Their only options are (a) submit a PR to add their
key to KNOWN_KEYS (cross-cutting touch into a shared crate) or (b) be
subject to V-002. There is no "registered extension" mechanism.

Post-split, this becomes a load-bearing decision: third parties
extending the spec format cannot do so through frontmatter alone unless
their extension is scalar-shaped. The current architecture forecloses
nested-mapping extensions silently.

**Verdict: sound-with-reservations.** The factoring is defensible as a
clean seam *and* it is a known-temporary workaround — both readings are
true. The lib.rs:63–74 comment documents the current state without
committing to either persistence or removal. The third-party-extension
gap is real but unrelated to Cut D's deliverables; it would have
existed regardless. The reservation is documentation: the architectural
choice (allowlist-vs-emitted as the permanent factoring, or as the
workaround pending `yaml_scalar_to_json` widening) is not stated.

### Q3b — parallel-pipelines design

Confirmed post-Cut-D parallel-read shape:

- `tools/spec-compiler/src/lib.rs:117–744` reads `specs/*/spec.md`
  directly via the `compile` function; `compute_content_hash` at
  lib.rs:910–960 hashes the spec.md paths it discovered.
- `tools/codebase-indexer/src/spec_scanner.rs:37–71` reads
  `specs/*/spec.md` independently in `scan_specs`. Codebase-indexer's
  `collect_input_files` (lib.rs:427–538) gathers a strictly larger
  input set — Cargo.toml, package.json, spec.md, factory adapter
  manifests, .claude/{agents,commands,rules}, schemas/, workflows.

So the two pipelines read the same spec.md corpus through different
parsers (spec-compiler validates frontmatter strictly; codebase-indexer
extracts only `id`, `status`, `implementation`, `depends_on`,
`implements`, `amends`, `amendment_record`) and compute different
content hashes over different inputs. They are intentionally
independent.

`build-meta.json` shape:

- spec-compiler/src/lib.rs:732–737 emits
  `{builtAt, compilerId, compilerVersion}`.
- codebase-indexer/src/lib.rs:308–312 emits
  `{builtAt, indexerId, indexerVersion}`.

Neither cross-references the other's content_hash. There is no shared
"the index.json agrees with this registry.json version" provenance.

The architectural question: is parallel-read the right design?

The artifacts have different consumers and different lifetimes. The
registry is a deterministic compile of authored frontmatter into a
typed shape; the index is a structural inventory of crates, packages,
spec-to-code mappings, factory adapters, etc. Their respective content
hashes cover different inputs by design (the index changes when
Cargo.toml changes even if no spec.md changed; the registry changes
only when spec.md changes). Forcing them to share an authoritative
hash would be ill-typed.

Cross-pipeline consistency *is* effectively nobody's job today. Whether
that should change depends on whether a consumer ever needs to assert
"this index reflects this registry version." `spec-code-coupling-check`
(the W-11 typed-reader consumer) reads index.json and registry.json
side-by-side, but it does not assert hash agreement — it asserts
spec/code coupling, which is a different invariant.

**Verdict: sound.** Parallel-read is the right design for two
artifacts with independent consumer pressure and different input
surfaces. The absence of cross-pipeline provenance is intentional; if a
future consumer demands hash agreement, the question is whether that
consumer wants a third tool, an enricher responsibility, or a CI
assertion — and the current shape does not foreclose any of those.
build-meta.json's compiler/indexer-id namespacing means a future
"agreement" tool could read both build-meta files and a content hash
they each computed, without restructuring the producers.

### Q3c — G-2 (W-10) placement at emitter-side

Confirmed wiring after Fix 3:

- `crates/factory-engine/src/governance_certificate.rs:759–783`
  defines `validate_spec_id_resolution(&cert, repo_root)`. It loads
  registry.json via `open_agentic_spec_registry_reader::load` and
  checks `registry.find_by_id(spec_id)`.
- The three callers, all reached during Fix 3:
  - `crates/factory-engine/src/bin/build_certificate.rs:140–157` —
    `repo_root` cascades from CLI arg → `current_dir()` → `"."`. Calls
    `validate_spec_id_resolution(&cert, &repo_root)` and
    `write_validation_warnings(&warnings, &cert_path)`.
  - `crates/factory-engine/src/bin/factory_run.rs:50–82` (the
    `emit_certificate` helper) — `repo_root` is passed as a parameter
    from main at lines 401–405 (same CLI→current_dir→`"."` cascade).
    Same two-step `validate + write_validation_warnings`.
  - `crates/factory-engine/src/bin/verify_certificate.rs:56–75` — same
    cascade for `repo_root`, same two-step.

The three call sites use the identical resolution cascade. No caller
synthesises a placeholder root or hard-codes `Path::new("/")`. The
`repo_root` threading is uniform and clean.

Sibling-file pattern: `write_validation_warnings` at
governance_certificate.rs:788–806 writes
`validation-warnings.json` next to the certificate file (lib.rs:795–798
computes `cert_path.parent().join("validation-warnings.json")`). Empty
warnings → no-op (lib.rs:792–794): the sibling file's presence is
itself meaningful (absence == no warnings). This preserves the cert's
signed-bytes invariant: the cert struct is untouched; its content_hash
remains stable across the W-10 wiring.

The architectural question: is emitter-side warn-by-default right, or
should spec_id validation be a verifier-side gate?

The two placements have genuinely different failure-mode semantics:

- **Emitter-side**: "you tried to assert provenance for a spec that
  doesn't exist in this filesystem's registry at emit time."
- **Verifier-side gate**: "this cert references a spec the registry
  doesn't know about — refuse to verify it as valid."

The current implementation runs validation on **both** the emitter and
the verifier side (build_certificate.rs:145, factory_run.rs:69,
verify_certificate.rs:63 all call `validate_spec_id_resolution`). So
the trade is not "one or the other" — it's "both, warn-by-default,
env-gated promotion to hard-fail." The env-gate (lib.rs:811–816,
`OAP_REQUIRE_SPEC_ID_RESOLUTION=1`) is the lever that turns the warning
into an error.

Sibling-file vs embed: embedding into the cert would have required
bumping `certificate_version`, which would have invalidated every
existing cert fixture and forced a migration of the verifier's expected
hash. The sibling-file approach preserves the cert format completely.
That is a sound trade — the cert is the load-bearing signed artifact,
and validation findings are auxiliary diagnostics that do not need to
be inside the signed bytes.

Verifier-side detection still works because the verifier
(verify_certificate.rs:56–75) calls `validate_spec_id_resolution`
against its own filesystem's registry. The verifier does not "trust the
emitter's findings" because there are no embedded findings to trust —
both sides validate fresh.

**Verdict: sound.** Placement is correct (emitter-side + verifier-side
both validate; warn-by-default; sibling-file). The repo_root threading
through three call sites uses an identical cascade with no
placeholders. The signed-bytes-invariant preservation is the
load-bearing trade; embedding would have broken cert-format stability
for unclear gain.

---

## Q4. Split-readiness

Classified list:

- **Workspace structure** — *substantive work needed*. There is no root
  `Cargo.toml` workspace today (`cat Cargo.toml` confirms — file does
  not exist at repo root). Each tool has its own manifest with `path`
  deps pointing at sibling tools (e.g., `tools/oap-registry-enrich`
  depends on `tools/registry-consumer` via `path = "../registry-consumer"`).
  Post-split, a `tools/spec-spine/Cargo.toml` workspace root would
  group the 6 spec-spine crates (spec-types, spec-compiler, spec-lint,
  registry-consumer, codebase-indexer, spec-code-coupling-check),
  enabling a single `cargo build` and consistent dep versioning. Not
  done today; not trivial because adding a workspace root forces
  reconciling lockfiles and existing per-tool target dirs.
- **`schemas/` location** — *substantive work needed*. The two
  registry-format schemas live in different homes:
  `specs/000-bootstrap-spec-system/contracts/registry.schema.json` and
  `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json`
  on one side; `schemas/codebase-index.schema.json`,
  `schemas/codebase-index-oap.schema.json`,
  `schemas/agent-frontmatter.schema.json`,
  `schemas/skill-frontmatter.schema.json` on the other. Post-split,
  the spec-spine schemas (registry, build-meta, codebase-index) want a
  consistent home — either co-located with their owning specs, or in a
  unified `schemas/` directory under the spec-spine repo. Cut D did not
  align these.
- **Path dependencies** — *near-trivial follow-up*. Every spec-spine
  crate's Cargo.toml uses `path = "../..."` for its sibling
  dependencies (confirmed via `grep -n "path = "" tools/*/Cargo.toml`).
  The mechanical cost of switching to git/registry deps is small per
  manifest. The semantic implications (version pinning, breakage
  propagation, CI velocity) are larger but those are split-planning
  concerns, not Cut D ones. Cut D leaves path deps in the position
  where a `Cargo.toml`-by-`Cargo.toml` migration is the only change
  needed.
- **Artifact-emission directories** — *near-trivial follow-up*. Both
  generic producers emit to `build/{spec-registry,codebase-index}/`
  (spec-compiler/src/lib.rs:111–115; codebase-indexer/src/lib.rs:156–159);
  both enrichers emit to the same `build/{spec-registry,codebase-index}/`
  subdirectories as siblings of the generic artifacts
  (oap-registry-enrich/src/lib.rs:117–119;
  oap-code-index-enrich/src/lib.rs:167–171). `build/` reads as an OAP
  convention today (the OAP `make` targets bake it in), but the
  spec-spine producers don't reach outside of `build/<artifact-name>/`.
  Post-split, the spec-spine compiler would plausibly retain `build/`
  as its output convention when run standalone, or accept an output
  path. The current code (`repo_root.join("build/<x>")`) makes this
  parameterisable already.
- **Cross-repo CI** — *substantive work needed*.
  `.github/workflows/spec-conformance.yml` interleaves spec-spine tools
  and OAP enrichers in one workflow (lines 30–110 mix `spec-compiler`,
  `oap-registry-enrich`, `registry-consumer`, `spec-lint`,
  `codebase-indexer`, `oap-code-index-enrich`, `policy-compiler`).
  Post-split, the spec-spine repo's conformance suite needs only the
  4 spec-spine tools (the release-bundle set + spec-code-coupling-check);
  the OAP repo's suite handles the enrichers + policy-compiler +
  coupling-check. The factoring is achievable but not trivial — the
  workflow YAML rewrites are non-mechanical.
- **Test fixtures** — *done by Cut D*. Each crate's tests are
  self-contained: spec-compiler uses `tempfile` for golden fixtures
  (tests/golden.rs et al.); registry-consumer has 41 fixture files
  under `tests/fixtures/` (12 contract subdirectories per
  `ls tools/registry-consumer/tests/fixtures/`); codebase-indexer has
  inline-string goldens in `tests/golden.rs`. No cross-crate fixture
  imports were observed. The test layout survives a split unmodified.
- **AGPL-3.0 implications** — *near-trivial follow-up*. spec-spine
  crates + OAP crates + apps/desktop + platform are all AGPL-3.0 today
  (e.g., `featuregraph/Cargo.toml:1` carries the SPDX header). Post-split,
  spec-spine remains AGPL; OAP remains AGPL. The split itself does not
  change license inheritance. The spillover question — does AGPL on
  the spec-spine published crate force AGPL on downstream consumers? —
  is a licensing read, not an architectural one; the architecture
  permits but does not require the AGPL choice to persist post-split.
  Flagging as architectural spillover only: any future relicensing
  decision must move with the split, not against it.

**Summary judgment:** Split-readiness is primarily *substantive
follow-up work* (workspace root, schemas/ unification, CI rewiring),
not architectural rework. Cut D resolves the *coupling* questions
(typed-reader contract, enricher separation, factoryProjects/compliance
overlay) but does not pre-position the *packaging* questions
(workspace, schemas/, CI, license messaging). A pre-split engineering
pass would be needed; the work is mechanical-to-moderate, not
re-architecting.

---

## Q5. External contract and grammar surface

### Grammar locus

The spec format's authoritative rules are *not* in a single document.
They are distributed:

- `tools/shared/spec-types/src/lib.rs:75–175` — KNOWN_KEYS,
  VALID_KINDS, VALID_RISK_LEVELS, SHAPE_TABLE,
  CONVENTIONAL_CATEGORIES. The compiler-enforced grammar of the
  authored corpus.
- `specs/000-bootstrap-spec-system/contracts/registry.schema.json` —
  the JSON Schema for the compiled artifact (referenced by spec 000
  itself at spec.md:184 and 304).
- `specs/000-bootstrap-spec-system/spec.md` + the constitution
  (`.specify/memory/constitution.md`) reference the spec format but
  do not enumerate the grammar.

Spec-compiler **does not** self-validate against
`registry.schema.json` (no `jsonschema` import; the only occurrence of
"registry.schema.json" in `tools/spec-compiler/src/lib.rs` is a comment
at line 1124). Codebase-indexer **does** self-validate against
`schemas/codebase-index.schema.json` (codebase-indexer/src/schema.rs
imports `jsonschema` and validates at compile time;
codebase-indexer/src/lib.rs:300–304 invokes
`schema::validate_against_schema`).

This is an asymmetry: registry.schema.json is normative *documentation*
the spec-compiler does not enforce on itself, while
codebase-index.schema.json is the enforcement target the codebase-indexer
runs against every compile. An external spec author looking up "the
spec format" cannot grep one file; they have to consult
`tools/shared/spec-types/src/lib.rs` AND
`specs/000-…/contracts/registry.schema.json` AND the prose in spec 000
to assemble the full picture.

The JSON Schemas are normative reference; `spec-types/src/lib.rs` is
the run-time source. The two are aligned by review, not by code
generation; there is no generator that emits the schema from the source
types, nor a CI gate that asserts they agree.

**Verdict on grammar locus: sound-with-reservations.** The grammar is
internally consistent but distributed across three locations with no
single normative source. An external author has to learn the trinity.

### SemVer policy for schema

`SPEC_VERSION` is at `tools/spec-compiler/src/lib.rs:52` (`"2.0.0"`).
`SCHEMA_VERSION` is at `tools/codebase-indexer/src/types.rs:31`
(`"2.0.0"`). The dispatching code in both typed readers accepts
prefixes `1.` and `2.`. No documentation explains what a third-major
bump (3.0.0) would require or permit. The W-06c commit body
(`460f5bde`, cross-referenced as claim) describes the specific 1→2
transition for the registry as "removed fields that are
Option/serde-default in the typed reader" but does not generalise to a
SemVer policy.

The plan's Phase 7 open question #2 on SemVer policy (field-removal =
major, field-addition = minor, validation-tightening = ?) is not
resolved in the post-Cut-D repo. The schema-version-dispatch mechanism
is enough to handle two-major-version compatibility *today*; it is not
enough to communicate to an external consumer "what could break in the
next version."

**Verdict on SemVer policy: sound-with-reservations.** Absence is not
a blocker for current internal use (one-repo consumption, in-tree
typed-reader callers all rebuild at the version they need). For
external consumers picking up release-bundle binaries, the absence of
"what to expect across major versions" is a real gap. Scope-boundary
flag: full risk-surface analysis of "how external consumers absorb a
schema bump" is a separate pass.

### Release-bundle external contract

The 4 release binaries (`spec-compiler`, `registry-consumer`,
`spec-lint`, `codebase-indexer` — confirmed at release-tools.yml:146)
are the external surface.

Documentation status by tool:

- **registry-consumer**: depth model.
  `tools/registry-consumer/tests/fixtures/help_contract/` contains
  expected help-text fixtures (confirmed via
  `ls tools/registry-consumer/tests/fixtures/help_contract/`).
  `docs/registry-consumer-contract-governance.md` exists (lines 30,
  53, 57 read: "Distilled extension rule: accept an extension only when
  it adds one clear guarantee with minimal surface area, explicit
  mode/flag interaction rules, fixture-first contract coverage,
  including help surface" — read as architectural posture, not as a
  directive).
- **spec-compiler**: no parallel artifacts. No `help_contract/` fixture
  dir; no `docs/spec-compiler-contract-governance.md`.
  `tools/spec-compiler/tests/` carries 8 test files covering goldens,
  schema-conformance, V-codes, but no CLI-help fixture set.
- **spec-lint**: no parallel artifacts. Only `tests/lint.rs`.
- **codebase-indexer**: no parallel artifacts. Three test files
  (`tests/exit_codes.rs`, `tests/golden.rs`, `tests/schema_conformance.rs`),
  no CLI-help fixture set, no contract-governance doc.

The asymmetry is real but defensible: registry-consumer has 30+ specs
worth of contract surface (specs 002, 007–031 — confirmed from the spec
list during /init) because the spec spine itself iterated on every CLI
flag and output-shape commitment as separate contracts. The other three
tools have far smaller external surfaces — `spec-compiler compile` is
mostly silent on success / writes structured V-code diagnostics on
failure; `spec-lint --fail-on-warn` is a binary gate; `codebase-indexer
compile|check|dump-inputs` is three-subcommand.

So registry-consumer's depth is appropriate for *its* surface; the
others' lack of equivalent depth is appropriate for *theirs* — adding
help-contract fixtures for `spec-compiler compile` (which has 2 flags)
or `spec-lint --fail-on-warn` (which has 1) would be ceremony without
matching consumer pressure.

The reservation is what's missing for *external* consumers, not
internal: the four binaries ship in a release archive, but only
registry-consumer has the kind of stability commitment
(`registry-consumer-contract-governance.md`) that an external
integrator would look for before depending on it. The others' contract
shape is "trust the binary's `--help` output and the spec they
implement."

**Verdict on release-bundle external contract: sound-with-reservations.**
registry-consumer's depth is a model for *its* surface, not a template
that should be replicated across the other three. The reservation is
about external-consumer onboarding: there is no equivalent of
`registry-consumer-contract-governance.md` for `spec-compiler`,
`spec-lint`, or `codebase-indexer`, so an external consumer's only
contract reference is the binary's `--help` text and the owning spec.

---

## Reservations consolidated

Items returning sound-with-reservations and the shape (not content) of
what would resolve them:

1. **Q1a, shared-types responsibility-level cohesion.** A sub-crate
   decomposition (parsing helpers / vocabularies / diagnostic codes)
   would clarify cohesion at the cost of more leaf crates. The current
   packaging convenience is defensible; a split planner would surface
   this as a sub-decomposition question.
2. **Q1b, naming triples.** Directory rename to align with the
   typed-reader role would close the gap; the release-archive binary
   name should remain unchanged. This is cosmetic now, near-trivial
   later.
3. **Q1c, featuregraph placement.** The crate is correctly placed
   structurally; what's missing is an in-tree breadcrumb identifying it
   as a typed-reader consumer on the eventual split boundary.
4. **Q2a, W-11 contractual symmetry.** No external consumer pressure
   today; the cost (API surface + tests) is paid in anticipation of
   pressure that may or may not materialise. A future audit of
   index.json typed-reader callers would settle whether the symmetry
   pays for itself.
5. **Q2b, enricher pattern.** Architecture commits to flat sibling
   artifacts (`*-oap.json`) and not overlay-of-overlays composition;
   the commitment is not documented as architectural intent. A spec or
   doc stating "enrichers read the generic artifact directly; overlays
   do not compose" would close the gap. Also: the "OAP-internal" vs
   "spec-spine" boundary is signalled by bundle-inclusion only, not by
   manifest structure or naming convention.
6. **Q2c, schema-version dispatch.** No forward-compat statement
   covers 3.0.0+; the empty-`specVersion` rejection is implicit. A
   documented dispatch contract would close both.
7. **Q3a, KNOWN_KEYS factoring.** Documented as the current state in
   `spec-types/src/lib.rs:63–74` and rationalized in the W-06c commit
   body but not classified as permanent-vs-workaround. The
   third-party-extension gap (nested mappings cannot reach
   extraFrontmatter) is pre-existing.
8. **Q4, split-readiness.** Workspace root, schemas/ home, and CI
   factoring are substantive follow-ups. Each is mechanical-to-moderate
   work — not architectural rework.
9. **Q5, grammar locus.** Grammar distributed across three loci with
   no normative single source; no schema-from-types generator.
10. **Q5, SemVer policy.** Undocumented; the dispatch mechanism is
    enough for in-tree use but not for external-consumer migration
    planning.
11. **Q5, release-bundle contract surface.** registry-consumer's
    contract-governance depth is not replicated for the other three
    binaries; for those, the only stability surface is the binary's
    `--help` and the owning spec.

## Reconsiderations

None. No structural element in Cut D's shape is unlikely to survive
the split cleanly. Every reservation above is a documentation, naming,
or follow-up-work item, not a "this shape will need re-architecting"
finding.

## Final architectural verdict

**SOUND-FOR-NOW.**

The shape Cut D produced is internally coherent and split-compatible at
the *coupling* layer — typed-reader contracts, enricher separation,
overlay artifacts, schema-version dispatch, governance-certificate G-2
validation. The reservations are concrete, well-localized, and
*follow-up* in character (documentation, naming, workspace packaging,
CI factoring). None of them are pre-split blockers in the sense of
"the architecture must change before a split can be planned."

A split planning pass should expect to spend time on (i) schemas/
unification (split between specs/000/contracts/ and schemas/ root),
(ii) workspace root for spec-spine, (iii) CI workflow factoring
(spec-conformance.yml interleaves both sides), and (iv) external-consumer
contract surface for the three release-bundle binaries lacking
registry-consumer-style governance docs. None of those require
modifying the post-Cut-D code shape; they require documentation,
packaging, and (for CI) workflow YAML rewrites.

Cut D is one a clean split would inherit, given the reservations above
are addressed at split-planning time rather than carried forward as
hidden technical debt.
