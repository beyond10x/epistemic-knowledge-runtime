---
format: aep.planning-md/1
id: task:two-revision-events-have-no-discriminator
kind: task
status: draft
title: A transaction rejected twice publishes two byte-identical events, so one of them can be lost
relations:
- blocks: story:commit-and-revision-lineage
- serves: vision:o2
- derived_from: story:version-persisted-contracts
revision: 2
---
## What is wrong

Four of `ekr_graph::RevisionEvent`'s six variants carry a minted `RevisionId` or another
discriminating payload. Two do not:

- `TransactionRejected { transaction_id, issues }`
- `TransactionStale { transaction_id, validated_against, current }`

A `TransactionId` is not unique per event. Design § 72 has a transaction revalidated rather than
committed, and `ekr_store::Fold::apply` is built for that — it **removes** a rejected or stale
transaction from the proposed set rather than marking it terminal, so the same transaction id
recurs by design.

So this lineage is ordinary: propose with one operation set, validate, reject with three issues;
propose again with a different set, validate, reject with the same three issues; commit. **The two
rejections are byte-identical**, and any store that keys an append on the event's content reads the
second as a retry of the first.

Found by the adversary of wave p1-05, unit 1, pass 2, which measured it: the store returned `Ok`,
wrote nothing, the fold never saw the second rejection, and the transaction committed.
`crates/ekr-store/tests/fold_rules.rs::a_rejected_transaction_cannot_then_commit` is the guard that
lineage walks past.

## Why the store is not the place to fix it

`ekr-store` derives its idempotency key from the event body because the alternative — deriving it
from the stream position — makes a retry after a lost acknowledgement compute a different key and
write the event twice. That was wave p1-05 unit 1's pass-1 finding 4 and it is the more dangerous
half. A key over content plus position reintroduces it exactly.

What unit 1 did instead, in its correction, is make `append` say which happened: written, or
recognised as an already-recorded request. That removes the silence. It does not remove the
ambiguity, because the two events really are identical and no store can tell a duplicate from a
retry when the bytes agree.

## What closes this

A discriminator on the two variants, so that two rejections of one transaction are two events.
The candidates:

- a minted `RevisionId`, as the other four carry — the simplest, and it makes every variant uniform;
- the `RevisionNumber` the rejection was decided against, which `TransactionStale` already carries
  half of as `validated_against`;
- the proposal's `operations_hash`, which distinguishes the two proposals the adversary's lineage
  makes and is already in `TransactionProposed`.

`systems/ekr/domains/kernel.yaml` declares these event shapes and `crates/ekr-graph/src/events.rs`
implements them, so the change is the graph crate's and the domain's together — and the hand-written
`variant_index` numbering and the declaration-order case move with it.

## Who owns it

`story:commit-and-revision-lineage`, wave p1-06. It writes the kernel that publishes these events
and is the first code that can produce the lineage at all; nothing in P1 writes it, because the
kernel does not exist yet. The adversary graded it on the mechanism rather than the consequence,
which is right: an append that returns `Ok` and drops the event is lossy whatever is written through
it, and the consequence needs a writer.

## Scope correction from persisted-contract preparation

The earlier proposed two-variant correction is incomplete. The read-only scope report
.engineering/waves/p1-persisted-contract-scope.md traces repeated Proposed and Validated content
as well as Rejected and Stale through the adapter's content idempotency. A discriminator tied to
the transaction, operations or validated revision can still repeat for a distinct decision.
story:version-persisted-contracts now owns this task before the writer.

Use an explicit occurrence envelope for every revision fact, with identity stable across retries
and new for separate occurrences. Same identity plus changed content refuses. Preserve backend
event metadata and route old records through their original decoder. Exact shapes and compatibility
rules are prepared in .engineering/waves/p1-persisted-contract-amendment-draft.md; they are not
implemented by this note. The old paragraph saying no kernel exists is historical.
