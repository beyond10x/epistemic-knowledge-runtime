---
format: aep.planning-md/3
id: review-result:schema-application-design-independent-r1
kind: review-result
status: active
title: Independent application publication design review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 3 coordinator/design findings; 0 delegated implementor findings. Two findings are blockers and one is a compatibility warning. This is a bounded design review, not runtime approval.

Reviewed .engineering/reviews/knowledge-schema-application-design/minimal-contract-proposal.md against current store preparation/review retention and kernel transaction/review authority. No Cargo, tests, source changes or upstream actions were performed. All reproduction traces below are source-derived design cases, not executed F behavior; F is not implemented.

1. Blocker — frozen transaction identity cannot satisfy the promised revalidation after unrelated advancement. The proposal fixes one transaction per step at lines 52–59, reuses ordinary Propose/Validate/Commit recovery at lines 101–104, and promises revalidation after an unrelated commit at lines 149–150. Current validate requires TransactionState::Proposed (crates/ekr-kernel/src/commands.rs:340), while commit after basis advancement publishes a terminal Stale result; TransactionRecord::state preserves that terminal state (crates/ekr-kernel/src/replay.rs:53). Revalidating the same step transaction is therefore refused. Even before a validation publishes, changing its against revision retains the same command key but changes the input hash, which conflicts with the elected immutable preparation.

   Trace: elect step T, propose and validate T at revision r, let unrelated transaction U commit, then commit T. T becomes stale and the frozen step cannot continue using ordinary validation. Specify either a stable semantic step with separately elected successor transaction attempts, preserving logical assertion/evidence identities and resolving every uncertain predecessor before replacement, or an explicit versioned application-only lifecycle/command-key extension. Preserve legacy ordinary stale semantics. Add the exact interleaving, including a crash around stale publication, to required probes. A successor publication preparation alone does not change the retained transaction lifecycle or the Validate input binding.

2. Blocker — renewed approval after partial progress has no compatible E admission rule. Lines 131–140 require F to recognize its own committed prefix and allow a new approval of the same immutable proposal after rejection. Current E approval still validates every original proposal correction against the current graph (crates/ekr-kernel/src/schema_proposal_review.rs:251 and its historical-material check); schema_proposal_corrections::validate requires distinct active claims in current disputes. The proposed step model does not restrict corrections to a final step with no remaining work.

   Trace allowed by this design: an approved proposal has a correction and mapping items; its correction step commits while a mapping remains unresolved; the operator rejects continuation; later the mapping becomes resolvable and the operator signs a new approval of the same proposal. The original correction now names an inactive or settled claim, so existing E admission refuses rather than allowing continuation. Define application-aware review material and correction applicability for verified completed versus remaining steps in both E admission and historical review replay, or constrain the protocol's sequencing and continuation semantics so this trace is impossible. State how a new signature binds the residual plan and how external coincidental changes remain distinguishable from this election's own commits. Merely changing F's initial preview comparison leaves the E approval entry point blocked.

3. Warning — the preparation-format baseline is stale. Lines 95–98 allocate the next format after /5 and preserve only /1–/5, while crates/ekr-store/src/preparation.rs:244 already defines /6 for signed publications with the mandatory shared human-decision append. Lines 117–118 likewise claim every non-object extra stream is rejected, but /6 explicitly authenticates the shared decision singleton. Allocate a distinct next format after /6, preserve /6 bytes/authorization, and state the closed composition rule for the application marker and any existing mandatory identity append. Include old /6 preparation recovery as a compatibility control.

Ordering follow-up assessed at coordinator request: strict schema-then-all-selected-mappings-then-one-corrections-transaction ordering removes the specific completed-correction reapproval trace, provided unresolved selected mappings cannot be silently dropped and no canonical work remains after corrections. Completion must be derived from actual linked commits even when the final reporting receipt was not written before a crash; exact recovery of those commits may then succeed after rejection. Application-aware reapproval after partial mappings is still required: rejection must block every future canonical publication, and a fresh signature must bind verified own-prefix references and residual evidence/effects. The revised proposal text implementing these constraints has not been reviewed here; the findings above describe the inspected candidate.

The shared per-proposal native CAS, physical cursor separate from human predecessor, nonempty marker in the same atomic publication group, and exact already-published receipt recovery are sound directions. The design also explicitly requires ordinary generic commit/revalidation to retain application linkage, so I found no additional supported ordinary-commit escape beyond the need to implement and test that requirement. This assessment does not establish implementation correctness, provider atomicity execution, cryptographic admission or a full gate.

```findings
[
  {
    "file": ".engineering/reviews/knowledge-schema-application-design/minimal-contract-proposal.md",
    "line": 56,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "The design freezes one ordinary transaction per step but promises revalidation after unrelated canonical advancement. Existing Commit publishes terminal Stale, Validate requires Proposed, and an already elected Validate command binds its requested revision in the input hash. Specify separately elected successor transaction attempts or an explicit scoped lifecycle/key extension; cover validate-T, unrelated-commit-U, commit-T and crash/retry without changing legacy stale semantics. This is a source-derived design incompatibility, not executed F behavior."
  },
  {
    "file": ".engineering/reviews/knowledge-schema-application-design/minimal-contract-proposal.md",
    "line": 138,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Continuation under a new approval after partial corrections is not compatible with current E admission, which validates every original correction against current active dispute claims. The proposed protocol permits corrections to commit while mapping work remains, then rejection; a later approval of the same immutable proposal is refused on the already-applied correction. Define application-aware residual review material/applicability in E admission and historical replay, or sequencing rules that exclude this trace, while distinguishing this election's own commits from external changes. This is a latent design path, not an observed runtime exploit."
  },
  {
    "file": ".engineering/reviews/knowledge-schema-application-design/minimal-contract-proposal.md",
    "line": 96,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "The design allocates the next preparation format after /5 although /6 already authenticates shared human-decision identity appends. Use a distinct next format, preserve /6 serialization and recovery, and update the extra-stream rule to describe the closed composition with the existing identity append rather than claiming every non-object append is currently rejected."
  }
]
```
