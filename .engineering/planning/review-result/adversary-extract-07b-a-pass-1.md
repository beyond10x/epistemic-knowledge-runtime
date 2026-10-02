---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-a-pass-1
kind: review-result
status: active
title: Attachment decoding adversary, first pass
relations:
- reviews: story:evidence-attaches-to-a-held-assertion
revision: 1
---
unit: story:evidence-attaches-to-a-held-assertion at 57a304dab7cfdcdbe60b6293fcdadb9c01f1b424 plus test-only working changes
verdict: NEEDS-CHANGE
cases: executed 3→6, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned A scratch and build directories
needs-coordinator: return decoder finding to implementor

1. Diff proof

```text
 crates/ekr-kernel/tests/attach_evidence.rs | 145 +++++++++++++++++++++++++++++
 1 file changed, 145 insertions(+)
```

No implementation file changed in this attack. Before count is the implementor's three ordinary attachment cases; after count is the six executed cases below. The expensive ignored measurement stays ignored in this default target.

2. First execution of each added case

All cases were written before any test command. The first case builds a real committed graph, checks its document round trip, duplicates exactly one attachment in its JSON, then requires a decoding refusal. It fails because the public graph decoder silently deduplicates the repeated record. The two other additions drive the design's peer-evidence exclusion through checkpoint and full replay, including historical reads, and a validated attachment losing to a later committed retraction. Both pass on File and SQLite.

Command: `cargo test --locked -p ekr-kernel --test attach_evidence adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder -- --exact --nocapture`; exit 101.

```text
   Compiling ekr-graph v0.0.26 (<worktree-A>/crates/ekr-graph)
   Compiling ekr-store v0.0.26 (<worktree-A>/crates/ekr-store)
   Compiling ekr-kernel v0.0.26 (<worktree-A>/crates/ekr-kernel)
    Finished `test` profile [unoptimized] target(s) in 12.66s
     Running tests/attach_evidence.rs (<cache>/b10x-target/ekr-x7b-a/debug/deps/attach_evidence-9fda2e584ba7320e)

running 1 test

thread 'adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder' (4137976) panicked at crates/ekr-kernel/tests/attach_evidence.rs:703:10:
duplicate attachment must not silently become one record: GraphDocument { root: GraphRoot { id: GraphRootId(302240678275694148452354), space: Canonical, schema_version_id: SchemaVersionId(302240678275694148452353), parent: None, created_at: Timestamp(0) }, revision: RevisionNumber(2), nodes: {NodeId(302240678275694148453121): Node { id: NodeId(302240678275694148453121), root_id: GraphRootId(302240678275694148452354), type_id: TypeId(302240678275694148452865), canonical_name: "Alice", aliases: [], type_state: None, properties: {} }, NodeId(302240678275694148453122): Node { id: NodeId(302240678275694148453122), root_id: GraphRootId(302240678275694148452354), type_id: TypeId(302240678275694148452865), canonical_name: "Bob", aliases: [], type_state: None, properties: {} }, NodeId(302240678275694148453123): Node { id: NodeId(302240678275694148453123), root_id: GraphRootId(302240678275694148452354), type_id: TypeId(302240678275694148452866), canonical_name: "Acme", aliases: [], type_state: None, properties: {} }}, edges: {}, assertions: {AssertionId(2165112674508708797500734542884492636): Assertion { id: AssertionId(2165112674508708797500734542884492636), root_id: GraphRootId(302240678275694148452354), subject: Node(NodeId(302240678275694148453121)), predicate: Relation(TypeId(302240678275694148452867)), object: Node(NodeId(302240678275694148453123)), evidence: {EvidenceId(302240678275694148453377)}, proposed_by: AgentId(302240678275694148452609), assessment: Accepted { validators: {AgentId(302240678275694148452610)} }, lifecycle: Active, valid_time: TemporalRange { from: Some(Timestamp(100)), to: None }, transaction_time: TransactionTime { recorded_from: Timestamp(22), recorded_to: None } }}, evidence: {EvidenceId(302240678275694148453377): Evidence { id: EvidenceId(302240678275694148453377), source: HumanStatement { identity: Some("Runtime operator") }, content_hash: ContentHash([47, 149, 79, 143, 119, 115, 30, 17, 164, 220, 162, 30, 11, 182, 197, 102, 247, 25, 170, 228, 17, 104, 56, 243, 9, 95, 96, 100, 158, 84, 41, 219]), extracted_by: AgentId(302240678275694148452609), observed_at: Timestamp(1773273600000), confidence: Confidence(10000) }, EvidenceId(302240678275694148453378): Evidence { id: EvidenceId(302240678275694148453378), source: HumanStatement { identity: Some("Runtime operator") }, content_hash: ContentHash([189, 29, 77, 201, 186, 80, 18, 223, 94, 67, 80, 84, 24, 170, 119, 40, 193, 91, 202, 5, 42, 26, 212, 59, 26, 45, 0, 208, 75, 117, 189, 214]), extracted_by: AgentId(302240678275694148452609), observed_at: Timestamp(1773273600000), confidence: Confidence(10000) }}, attachments: {AssertionId(2165112674508708797500734542884492636): {AttachedEvidence { evidence: CanonicalRef { id: EvidenceId(302240678275694148453378) }, revision: RevisionNumber(2) }}} }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder ... FAILED

failures:

failures:
    adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.03s

error: test failed, to rerun pass `-p ekr-kernel --test attach_evidence`
```

Command: the same target with exact filter `adversary_checkpoint_explanations_do_not_import_peer_attachments`; exit 0.

```text
    Finished `test` profile [unoptimized] target(s) in 0.11s
     Running tests/attach_evidence.rs (<cache>/b10x-target/ekr-x7b-a/debug/deps/attach_evidence-9fda2e584ba7320e)

running 1 test
test adversary_checkpoint_explanations_do_not_import_peer_attachments ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.19s

```

Command: the same target with exact filter `adversary_a_validated_attachment_loses_to_a_committed_retraction`; exit 0.

```text
    Finished `test` profile [unoptimized] target(s) in 0.18s
     Running tests/attach_evidence.rs (<cache>/b10x-target/ekr-x7b-a/debug/deps/attach_evidence-9fda2e584ba7320e)

running 1 test
test adversary_a_validated_attachment_loses_to_a_committed_retraction ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.22s

```

3. Target suite after the first-case runs

Command: `cargo test --locked -p ekr-kernel --test attach_evidence -- --nocapture`; exit 101.

```text
    Finished `test` profile [unoptimized] target(s) in 0.14s
     Running tests/attach_evidence.rs (<cache>/b10x-target/ekr-x7b-a/debug/deps/attach_evidence-9fda2e584ba7320e)

running 7 tests
test a_thousand_attachments_against_seventy_thousand_assertions_commit ... ignored, builds a 70,000-assertion store; run by hand to record the time

thread 'adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder' (4141676) panicked at crates/ekr-kernel/tests/attach_evidence.rs:703:10:
duplicate attachment must not silently become one record: GraphDocument { root: GraphRoot { id: GraphRootId(302240678275694148452354), space: Canonical, schema_version_id: SchemaVersionId(302240678275694148452353), parent: None, created_at: Timestamp(0) }, revision: RevisionNumber(2), nodes: {NodeId(302240678275694148453121): Node { id: NodeId(302240678275694148453121), root_id: GraphRootId(302240678275694148452354), type_id: TypeId(302240678275694148452865), canonical_name: "Alice", aliases: [], type_state: None, properties: {} }, NodeId(302240678275694148453122): Node { id: NodeId(302240678275694148453122), root_id: GraphRootId(302240678275694148452354), type_id: TypeId(302240678275694148452865), canonical_name: "Bob", aliases: [], type_state: None, properties: {} }, NodeId(302240678275694148453123): Node { id: NodeId(302240678275694148453123), root_id: GraphRootId(302240678275694148452354), type_id: TypeId(302240678275694148452866), canonical_name: "Acme", aliases: [], type_state: None, properties: {} }}, edges: {}, assertions: {AssertionId(2165112710681009417079041544238166176): Assertion { id: AssertionId(2165112710681009417079041544238166176), root_id: GraphRootId(302240678275694148452354), subject: Node(NodeId(302240678275694148453121)), predicate: Relation(TypeId(302240678275694148452867)), object: Node(NodeId(302240678275694148453123)), evidence: {EvidenceId(302240678275694148453377)}, proposed_by: AgentId(302240678275694148452609), assessment: Accepted { validators: {AgentId(302240678275694148452610)} }, lifecycle: Active, valid_time: TemporalRange { from: Some(Timestamp(100)), to: None }, transaction_time: TransactionTime { recorded_from: Timestamp(22), recorded_to: None } }}, evidence: {EvidenceId(302240678275694148453377): Evidence { id: EvidenceId(302240678275694148453377), source: HumanStatement { identity: Some("Runtime operator") }, content_hash: ContentHash([47, 149, 79, 143, 119, 115, 30, 17, 164, 220, 162, 30, 11, 182, 197, 102, 247, 25, 170, 228, 17, 104, 56, 243, 9, 95, 96, 100, 158, 84, 41, 219]), extracted_by: AgentId(302240678275694148452609), observed_at: Timestamp(1773273600000), confidence: Confidence(10000) }, EvidenceId(302240678275694148453378): Evidence { id: EvidenceId(302240678275694148453378), source: HumanStatement { identity: Some("Runtime operator") }, content_hash: ContentHash([189, 29, 77, 201, 186, 80, 18, 223, 94, 67, 80, 84, 24, 170, 119, 40, 193, 91, 202, 5, 42, 26, 212, 59, 26, 45, 0, 208, 75, 117, 189, 214]), extracted_by: AgentId(302240678275694148452609), observed_at: Timestamp(1773273600000), confidence: Confidence(10000) }}, attachments: {AssertionId(2165112710681009417079041544238166176): {AttachedEvidence { evidence: CanonicalRef { id: EvidenceId(302240678275694148453378) }, revision: RevisionNumber(2) }}} }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder ... FAILED
test adversary_a_validated_attachment_loses_to_a_committed_retraction ... ok
test a_supersession_leaves_attachments_with_the_superseded_assertion ... ok
test adversary_checkpoint_explanations_do_not_import_peer_attachments ... ok
test evidence_added_and_attached_in_one_transaction_commits_and_explain_lists_it_from_then_on ... ok
test each_attachment_refusal_is_named_and_moves_nothing ... ok

failures:

failures:
    adversary_duplicate_attachment_members_are_refused_by_the_graph_decoder

test result: FAILED. 5 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.11s

error: test failed, to rerun pass `-p ekr-kernel --test attach_evidence`
```

4. Finding

| file:line | verdict | origin | measured | reaches it |
|---|---|---|---|---|
| crates/ekr-store/src/snapshot.rs:68 | NEEDS-CHANGE | introduced | attachment test line 703 fails after a successful unique-document control; the repeated item decodes to one set entry, exit 101 | public GraphDocument::from_bytes; the same graph carrier is decoded in checkpoint restore; the test mutates a real committed snapshot, not canonical state |

The map decoder checks assertion-key uniqueness but its BTreeSet values use permissive serde decoding. Decode each attachment set with the existing strict set primitive. The normal exporter does not emit duplicates, and this finding makes no claim that malformed input can create unauthorized canonical knowledge; it concerns the typed input decoder discarding repeated data. The new field did not exist at the base.

Owners: 1 finding, 0 coordinator, 1 implementor.

5. Attacks without a finding

- A transaction attaching evidence to two assertions does not leak the peer's evidence into either explanation.
- Attachments survive a retained checkpoint and full replay; a read of the earlier revision contains none.
- A validated attachment becomes Stale after retraction wins, with the head and attachment collection unchanged.

6. Outside-tree writes

Assigned scratch: <cache>/ekr-extract-07b/a/adversary-additions.rs; adversary-duplicate-first.log and .exit; adversary_checkpoint_explanations_do_not_import_peer_attachments.log and .exit; adversary_a_validated_attachment_loses_to_a_committed_retraction.log and .exit; adversary-suite-first.log and .exit; adversary-pass-1.raw.md; adversary-pass-1.md. Build: <cache>/b10x-target/ekr-x7b-a. Public log copies replace machine-root paths with the declared aliases; private originals remain intact.

```findings
- file: crates/ekr-store/src/snapshot.rs
  line: 68
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The attachment map silently collapses repeated attachment records because its set values do not use strict duplicate decoding.
```
