---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-q-pass-1
kind: review-result
status: active
title: Seed and constrained type counts adversary pass
relations:
- reviews: task:quality-counts-seed-evidence-and-constrained-types
revision: 1
---
unit: task:quality-counts-seed-evidence-and-constrained-types at cadbc785a, on consumer b0ac27c7920
verdict: nothing found
cases: executed 11→13, product red 0 (one test setup refusal corrected)
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned consumer scratch and target, full inventory retained privately
needs-coordinator: record review, integrate tests, regenerate conformance and run combined gate

1. Test-only diff

 crates/ekr-views/tests/adversary_quality.rs | 84 +++++++++++++++++++++++++++++
 1 file changed, 84 insertions(+)

Read the complete Q source/specification/documentation delta, acceptance, original tests and projection/SDK/CLI callers before adding cases. The implementor report supplies before counts: quality7 and adversary_quality4. Only this test file is changed by this review; the concurrent OCEL implementor's separate SDK fix was committed separately and is outside this review.

2. First cases

Both cases were written before any execution. Retraction of an assertion holding two seed attachments removes exactly one active/seed/item count; historical bytes remain unchanged through provider reopen. The equal-payload case adds a new evidence identity with seed-identical bytes and verifies that it counts as item evidence until an actual seed attachment is committed. Both exercise File and SQLite via admitted transactions.

The second case's first execution reached the new identity assertions successfully, then its attachment was refused as evidence-set-mismatch: the reused test writer collected AddAssertion evidence but omitted AttachEvidence evidence. This is a test setup defect, not a product finding. Extended the writer's operation match to include attachment evidence, preserving all prior test assertions; reran that case alone before the suites. No implementation changed and the first output is retained below.

Log: q-review-first-attached_seed_counts_follow_retraction_without_rewriting_historical_counts; exit 0

```text
   Compiling ekr-views v0.0.26 (<consumer-tree>/crates/ekr-views)
    Finished `test` profile [unoptimized] target(s) in 2.32s
     Running tests/adversary_quality.rs (<cache>/b10x-target/ekr-x7b-consumer/debug/deps/adversary_quality-801452c3e45b445a)

running 1 test
test attached_seed_counts_follow_retraction_without_rewriting_historical_counts ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.31s


```

Log: q-review-first-seed_counts_classify_evidence_identity_even_when_payload_hashes_are_equal; exit 101

```text
    Finished `test` profile [unoptimized] target(s) in 0.10s
     Running tests/adversary_quality.rs (<cache>/b10x-target/ekr-x7b-consumer/debug/deps/adversary_quality-801452c3e45b445a)

running 1 test

thread 'seed_counts_classify_evidence_identity_even_when_payload_hashes_are_equal' (773458) panicked at crates/ekr-views/tests/adversary_quality.rs:242:9:
Rejected(RejectionRecordV1 { format: "ekr.rejection-record/1", event_id: EventId(2165115634653052243987032747712062738), proposed_event_id: EventId(2165115634643372669837760993302031574), proposal_record_hash: ContentHash([125, 44, 4, 98, 175, 49, 118, 93, 1, 188, 60, 19, 184, 201, 205, 60, 112, 236, 121, 84, 88, 208, 120, 204, 107, 161, 130, 31, 77, 150, 152, 174]), requested_basis: ValidationBasisV1 { format: "ekr.validation-basis/1", graph_root_id: GraphRootId(302240678275694148452354), previous_revision_id: RevisionId(2165115634626466446760453611552170757), previous_event_id: EventId(2165115634626466446760453607697647400), previous_record_hash: ContentHash([90, 243, 160, 136, 152, 6, 74, 139, 200, 35, 234, 213, 45, 136, 21, 95, 231, 23, 135, 31, 150, 187, 75, 105, 197, 1, 177, 132, 172, 13, 59, 196]), previous_root: Root { revision: RevisionNumber(2), parent: Some(ContentHash([128, 98, 203, 165, 229, 121, 21, 58, 121, 4, 19, 202, 56, 148, 249, 109, 14, 125, 142, 110, 203, 108, 161, 205, 150, 10, 120, 27, 53, 1, 110, 251])), ontology_root: ContentHash([209, 188, 139, 2, 209, 97, 116, 250, 163, 241, 144, 7, 92, 89, 39, 121, 79, 117, 101, 62, 64, 83, 123, 57, 38, 19, 244, 150, 14, 125, 45, 81]), knowledge_root: ContentHash([129, 62, 160, 31, 193, 124, 131, 123, 106, 137, 114, 12, 228, 157, 112, 139, 224, 245, 43, 140, 42, 22, 69, 127, 51, 184, 82, 175, 105, 87, 175, 245]), evidence_root: ContentHash([174, 92, 171, 1, 94, 129, 254, 178, 179, 239, 20, 188, 133, 22, 244, 19, 176, 76, 95, 34, 232, 120, 54, 218, 160, 112, 1, 90, 93, 94, 109, 93]), agent_root: ContentHash([205, 79, 202, 150, 208, 140, 255, 209, 237, 148, 179, 112, 203, 238, 49, 55, 93, 229, 172, 221, 22, 205, 250, 65, 192, 187, 127, 212, 157, 130, 136, 23]), transaction: ContentHash([71, 129, 225, 25, 34, 247, 177, 198, 73, 63, 3, 216, 205, 255, 160, 42, 184, 80, 147, 80, 48, 15, 248, 211, 232, 54, 146, 208, 144, 127, 90, 160]) }, previous_root_hash: ContentHash([254, 142, 181, 25, 56, 1, 132, 38, 91, 170, 114, 45, 140, 169, 36, 149, 150, 160, 141, 75, 7, 148, 129, 214, 66, 158, 123, 117, 70, 17, 82, 132]), seed_hash: ContentHash([88, 80, 173, 196, 181, 161, 130, 195, 94, 37, 145, 229, 126, 195, 43, 105, 65, 13, 48, 219, 114, 231, 214, 209, 90, 174, 35, 126, 193, 174, 185, 221]), ontology_root: ContentHash([209, 188, 139, 2, 209, 97, 116, 250, 163, 241, 144, 7, 92, 89, 39, 121, 79, 117, 101, 62, 64, 83, 123, 57, 38, 19, 244, 150, 14, 125, 45, 81]), authority_root: ContentHash([205, 79, 202, 150, 208, 140, 255, 209, 237, 148, 179, 112, 203, 238, 49, 55, 93, 229, 172, 221, 22, 205, 250, 65, 192, 187, 127, 212, 157, 130, 136, 23]), validation_profile_hash: ContentHash([201, 211, 163, 92, 83, 204, 128, 241, 147, 95, 170, 85, 242, 7, 128, 20, 37, 44, 22, 155, 8, 153, 114, 218, 236, 58, 61, 82, 235, 91, 82, 172]) }, validator: AgentId(302240678275694148452356), rejected_at: Timestamp(1800000100005), issues: [RecordedValidationIssue { id: IssueId(2165115634653052243987032751904648366), transaction_id: TransactionId(302240678275694159791372), validator: Structural, code: "evidence-set-mismatch", message: "the transaction declares it rests on [] and its assertions cite or its attachments attach [\"00000000-0000-4000-8000-000000ad0300\"]; the declared set is what `evidence_hash` addresses and is held to the operations" }] })
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test seed_counts_classify_evidence_identity_even_when_payload_hashes_are_equal ... FAILED

failures:

failures:
    seed_counts_classify_evidence_identity_even_when_payload_hashes_are_equal

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.08s

error: test failed, to rerun pass `-p ekr-views --test adversary_quality`

```

Log: q-review-staging-corrected; exit 0

```text
   Compiling ekr-views v0.0.26 (<consumer-tree>/crates/ekr-views)
    Finished `test` profile [unoptimized] target(s) in 3.11s
     Running tests/adversary_quality.rs (<cache>/b10x-target/ekr-x7b-consumer/debug/deps/adversary_quality-801452c3e45b445a)

running 1 test
test seed_counts_classify_evidence_identity_even_when_payload_hashes_are_equal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.27s


```

3. Subsequent suites

Command: cargo test --locked -p ekr-views --test adversary_quality --test quality
Exit: 0. Run after both new cases executed alone.

```text
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `test` profile [unoptimized] target(s) in 1.16s
     Running tests/adversary_quality.rs (<cache>/b10x-target/ekr-x7b-consumer/debug/deps/adversary_quality-801452c3e45b445a)

running 6 tests
test a_share_that_does_not_divide_is_rounded_down_and_an_inherited_property_counts_at_its_declarer ... ok
test attached_seed_counts_follow_retraction_without_rewriting_historical_counts ... ok
test a_whole_of_zero_omits_its_share ... ok
test one_name_two_thousand_nodes_share_is_one_entry_listing_every_node_by_id ... ok
test seed_counts_classify_evidence_identity_even_when_payload_hashes_are_equal ... ok
test the_property_and_assertion_figures_follow_each_revisions_schema_and_lifecycle ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.27s

     Running tests/quality.rs (<cache>/b10x-target/ekr-x7b-consumer/debug/deps/quality-6a5d8237e330f39f)

running 7 tests
test an_unseeded_store_and_a_revision_beyond_the_head_are_refused_by_name ... ok
test every_figure_of_every_revision_equals_the_fixtures_count_on_both_providers ... ok
test attached_evidence_counts_in_both_evidence_figures ... ok
test the_document_of_revision_zero_is_exactly_the_formats_bytes ... ok
test the_pure_half_answers_the_reads_bytes_from_the_loaded_revision_and_the_seeds_evidence ... ok
test retained_seed_attachments_count_once_and_declaring_types_do_not_count_inheritance ... ok
test two_reads_of_one_revision_are_byte_identical_on_both_providers_and_after_a_later_commit ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.24s

```

4. Findings

Nothing found.

Owners: 0 findings, 0 coordinator, 0 implementor.

5. Attacks without a finding

- Active counts change once per retracted attached assertion; immutable historical bytes survive reopen.
- Evidence identity determines seed membership even when retained payload hashes coincide; a later seed attachment then changes exactly one count.
- Existing explicit owner/inheritance, empty-population, rounding and revision cases remain green.

6. Outside-tree writes

Raw first-run, corrected-run, suite and clippy logs and exit markers, this report and commit evidence remain under <cache>/ekr-extract-07b/consumer with prefix q-review. The assigned target is <cache>/b10x-target/ekr-x7b-consumer. No push, cleanup or planning mutation occurred in reviewer role. Root owns integration and the full gate.

```findings
[]
```
