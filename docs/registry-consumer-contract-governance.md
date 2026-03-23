# Registry Consumer Contract Governance

Status: **Contract stabilization complete; controlled extension mode active**.

This document defines how changes to `tools/registry-consumer/` are classified and reviewed.

## Stabilization boundary

The registry-consumer interface is now a governed contract-bearing tool. The following surfaces are normative:

- JSON/content contracts (`010`, `018`, `021`)
- README examples contracts (`019`)
- Runtime failure contracts (`020`)
- Help/usage and version contracts (`022`, `024`)
- Argument-validation contracts (`023`)
- Default-path and policy-override contracts (`025`, `026`)
- Sorting-order contracts (`027`)
- stdout/stderr channel contracts (`028`)

Normative behavior is defined by fixture-backed tests under `tools/registry-consumer/tests/fixtures/` and contract tests in `tools/registry-consumer/tests/cli.rs`.

## Change classification rubric (required)

Every change touching `tools/registry-consumer/` MUST declare one class:

1. **contract extension**
   - Adds new externally visible behavior and new contract fixtures/tests.
2. **internal refactor, no observable change**
   - Improves internals only; existing contract fixtures/tests remain unchanged.
3. **breaking change candidate**
   - Alters settled observable behavior; requires explicit versioning and migration language.

## Release gate checklist (required)

For every PR touching `tools/registry-consumer/`, answer:

- Does this change observable output?
- Does this change ordering?
- Does this change stdout/stderr routing?
- Does this change exit behavior?
- Does this require a new or updated fixture?
- Does this require explicit versioning language?

If any observable behavior changes, fixture updates and contract rationale are mandatory.

## Contract baseline corpus

The current fixture set under `tools/registry-consumer/tests/fixtures/` is the baseline contract corpus.

Future work is judged against this baseline. Do not update fixtures to match implementation drift without explicit contract justification.

## CI expectation

Main validation path must run both full registry-consumer tests and explicit fixture-bearing contract subsets.
