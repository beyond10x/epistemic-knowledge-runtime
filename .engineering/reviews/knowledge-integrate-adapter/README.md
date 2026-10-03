# Bounded A/E integration conformance adapter handoff

Status: PARTIAL / GATE HELD. Source is final and frozen. No commit, publication, AEP write, whole-package gate, or implementation-conformance completion is claimed. Root owns integration and the missing Submit witness/F application work.

Tree id: ekr-knowledge-conformance-20261003. Branch: ekr/knowledge-conformance-20261003. Base: ab6dcf888. Managed tree is preserved. Own lease codex-ekr-integrate-conformance-20261003 was released. Compiler lane returned; all worker Cargo sessions are terminal, including final target session 56563 (exit 101) and bounded clippy session 63662 (exit 0).

## Exact owned source

The source-only patch `source.patch` contains exactly:

- crates/ekr/src/conformance/integrate.rs
- crates/ekr/src/conformance/knowledge.rs (new)
- crates/ekr/tests/conformance_integrate.rs
- crates/ekr/tests/fixtures/conformance/knowledge-empty-interpretation.json (new)
- crates/ekr/tests/fixtures/conformance/knowledge-empty-schema-proposal.json (new)

Patch SHA256: 86affa74ac99c9d7cd0348dce58e18fee1c95316949cdcd19461b6602527f205.

Root-owned specification, generated contracts, suite and baseline changes present in the tree are excluded from this patch. This source depends on the explicit finite fixture_inputs supplied by those root-owned changes. No new dependency or production signer was introduced. All executable changes are Rust.

## Behavior and measurement boundary

The adapter calls the real native Runtime on file and SQLite providers. Import/list/show interpretations, gap discovery, proposal submission/show and signed approval/rejection return their actual native command outcomes. Events and results come from returned records. Refusal fixture setup establishes concrete invalid input/state; it cannot choose the response. The target injects a caller-supplied synthetic public key and signing callback for independent host enrollment and exact protocol signing; it cannot synthesize admission success.

Finite Import and Submit payload fixtures match the unchanged generated literal documents exactly. They do not replace the command's document/proposal. The valid finite Import literal can execute. The current generated Submit literal has empty evidence/additions/mappings/sources and a foreign base schema. It remains refused; production admission was not weakened.

After approve/reject calls the adapter drops Runtime, opens it again with full replay enabled, and checks that schema_proposal_reviews contains the returned review_id and exact Approved/Rejected decision. Refused review calls must leave that list empty. These assertions executed successfully on both providers in final target runs. This is direct retained-state evidence, distinct from emitted-event checks.

## Red evidence and final report/2 results

The authored test was first run before adapter implementation: `red.log`, `red.status` exit 101, session 82388 terminal. The observed File report showed 2 passed, 0 failed, 0 error, 17 unsupported, 0 skipped, total 19. That initial assertion stopped before SQLite; it does not establish a pre-change SQLite measurement. The targeted Rust test was 0 passed / 1 failed / 0 ignored / 5 filtered.

Final run command: `cargo test -p ekr --test conformance_integrate -- --nocapture`, with EKR_INTEGRATE_REPORT_DIR set to the evidence run directory and the coordinator's sole shared compiler-lane environment (one job, incremental disabled, dev/test debug disabled and symbol stripping, offline, RUSTC_WRAPPER unset).

Authoritative raw evidence: `final-3.log`, `final-3.status` exit 101 and `reports-final-3/*.json`. Rust target: 5 passed / 3 failed / 0 ignored / 0 filtered. Both providers were measured before aggregate assertions. On each provider, report/2 records:

| Total | Passed | Failed | Error | Unsupported | Skipped | Answered (passed+failed) |
|---|---|---|---|---|---|---|
| 19 | 16 | 1 | 0 | 2 | 0 | 17 |

Thus the measured passing count rose from 2 to 16 on File. Fourteen new A/E scenarios pass; one newly exercised scenario fails. Do not describe that failure as coverage progress or describe this as full conformance. SQLite has the same measured final counts; its pre-change count was not independently captured in the initial red run.

Remaining failed scenario:

- ekr.integrate.SubmitSchemaProposal/outcome/answered: actual native outcome is refused, where the generated case expects answered and its success event. Exact input and diagnostics are in final-3.log. The report records the outcome/event mismatch, not the native semantic refusal reason; the invalidity explanation above is source/fixture inspection, not a separately captured error-code observation.

Remaining unsupported scenarios and target refusal reasons:

- ekr.integrate.ApplySchemaProposal/outcome/answered: Story F schema application is not implemented; this target cannot publish schema or mapped facts.
- ekr.integrate.ApplySchemaProposal/outcome/refused: Story F schema application is not implemented; no application refusal can be exercised.

The original answered floor 19, zero-unavailable ceiling, zero quarantine and scenario-content floor remain enforced and RED. No scenario was deleted and no gate was relaxed. The additional intended-17-passing test also remains RED, exposing the Submit fixture limitation.

The coordinator reported ESS 0.52.0 refuses recursive typed document/proposal fixture binding with `recursive response type cannot be finitely admitted`; this worker did not independently reproduce upstream synthesis in this bounded unit. Root retained finite field bindings and original finite literals. Upstream witness support/authored finite admissible cases remain coordinator work.

## Red-capable controls

Both controls run the same admitted suite through the native adapter and compare new failures against its actually measured baseline, so the pre-existing Submit failure is never counted as a killed behavior.

- Event/response suppression: 8 passed, 9 failed, 0 error, 2 unsupported, 0 skipped on each provider. Eight additional positive cases fail: ApplyExtraction/applied; ApproveSchemaProposal, DiscoverSchemaGaps, ImportInterpretation, ListInterpretations, RejectSchemaProposal, ShowInterpretation and ShowSchemaProposal/answered. This tests observable projection and is not evidence of persisted-state corruption.
- Actual proof-signature corruption: 14 passed, 3 failed, 0 error, 2 unsupported, 0 skipped on each provider. The two additional failures are ApproveSchemaProposal/answered and RejectSchemaProposal/answered. The command's signature is corrupted before native admission; actual admission refuses. This is distinct from suppressing output.

The dedicated mutation-control Rust tests pass. Persisted review assertions described above independently verify retained review state after reopen/full replay. No claim is made that event assertions alone prove side effects.

## Other verification and limits

Bounded `cargo clippy -p ekr --lib --test conformance_integrate -- -D warnings`: exit 0 (`clippy.log`, `clippy.status`). Exact-file rustfmt check and owned-path git diff --check: exit 0 (`format.*`, `diff-check.*`). No full package, task check, CI image or release verification was performed for this unit. Prior intermediate runs are retained; final-3 is authoritative. Three consecutive runs of identical final source and a CI-image run were not performed, so this is a bounded local observation, not a stability or CI claim.

The fixed runner clock produced report completed_at=1700000003900. This is synthetic fixture-clock time, NOT actual execution UTC or freshness evidence. Preserve exact report bytes and do not ingest that field as current AEP evidence. Host status-file mtime observations separately place final-3.status at 2026-10-03 21:00:17.538522225 UTC and clippy.status at 2026-10-03 21:01:09.861162395 UTC; these are filesystem observations, not signed run timestamps.

All eight JSON documents in `sanitized-reports/` were inspected for local user/home/tmp/worktree paths; none occurred, so byte-identical copies were made and compared with the originals. Their report identity is spec digest f356a296c15cb806b72a218ce953f41b3edb6194bcefd41f350d20df79d80073, suite digest sha256:c7104160620dad2bc5c37ff715b8fa6455d37fd247b79da945e66c36eb8f0563, ess-conformance/19. `SHA256SUMS` holds artifact hashes with relative paths. Raw logs remain private evidence and can contain local paths; they are not the sanitized publication artifact.

Next owner: root. Apply/review the owned-only patch, retain the explicit failed floor, and resolve the positive Submit witness and F application before claiming the full integrate gate passes.
