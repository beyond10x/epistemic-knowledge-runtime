# Released ESS contract adoption checkpoint

ESS 0.52.0 is the published generator, at source
`4d6a4ecafc0feb4e11e4bee777b19c7351fa3647`. `release.json` records the public release,
uploaded assets and successful exact-commit workflows. The downloaded Linux archive matches
SHA256SUMS: `54ae56c63135afc9d8da12e36ef3dcc3d6c4ec8fdc6c2c30fb34bd2d8dc8a5ce`.
The extracted executable reports 0.52.0 and hashes to
`7c0f35fd2c2365c2390113f756eba1275ea25902d04f7ca5a8f8f6ab30395be7`.
No ESS source, release, trust policy or tag was changed by this adoption.

The EKR source digest remains
`16f0bcba9385e76553b16321dba10f073fcbbf1d53cc29a9a686cf2a907b6146`;
the semantic contract digest remains
`b3aecc34f60f0ce8f8908536c64e38d6497f7a61baffe2b2c753efbc70236b8c`.
Generated artifacts are exact tool output, including the typed timestamp representation.
The generator plan still distinguishes generated capabilities, runtime obligations and
caller-identity refusals; compilation does not establish those obligations.

## Verified behavior

- `contracts-check.log`: exact released executable verification, both complete artifact trees
  regenerated without drift, and synthesized workspace compilation. No development-candidate flag.
- `spec-fresh.log`: specification validation and all six complete conformance suites regenerated
  and compared byte for byte.
- `kernel.log`: 62 targeted Rust tests pass across kernel unit, retained knowledge, human review,
  authority upgrade and answer recovery targets. The native recovery cases include both providers.
- `public.log`: six CLI/session, read-only viewer and ESS pin-control tests pass.
- `sdk.log`: sixteen SDK tests pass, including generated document ingress and the transitive
  no-runtime/no-storage boundary.
- `guards.log`: product-member formatting check, fourteen xtask tests (including the seven
  drift controls), 53 identity tests and fourteen workspace architecture tests pass.
- `clippy-all.log`: workspace Clippy, all targets, with warnings denied.
- `views.log` and `views-*-report.json`: all 78 component scenarios pass on each provider.
- Named retention, upgrade and answer report/run files: respectively 3, 3 and 2 real scenarios
  pass on each provider; inert controls fail every selected scenario as required. The nine Rust
  tests that run these cases pass. Each family shares the retained `*.suite-input.json` bytes;
  identical inputs are stored once, including their full parent lineage.

## Timestamp regression and target corrections

Historical answer format /1 could retain correction times with trailing fractional zeros or
explicit zero offsets. The new typed serializer normalizes those spellings. Replay now compares
the complete expected record using the original spelling only for the two correction bounds,
after checking their instants; retained bytes, addresses, signature encoding and ordinary
derived transactions are unchanged.

The provider-backed full replay test covers three equivalent historical spellings and refuses
changed milliseconds, sub-millisecond precision and malformed text. Removing the compatibility
comparison fails with `answer-record-not-canonical` (`legacy-time-mutant.log`). Both mutated
call sites were restored byte for byte before the final kernel run. The restored durable source
SHA256 before subsequent formatting was
`160978bcc2647fd7286febba53b8b6f17109438dda6f3def975d6deb0d4e8dc7`.
The legacy variants are constructed retained-history test inputs, not a claim that a separately
built historical executable wrote them to a native provider.

ESS 0.52 also synthesizes external controls in authored timelines. The kernel target now
verifies the captured proposal instead of requiring a particular fixture filename. The views
target preserves explicitly named fixture stores instead of replacing them with a generic
refusal fixture. Actual handler results still determine outcomes. The first views run caught
six target errors and one incorrect missing-node result per provider (`views-before.log`);
the corrected run passes every original scenario without editing authored expectations.

## Remaining gate failures

The complete regenerated inventories now require 72 kernel and 18 integrate scenarios, with
zero unavailable allowed. `component-conformance.log` and the kernel reports show:

| Component | Per-provider pass | Unsupported | Failed/error/skipped |
| --- | ---: | ---: | ---: |
| Kernel | 62 | 10 | 0 |
| Integrate | 2 | 16 | 0 |

The missing component bindings are the newly declared attention/upgrade/answer and
interpretation/schema-proposal commands. The named A/B/C suites separately exercise their
implemented behavior. The component tests remain red until every new binding is implemented;
no scenario was removed or floor lowered. All previous step floors were preserved or raised
to cover the newly synthesized checks.

This is coordinator-local evidence, not independent review or a complete `task check`.
The adoption story, A–F delivery, both demonstrations and PR64 remain unfinished.
