# Signed-answer implementation conformance

Story `story:resolve-knowledge-question`. These are local Rust-runner reports over the real Runtime,
not independent review or final delivery acceptance. Source and contract digests are recorded in
`measurements.json`; `suite-input.json` is the exact admitted selection, including its parent suite.
The authored scenarios and generated suite live under `crates/ekr/tests/fixtures/conformance/`.

`human_resolution_preserves_evidence_and_history` signs a choice before an unrelated ordinary
transaction, then applies that original signed answer. `changed_answer_basis_requires_review` signs
before a new competing assertion and its evidence are admitted through ordinary propose/validate/
commit; the stale answer refuses and a separately signed renewed answer succeeds. Every command and
view reopens File or SQLite with full replay. Positive assertions inspect authenticated answer rows,
the original signed basis and operator, the accepted claim, retracted competitors, settled counts,
retained source evidence and the original revision root. Teardown checks the original retained
bytes and event prefix. Refused answers must change no published event.

Fixture input is independently signed with a disposable test-only key. The external-outcome hook
acknowledges an already established fixture precondition, stores no requested result and cannot
select the command's outcome. The command result comes from the actual kernel. Views read durable
runtime state rather than fixture bookkeeping; evidence rows also read and hash the retained bytes.

In the first executable reports each provider had 1 passed and 1 unsupported scenario: the adapter
had not acknowledged the external refusal fixture. Correcting that adapter gap yielded 2 passed,
0 failed/error/unsupported/skipped on each provider in three consecutive runs (`final1`–`final3`).
Each corresponding inert target produced 0 passed and 2 failed, with no skips/errors. The complete
knowledge-conformance test target also passed all 9 Rust tests, including existing A/B cases and
controls; repeated runs of the new two tests are not additional distinct acceptance cases.

Two deliberate production mutations establish that the scenarios can detect wrong behavior:

| Mutation | Observed result on both providers |
| --- | --- |
| Bypass material digest checks in both verification and correction derivation | changed-answer-basis-requires-review failed; human-resolution passed |
| Require exact observed revision equality despite unchanged material basis | human-resolution-preserves-evidence-and-history failed; changed-answer-basis passed |

Bypassing only the verifier's material check did not change either verdict (`material-mutant`): the
separate correction-derivation check still refused the stale input. `material-mutant2` disables both
checks for the same rule. Exact patches are retained, and both production files were restored
byte-for-byte before all three final runs; their original hashes are recorded. No mutation remains.

ESS validated all 9 specification files. Fresh synthesis produced 118 scenarios including these
2 authored cases with zero refusals; a second synthesis matched the committed suite exactly. The
coverage claim selects only the two executed authored cases, not all 118. The existing freshness gate
now includes this suite. Targeted Clippy with warnings denied and actual `xtask fmt --check` passed.

These runs used the previously pinned development generator, not release adoption. ESS 0.52.0 was
observed publicly released during this work; its assets and generated outputs still need verified
adoption. The CI job uses `ubuntu-latest`, Rust 1.98.1 and the full `task check`; its pinned ESS 0.36.0
and all existing suite freshness checks must be updated together during adoption. No CI-image run or
full-gate pass is claimed here. C final acceptance and independent review, D–F, both required demos
and the final full gate remain outstanding.
