---
format: aep.planning-md/1
id: review-result:p1-12-replay-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-12 replay unit
relations:
- reviews: task:replay-refusals-have-no-forged-history-case
- reviews: task:validation-hash-uses-payload-domain
revision: 1
---
Adversary pass 1 on unit p1-12-replay (wave p1-12), `aep-drive:adversary`, against the uncommitted
tree on top of `e6af002`.

Owners: 3 findings, 3 implementor.

Verdict: NEEDS-CHANGE. cases: executed 220→224, red 2. origin: introduced 1 / pre-existing 2.

Cases added in `crates/ekr-kernel/tests/adversary_p1_12_replay_consistent_lie.rs`: a commit whose
receipt and payload both claim a skipped revision (red: `Document("commit-result-disagrees")`
instead of `RevisionOutOfOrder`); both publish an unreached knowledge root (red: same code instead
of `KnowledgeRootDisagrees`); a commit of a never-validated proposal (green); commands over a
forged lineage refuse and write nothing (green).

Suggested fix: compare the receipt's claimed revision and knowledge root with the recomputed root
before the full `record.result == root` check at `replay.rs:403`.

Attacked without a break: seal hash completeness, determinism and domain; removed variants
unreachable at base; reordered occurrences; provider bytes on read and command paths.

```findings
- file: crates/ekr-kernel/src/replay.rs
  line: 403
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: a commit whose receipt and payload both claim a revision that skips its parent is refused as Document("commit-result-disagrees"), because the full-result check runs before RevisionOutOfOrder, so the named refusal only fires for a payload that contradicts its own receipt
- file: crates/ekr-kernel/src/replay.rs
  line: 403
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: KnowledgeRootDisagrees is documented as the commit's claim disagreeing with the fold, but a commit whose receipt and payload both publish an unreached root gets the generic commit-result-disagrees code
- file: crates/ekr-kernel/src/replay.rs
  line: 366
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the Proposed arm of the new ValidationMissing guard was reached by no kernel test before this pass, so dropping it would have left the suite green
```
