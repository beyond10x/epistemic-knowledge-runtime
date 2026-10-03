# Signed schema proposal reviews — implementation checkpoint

This increment retains externally signed approval/rejection decisions without advancing canonical
knowledge. It authenticates the enrolled operator, exact proposal bytes, historical/current review
material, statement and previous human proof. Exact retries recover the elected record even after
a later rejection. A shared store binding prevents decision identity reuse across reviews,
authority upgrades and attention answers. Source history includes store correction 8369cb18b and
its merge 21668c56c; kernel/CLI/SDK changes accompany this record.

Affected dispute components contribute complete evidence/options/effects digests, including
competitors that a correction does not explicitly select. The common applicability validator
rejects invalid corrections before proposal retention or approval. Rejections remain inspectable
and leave the current attention projection. The read-only proposal page renders authenticated human
statements as escaped text. Application of approved proposals remains F work.

## Observed verification

- `integrated-kernel-3.log`: 34 passed, zero failed/ignored across signature codec/verifier,
  schema review admission, upgrade and answer recovery targets. New review cases use file and
  SQLite, historical reads/full replay, cross-kind identity reuse and concurrent exact retries.
- `integrated-cli.log`: one signed CLI dispatch case passed on both providers. Its command filter
  excluded the docs/session test bodies; those zero-case runners are not counted as verification.
- `sdk-docs-green.log`: the separate unfiltered run passed 18 documentation tests and two
  CLI/SDK session tests. The session test includes typed approval/rejection refusals without trusted
  enrollment; positive signatures execute through native CLI dispatch with independent host trust.
- `clippy-4.log`: all targets in ekr-kernel, ekr-sdk and ekr passed with warnings denied.
  Earlier lint attempts found Copy clones, needless test borrows and duplicate fixture inclusion;
  corrections preserve assertions and use the already shared fixture module.
- `contracts-check.log`: the pinned released ESS regenerated both complete artifact trees with
  no drift, and the generated semantic workspace compiled. This is not implementation conformance.
- `kernel-package.log`: the complete ekr-kernel package passed 611 tests with six existing ignores,
  including its compile-fail membrane cases. This run contains seven schema-review cases; the later
  correction-history regression is verified separately.
- `correction-history.log`: the additional review regression passed on file and SQLite. A signed
  correction review survives unrelated advancement, remains historical after resolution, and
  recovers its exact retry while a fresh inapplicable approval is refused.
- `viewer-capture.log`: the real CLI dispatch fixture passed on both providers and saved its
  Rust-rendered proposal HTML. `schema-review-history.png` is a headless Firefox rendering of the
  file-provider HTML, visually inspected for both decisions, reviewer identity and escaped human
  statement text. This is a saved-page rendering, not a live HTTP/browser write test.

Red evidence is retained alongside green evidence. `historical-red-3.log` accepted an impossible
historical review basis. `identity-red.log` accepted a reused upgrade decision UUID.
`component-red.log` accepted approval after a valid third claim joined the selected dispute while
the selected row remained unchanged. `correction-applicability-red-2.log` accepted a blank reason.
`cli-ui-red.log` left a rejected proposal in attention. These are behavioral failures, not compile
failures. `correction-support-red-3.log` stopped at assertion-not-active; its proposed attachment
route was unreachable and is explicitly withdrawn by the review reports.

Independent review reports record their source/log-only scope and do not claim independent test
execution. The immutable planning records preserve original findings and subsequent corrections.
Raw evidence remains outside disposable outputs; copies here replace local path prefixes and
omit trailing empty log lines.

## Remaining delivery work

Final formatting, current conformance adapters and full task check remain separately recorded
gates. Regeneration adds the already-declared discovery refusal scenario
to the integrate inventory: its floor rises from 18 to 19 with a zero unavailable ceiling; existing
floors are not weakened. This checkpoint is not full E/F acceptance or PR completion.

The pinned ESS conformance compiler refuses recursive document/proposal fixture contracts;
`fixture-generation/` preserves that refusal and supported-alternative probes. The current suite
keeps finite fixture bindings and literal document/proposal inputs, preserving the same 19
obligations. Synthesis succeeds but a generated proposal without supporting evidence cannot
legitimately pass admission. The dependency blocker records this missing witness capability;
neither successful synthesis nor authored coverage discharges the generated execution floor.

Legacy published signed preparations retain exact receipts. Unpublished legacy signed preparations
refuse human-decision-revalidation-required and retain their immutable command slot; automatic
same-slot migration is not implemented. Mixed old/new writer binaries remain unsupported. These
limits are distinct from revalidating ordinary pending transactions after authority upgrade.
