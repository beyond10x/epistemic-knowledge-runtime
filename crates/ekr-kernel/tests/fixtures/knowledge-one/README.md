# Native knowledge/1 stores

These synthetic stores were written by the unmodified kernel at
`0333f526ea7b607c1477fddcd4758d26de0d1694` (the capture checkout also contained planning-only
commit `e023a32c2c646fd57d30419f0807fbff714c7edd` and ESS comments). `git diff --exit-code --
crates/ekr-kernel/src` passed before capture. The retained `capture.rs` was temporarily installed as
`crates/ekr-kernel/tests/schema_evidence_legacy_capture.rs`, with the existing authority-review
fixture; `EKR_LEGACY_CAPTURE` selected an empty owned output directory.

The capture explicitly asserts `ekr.knowledge-deterministic/1`, writes a schema-evidence rejection
and a still-validated ordinary transaction after the reviewed transition, and records their IDs,
original roots and independently provisioned public reviewer binding. The private signer is the
existing synthetic test key, never an operator credential. Both native writers finished and
closed before these bytes were copied. No new-kernel writer regenerated these fixtures.

Verification command: `cargo test --locked -p ekr-kernel --test schema_evidence_legacy_capture`.
After clearing this task's completed kernel package artifacts to prevent a stale cross-checkout
library, the actual result was one passed, zero failed. The earlier stale-library attempt failed
the explicit knowledge/1 assertion and was not used. Raw logs remain in the task's retained cache;
the checked-in inputs are the successful native file and SQLite stores.

`schema_evidence.rs` copies them to a temporary directory before opening or upgrading them. Its
checks preserve the historical roots and rejected decision, invalidate the pending validation,
activate the exact reviewed newer profile, commit a supported schema change, and reopen using
full replay. Never overwrite these fixtures to make a changed replay rule pass.
