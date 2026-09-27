---
format: aep.planning-md/1
id: review-result:p1-persisted-contract-amendment-review
kind: review-result
status: active
title: Independent review of proposed assertion and persisted-format amendments
relations:
- reviews: story:version-persisted-contracts
- reviews: story:commit-and-revision-lineage
revision: 1
---
# Persisted-contract amendment review

Reviewed the supplied amendment draft §§88–90 against integration `4032d002ecd2b08519910398ae6ed3b9ec8ea0f6`, the original design, persisted-contract story and scope report, and writer acceptance. Read-only design review: no builds, runtime probes, source edits, ESS edits or AEP writes. Citations to `amendment-draft.md` refer to the supplied draft before coordinator corrections.

Owners: the subject-equality contradiction is coordinator-introduced; the remaining items are pre-existing unresolved contract decisions. No implementation finding is attributed to the seed worker.

## Corrections required before persisted-contract dispatch

1. **The replacement equality rule prevents the required example.** Draft `amendment-draft.md:24–27` requires the same subject and predicate. Design `docs/epistemic-knowledge-runtime-design.md:2595–2599` replaces `Alice CEO_OF Acme` with `Bob CEO_OF Acme`; the subjects differ. Writer acceptance `.engineering/planning/story/commit-and-revision-lineage.md:41` requires that exact example. **Origin: draft-introduced; blocker.** The coordinator independently verified this contradiction and selected removal of subject/predicate equality, retaining explicit replacement identity and ordinary ontology/provenance/cardinality checks. Preserve a regression using different subjects when implementing the amendment.

2. **The enclosing graph and seed versions remain intentionally undecided.** Draft `amendment-draft.md:67–70` postpones the nesting, while `.engineering/planning/story/version-persisted-contracts.md:57` requires exact shapes before source work. Scope report `.engineering/waves/p1-persisted-contract-scope.md:94–96` also requires distinct legacy decoding. The reported seed interfaces are `ekr-seed/1` and persisted `ekr-seed-envelope/1 {input, context:{operator,validator}}`; putting changed assertion bytes under either `/1` changes their meaning. **Origin: pre-existing unresolved coordination; dispatch blocker.** Record an exact compatibility table: old unversioned GraphDocument, seed/1 and seed-envelope/1 remain legacy-only; the new graph document and both enclosing seed formats get explicit new versions, with the exact location of the graph envelope fixed. Unknown/mismatched combinations must refuse, including a new-format graph hidden under an old seed tag.

3. **The read rule does not settle transaction-time writes and eligibility.** Draft `amendment-draft.md:21–33` establishes latest-revision historical valid-time reads but does not say whether supersession closes `transaction_time.recorded_to`, or whether that field still gates those reads. Existing `crates/ekr-graph/src/assertion.rs:278–283,317–325,639–640` calls it the end of the runtime's belief; `crates/ekr-graph/src/snapshot.rs:75–79` filters on `is_current`, which requires that interval to remain open. The original design distinguishes valid and transaction time at `docs/epistemic-knowledge-runtime-design.md:788–809`. **Origin: pre-existing semantic gap; decision required with the new query contract.** Specify the timestamp source and changes to both assertions on retraction/supersession, and write the exact predicate for latest-revision valid-time reads. In particular, closing transaction time on supersession must not accidentally suppress the earlier valid interval. Do not leave `is_current` to determine the answer by its old implementation.

## Decision required before writer dispatch

4. **Replacement acceptance needs an explicit evaluation point.** Draft `amendment-draft.md:17–18` requires fresh assertions to arrive Proposed, but `:24` requires the replacement to be Accepted. These are compatible only if replacement acceptance is checked against the validated candidate, or if two separate commits are deliberately required. Writer acceptance `commit-and-revision-lineage.md:41` does not choose. **Origin: pre-existing operation-semantics gap; writer blocker, not a demand to implement the writer in this format unit.** Allow AddAssertion plus explicit supersession in one atomic transaction, evaluating acceptance after validation; require the replacement to survive the resulting state and reject retraction or conflicting supersessions of that replacement in the same unordered operation set. Record this decision now so the new lifecycle shape is not tested solely against already-accepted fixture replacements.

## What does not need reopening

Separating assessment from lifecycle resolves the original §§17/36 overlap and preserves acceptance attribution. Distinguishing withdrawal from a supported historical fact resolves §65 without deleting old revisions. Per-occurrence identity on all revision facts addresses the repeated proposal/validation class, not just rejection. Preservation-first migration correctly refuses missing payloads or attribution instead of manufacturing history. Full-root validation basis may remain assigned to the durable writer, provided its receipt format is fixed before any such receipt is published. No additional future subsystem is required by this review.

## Durable finding inventory

Owners: 4 findings, 4 coordinator contract decisions, 0 seed implementor. The draft contradiction is introduced; the other decisions predate the draft.

The corrected draft is retained in .engineering/waves/p1-persisted-contract-amendment-draft.md.
The reviewed clauses and their citations above preserve the original findings; the amended draft
is a proposal for the next wave, not a claim of implemented behavior.

```findings
- file: .engineering/waves/p1-persisted-contract-amendment-draft.md
  line: 24
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The original draft required replacement subject equality, contradicting the different Alice/Bob subjects in design section 65. Coordinator verified and removed it."
- file: .engineering/waves/p1-persisted-contract-amendment-draft.md
  line: 67
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Original draft left graph and enclosing seed format nesting undecided. Record an exact compatibility table before dispatch; never reinterpret new assertions under old seed tags."
- file: crates/ekr-graph/src/assertion.rs
  line: 278
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Settle transaction-time closure, timestamp source and exact valid-time eligibility; the old is_current predicate suppresses every superseded assertion."
- file: .engineering/planning/story/commit-and-revision-lineage.md
  line: 41
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "Before writer dispatch, define replacement acceptance against the validated candidate so AddAssertion and supersession may be atomic without admitting caller-supplied Accepted state."
```
