## Change classification (required)

Select one:

- [ ] contract extension
- [ ] internal refactor, no observable change
- [ ] breaking change candidate

## Registry-consumer contract checklist

If this PR touches `tools/registry-consumer/`, complete all:

- [ ] I assessed whether observable output changes.
- [ ] I assessed whether ordering changes.
- [ ] I assessed whether stdout/stderr routing changes.
- [ ] I assessed whether exit behavior changes.
- [ ] I added/updated fixture(s) when observable behavior changed.
- [ ] I added explicit versioning language if this is a breaking change candidate.

### Registry-consumer extension check

- [ ] This change adds one clear guarantee with operator or automation value.
- [ ] The surface area is minimal and does not overlap existing semantics.
- [ ] Flag/mode interactions are explicit and documented.
- [ ] Observable behavior is fixture-backed, including help output when applicable.
- [ ] Settled guarantees are unchanged, or this is explicitly classified as a breaking change candidate.

## Notes

- Existing fixtures under `tools/registry-consumer/tests/fixtures/` are normative contract baseline.
- Do not update contract fixtures to match implementation drift without explicit contract justification.

### Governance evidence

- Fixtures touched: `<paths or "none">`
- Spec/doc touchpoints: `<paths or "none">`
