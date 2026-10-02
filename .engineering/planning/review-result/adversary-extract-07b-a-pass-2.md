---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-a-pass-2
kind: review-result
status: active
title: Attachment decoder correction, second adversary pass
relations:
- reviews: story:evidence-attaches-to-a-held-assertion
revision: 1
---
unit: story:evidence-attaches-to-a-held-assertion at bf48f7060fc067635f440c2bcb0d1fc273545f8a plus test-only addition
verdict: nothing found
cases: executed 6→7, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned A scratch and build directories
needs-coordinator: none

1. Diff proof

```text
 crates/ekr-kernel/tests/attach_evidence.rs | 40 ++++++++++++++++++++++++++++++
 1 file changed, 40 insertions(+)
```

2. Case before suite

The correction uses a transparent nested unique-set decoder inside the unique-key map. The new case attacks decoded identity rather than identical JSON text: it writes a real attachment once normally and once with an escaped UUID separator, proves those records parse to the same JSON value, and requires duplicate decoded set member. Its unique-document control round trips first. Source and existing tests were unchanged.

`cargo test --locked -p ekr-kernel --test attach_evidence adversary_equivalent_attachment_spellings_are_duplicate_records -- --exact --nocapture`, exit 0:

```text
   Compiling ekr-kernel v0.0.26 (<worktree-A>/crates/ekr-kernel)
    Finished `test` profile [unoptimized] target(s) in 0.92s
     Running tests/attach_evidence.rs (<cache>/b10x-target/ekr-x7b-a/debug/deps/attach_evidence-9fda2e584ba7320e)

running 1 test
test adversary_equivalent_attachment_spellings_are_duplicate_records ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.04s

```

3. Subsequent target suite

`cargo test --locked -p ekr-kernel --test attach_evidence -- --nocapture`, exit 0:

```text
    Finished `test` profile [unoptimized] target(s) in 0.08s
     Running tests/attach_evidence.rs (<cache>/b10x-target/ekr-x7b-a/debug/deps/attach_evidence-9fda2e584ba7320e)

running 8 tests
test a_thousand_attachments_against_seventy_thousand_assertions_commit ... ignored, builds a 70,000-assertion store; run by hand to record the time
test adversary_equivalent_attachment_spellings_are_duplicate_records ... ok
test adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder ... ok
test adversary_a_validated_attachment_loses_to_a_committed_retraction ... ok
test a_supersession_leaves_attachments_with_the_superseded_assertion ... ok
test adversary_checkpoint_explanations_do_not_import_peer_attachments ... ok
test evidence_added_and_attached_in_one_transaction_commits_and_explain_lists_it_from_then_on ... ok
test each_attachment_refusal_is_named_and_moves_nothing ... ok

test result: ok. 7 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.51s

```

4. Findings

Nothing found. The first pass's identical-record case now passes, and equivalent decoded spellings are refused as well.

Owners: 0 findings, 0 coordinator, 0 implementor.

5. Attacks without a finding

- Byte-distinct JSON spellings of one attachment cannot bypass duplicate-set decoding.
- The first pass's unique-document control, checkpoint isolation and stale commit cases remain green.

6. Outside-tree writes

<cache>/ekr-extract-07b/a/adversary-equivalent-first.log and .exit; adversary-suite-second.log and .exit; adversary-pass-2.raw.md; adversary-pass-2.md. Build output: <cache>/b10x-target/ekr-x7b-a. Public logs replace machine-root paths with aliases; originals remain intact.

```findings
[]
```
