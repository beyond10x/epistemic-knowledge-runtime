# Durable reviewed answer publication

The source files in tested-source.json implement the generated HumanAnswerRecord and AnswerReceipt through an atomic AttentionAnswered occurrence (revision envelope /4, recovery preparation /5). Answer identity is a checked wrapper around the generated HumanAnswerId. Historical event indices, envelope dispatch and absent answer-key bytes remain unchanged. The production Commit API now invokes private signed-answer admission, seals the exact derived transaction, retains its statement/proof/policy/transaction, applies it and recomputes disputes. Read-only answer history returns immutable generated records.

Replay re-verifies the human signature under independently provisioned retained policy, material basis and decision predecessor; derives and validates operations again; and compares the entire generated record, transaction bytes, validation hash, validators, receipt and resulting roots. Exact retries return the original record without sampling time, including after another answer settles the question. Changed request material refuses. Answer transaction identities cannot be reused by ordinary proposals.

Both file and SQLite tests publish Choose, Retract, CorrectTime and Unresolved; reopen and fully replay; preserve historical graphs, original valid intervals and retained support; and reject re-addressed record tampering. Unresolved and still-overlapping temporal corrections remain visible and can be resolved by a subsequent correctly chained answer. The fixture first attaches new evidence through an ordinary committed transaction. This caught a real defect: incremental replay omitted bytes needed by fresh material review, so the answer command now loads complete retained evidence before admission. evidence_red.txt records the missing-object failure, and evidence_green.txt the fix.

Recovery tests inject an unsent request or lost response at the provider port and retry in a fresh process with full replay on both providers. The returned bytes equal the original elected record. This is process-boundary recovery with port-level faults, not a native mid-transaction crash witness.

Mutation: removing the exact regenerated-record comparison admitted a forged evidence digest and failed the tampering test; record_mutant.txt records that failure. Source was restored byte-for-byte before subsequent changes and verification. An initial temporal-test expectation was corrected because an unbounded competing claim still overlaps; that fixture correction is not a red implementation test.

Measured final verification: graph/store tests {"passed":276,"failed":0,"ignored":3} across 65 result rows; selected kernel lib, authority_upgrade, durable_commands, attention_answers_recovery and human_review tests {"passed":66,"failed":0,"ignored":0} across 5 result rows. Workspace all-target Clippy and xtask fmt --check passed. Eight compile-fail snapshots were reviewed and refreshed only for diagnostic qualification/help-list differences; error codes, forbidden source expressions and primary diagnostics are unchanged. The non-overwrite graph rerun passed. Source-scanning guards now handle inline tuple variants and multiline field attributes.

Existing ignored executions:

- test eventlog::retention_faults::retention_crash_child ... ignored, child-process entry point; executed by abrupt_exit_retention_reopens_without_duplicates
- test writer_process_beside_the_read_only_opens ... ignored, helper: runs only as the writer process another case starts
- test measure_two_history_reads_on_one_handle ... ignored, measurement; prints timings

Raw logs remain outside disposable build output; only owned paths are sanitized here. These are root-local implementation checks, not independent review, named C conformance or task check. CLI, Runtime/session/SDK answer writes and history, explanation integration, native crash coverage and C conformance remain. D–F, released ESS adoption, both demonstrations and the final full gate still gate PR64 readiness.

The integration public-surface guard then identified three new exports without direct test references. A test-only follow-up checks the version-four constant, the checked answer-ID getter and the generated-backed slot identity round trip/refusal. surface-followup.json records that source change over the initial checkpoint; its 12 public-surface and 10 revision-event tests pass. The initial tested-source.json remains the initial checkpoint manifest.
