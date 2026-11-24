---
format: aep.planning-md/3
id: story:checkpoint-cases-cost
kind: story
status: draft
title: Checkpoint cases under 60 s summed
summary: Profile and cut the cost of the checkpoint cases with every property kept
relations:
- serves: vision:o5
revision: 1
---
## Outcome

EKR's checkpoint cases take under 60 s summed on CI, down from about 175 s, and every property they
check still holds. This is part of `story:ekr-test-step-budget`.

## Work

Measured on CI run 45314673716 (after PRs 115 and 116):

- `adversary_checkpoint_cadence_p1` takes 126.6 s across 15 cases. The 12 mixed-history cases take
  8.4-11.4 s each, with the file provider about 1.5 s slower than sqlite. Each of 36 steps opens a
  fresh handle and compares answers on three handles. At each new checkpoint, a fresh handle
  replays from the seed (PR 115).
- `replay_checkpoint` takes 32.5 s and `adversary_checkpoint_by_size_p2` about 18 s. The byte-bound
  cases (10.2-10.5 s each) commit about `REPLAY_CHECKPOINT_BYTES` (16 MiB) of documents, so they
  validate, hash and store 16 MiB.

Profile one case of each on CI or on the build server: user CPU against fsync and other waits.
Then cut the dominant cost without sampling steps or dropping a comparison.

## Acceptance

On CI the checkpoint cases sum under 60 s. Every assertion of the three files is kept.
