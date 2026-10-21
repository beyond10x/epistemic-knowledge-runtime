---
format: aep.planning-md/3
id: review-result:adversary-w20261021b-u1-pass-2
kind: review-result
status: active
title: Adversary pass 2 on wave 2026-10-21b U1 (profile v4)
relations:
- reviews: story:supersession-verdict-fixes-replay-safe
revision: 1
---
Owners: findings 1-5 are the U1 implementor's (`bb5b5b05b`, correction 1 `d11b3acdf`).

Adversary pass 2 (`aep:adversary`) on U1 of wave 2026-10-21b at `d11b3acdf`. Two new test files:
`crates/ekr-kernel/tests/adversary_w20261021b_supersession_v4_pass2.rs` (5 cases, 3 red) and
`crates/ekr-sdk/tests/adversary_w20261021b_supersession_v4_pass2.rs` (2 cases, 1 red).
`cargo test -p ekr-kernel --no-fail-fast`: 711 passed, 3 failed, 5 ignored; `-p ekr-sdk`: 195
passed, 1 failed, 3 ignored (both exit 101, every failure in the new files).

| # | where | finding | sev | coordinator's routing |
|---|---|---|---|---|
| 1 | `crates/ekr-kernel/src/validate/structural.rs:378` | v4 `alias-already-exists` for a committed-superseded holder names its `by` as where the alias resolves when the same transaction supersedes that `by`, contradicting `node-superseded` in the same rejection | warning | fix in U1 (correction 2): follow the transaction's supersessions as `Known::node` does |
| 2 | `validate/reference.rs:338` | the F2 fallback advises superseding by a node the same transaction supersedes (a cycle); following it is refused twice | note | fix: a v4 message that names the cycle |
| 3 | `validate/reference.rs:488` | v4 `node-superseded` follows one hop and names a node the transaction also supersedes in a chain | note | coordinator's decision: follow to the end of the chain with a cycle guard; design § 112.2 says so |
| 4 | `validate/reference.rs:527` | no test reached the v4 edge message's `Some(next)` branch before the added guard | note | kept (guard green) |
| 5 | `crates/ekr-sdk/src/document/transaction.rs:30` | `from_yaml` claims to read as `ekr propose` does, but accepts an `!AddAssertion` citing one evidence id twice that propose refuses under every profile (SDK `Assertion.evidence` is a plain `BTreeSet`, pre-existing) | note | fix the claim: both doc lines narrowed to the `!SupersedeNode` profile rule; SDK decoding unchanged |

v1-v3 identity held on the pass's four new inputs, both providers (green case
`v1_to_v3_answer_this_pass_s_inputs_as_released`).

```findings
[{"file":"crates/ekr-kernel/src/validate/structural.rs","line":378,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"v4 alias-already-exists for a committed-superseded holder names its by as where the alias resolves even when the same transaction supersedes that by, contradicting node-superseded in the same rejection"},
{"file":"crates/ekr-kernel/src/validate/reference.rs","line":338,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"F2's fallback advises superseding by a node the same transaction supersedes (a cycle), and following the advice is refused twice"},
{"file":"crates/ekr-kernel/src/validate/reference.rs","line":488,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"v4 node-superseded follows one hop and names a node the same transaction also supersedes in a chain"},
{"file":"crates/ekr-kernel/src/validate/reference.rs","line":527,"category":"mutant","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"no test reached the v4 edge message's Some(next) branch before the added guard; mutant run not executed"},
{"file":"crates/ekr-sdk/src/document/transaction.rs","line":30,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"from_yaml claims to read as ekr propose does but accepts an AddAssertion citing one evidence id twice that propose refuses under every profile"}]
```
