---
format: aep.planning-md/3
id: task:checkpoint-cadence-by-size
kind: task
status: draft
title: Checkpoints fall due by size, and cold open does not re-parse proposals
relations:
- serves: vision:o5
- decomposes: epic:ingestion-throughput
revision: 1
---
## Context

Performance audit of 2026-09-29. Design § 99 makes a checkpoint due after 512 operations, so every
consumer batch (1,561 operations) writes a full checkpoint: 44–48% of commit CPU; the checkpoint blob
is 53 / 158 / 527 MB at 1× / 3× / 10× (`crates/ekr-store/src/eventlog.rs:1206`, `:1253`,
`crates/ekr-kernel/src/checkpoint.rs:71`, `:379`).

Cold open also spends 52% of its time at 1× in `held_by` re-parsing every committed proposal's YAML
under profile v3, and 26% in SHA-256 over all loaded blobs (`checkpoint.rs:636`).

## Build

- A dated amendment to design § 99: a checkpoint is due by the bytes of operations or the number of
  commits since the last one, not by operation count alone; the thresholds are chosen from
  measurements at the 1× and 3× shapes.
- `held_by` reads the ids each commit created from a per-commit list bound to the commit by digest
  instead of re-parsing its proposal (a retained record added for new commits only; old commits keep
  the parse).

## Acceptance

- At the 1× shape, fewer than one checkpoint per 4 consumer batches is written (measured).
- Cold open under profile v3 at 1× takes at most half of today's 6.3 s CPU (measured).
- A reopen after any number of commits reaches the same roots (the replay-checkpoint tests).

## Scope (cited from the audit)

`crates/ekr-kernel/src/checkpoint.rs`, `crates/ekr-kernel/src/commands.rs` (cadence),
`docs/epistemic-knowledge-runtime-design.md` § 99.
