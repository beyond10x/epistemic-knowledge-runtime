# Transaction-document parser preparation

This unit is part of the approved completion plan. Owner:
task:bounded-transaction-document-parser, derived from
story:commit-and-revision-lineage. Root owns all planning, shared DESIGN/ESS,
integration and publication. One implementor, then an independent adversary.

Read AGENTS.md, .engineering/waves/COORDINATOR.md,
.engineering/waves/p1-durable-record-decisions.md (exact document and frozen
profile sections), and .engineering/reviews/p1-transaction-parser-readiness.md.
Implement the bounded parser and its tests under that exact agreed contract.
No new format activation or store mutation is part of this preparation.

## Assigned resources

- Managed id: ekr-transaction-parser-20260922.
- Worktree: <worktrees>/ekr-transaction-parser-20260922.
- Branch: codex/ekr-transaction-parser-20260922.
- Build: <cache>/b10x-target/ekr-transaction-parser-20260922.
- Scratch: <cache>/ekr-completion-20260922/transaction-parser.
- Worker lease: codex-ekr-transaction-parser-implementor.

Use two build jobs, incremental disabled and dev/test debug info disabled. Do not
share the ESS build directory. Stop before free disk falls below 10 GiB and report
the measurement. Maintain and release only your own worktree lease.

## Scope and interface

Add the document module and focused integration tests; export the usable parser
from kernel lib.rs. Start from GraphTransaction<Value> and the existing typed
serde YAML operation tags. The later format unit supplies reasoned retraction
and supersession; use the shared GraphOperation decoder rather than a duplicate
operation enum. Do not claim those future operation shapes tested here.

The parser needs a bounded byte-slice entry and bounded Read ingress; expose exact
retained bytes, parsed transaction and original payload-domain hash. A stricter
caller upload limit must not redefine the frozen historical parser profile.
Choose ordinary API names; no state machine or provider layer is needed here.

New source scope: crates/ekr-kernel/src/document.rs and document/; new tests:
crates/ekr-kernel/tests/transaction_document.rs. Existing allowed edits:
crates/ekr-kernel/src/lib.rs and transaction.rs. Report any needed strict nested
declaration annotation outside these files before editing it. Do not edit Cargo,
planning, DESIGN, systems, legacy codecs, fixed historical vectors or CI.

## Measured scope addition

The implementor found actual typed-ID maps in NodeType.properties and
EdgeType.properties. Root added crates/ekr-core/src/decode.rs (inferred helper),
crates/ekr-core/src/lib.rs (export) and crates/ekr-ontology/src/types.rs (the two
field annotations) to the parent story scope through AEP. Implement one generic
unique-map decoder at that primitive layer and reuse it in the kernel carriers.
The duplicate key must refuse before its second value is decoded. Keep every
frozen legacy carrier unchanged and include nested schema-operation controls.

## Verification and handback

Start with meaningful failing tests and retain the red output. Exercise inclusive
limits independently without masking, exact original bytes, normal Record keys,
tagged operations, alias expansion and typed-key duplicates. Do not turn an
expected named refusal into a generic is_err test where the limit matters.
Run touched package tests, formatting, strict package Clippy and docs. Full
integration gate and provider/restart acceptance belong to root's later work.

Check each coordinator acceptance against the real decoder; report contradictions
with a reproducible witness. Preserve original adversary cases unchanged. Leave
the implementation reviewable and uncommitted for root. Report executed cases,
commands and exit codes, actual scope, limits and integration obligations. No
parser success establishes a durable Proposed record or writer completion.
