---
format: aep.planning-md/2
id: architecture-decision-record:0010-replay-checkpoints-record-verified-history
kind: architecture-decision-record
status: accepted
title: ADR 0010 — A replay checkpoint records verified history; --full-replay re-derives it
summary: Opens continue from a bound, checked checkpoint instead of re-deriving every past decision; a forged one is ignored
relations:
- decides: story:version-persisted-contracts
- decides: story:eventlog-0-4-batched-reads
revision: 2
---
## Status

Accepted on 2026-09-26 by the operator's orchestrator for release 0.0.7, together with design
§ 96.3, which it records as a decision.

## Context

Invariant 1 of `AGENTS.md` and design §§ 91.5 and 94 had every open of a store replay the whole
retained lineage through the kernel authority, re-deriving each decision. On a store of 65
revisions that cost 5.1 s per `ekr head` and about 25 s per propose, validate or commit of 250
operations (§ 96, "What was measured").

## Decision

A kernel writes a private replay checkpoint (`ekr.replay-checkpoint/1`) after each seed or
commit it publishes, and a later open continues from it. A checkpoint records that the prefix it
covers was replayed and every retained decision in it re-derived when it was written; the open
does not repeat that verification for the covered prefix. It checks the checkpoint's binding to
this host, anchor and exact prefix digest, and that its head graph and schema versions reproduce
the roots the retained receipts carry; a checkpoint or pointer that fails any check is ignored and
the history replays in full.

`--full-replay` (`EKR_FULL_REPLAY=1`, `Runtime::set_full_replay`) remains the full check: it
ignores checkpoints and replays from the seed, re-deriving every decision.

## Consequence accepted

A writer able to append both a self-consistent history and a matching checkpoint to the store could
have an unvalidated transaction read as canonical until a full replay. Such a writer already holds
the store's storage credentials; the checkpoint is never an authority, is outside canonical object
coordinates, and a forged or stale one that fails its binding is ignored.

## Alternatives

Replaying in full on every open is the previous behaviour and its measured cost. Caching within a
process only (§ 96.2) does not help a one-command process such as the `ekr` binary. Signing
checkpoints would need a key the store's own writer cannot hold, which this runtime has no place for
yet.

## Evidence

`crates/ekr-kernel/tests/replay_checkpoint.rs` on both providers: a fresh open continues from the
checkpoint with the answers of a full replay; a checkpoint that does not verify is ignored and the
history replays in full; validating against a revision the checkpoint holds no graph of replays in
full. `crates/ekr-kernel/src/checkpoint.rs`: a fresh open restores the head and replays nothing the
checkpoint covers.
