---
format: aep.planning-md/3
id: story:validation-profile-move
kind: story
status: draft
title: An operator's recorded event moves a store forward to a later validation profile; replay uses the profile in force (ADR 0102 d6, d7)
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersession-verdict-fixes-replay-safe
revision: 1
---
## Context

Atlas ADR 0102 (accepted 2026-10-21, atlas `d5d1dcd6`) decision 6 = B: "an operator-authorised log
event moves a store from its profile to a later one at a revision; replay uses the profile in force
at each proposal's position; a checkpoint already hashes the profile (`replay.rs:642`) and would hash
the one in force". Today a store keeps the profile its seed named and "no mechanism moves a store
from one to the other" (`crates/ekr-kernel/src/authority.rs:39-41`; design
`docs/epistemic-knowledge-runtime-design.md:4231-4237`, "a later milestone").

Decision 7: "A store written by the 0.0.39 binary, holding a D7, a D8 and a Q7 verdict, is checked in
and reopened with and without `--full-replay` on both providers, as `base_era_v1_replay.rs` did for
v2."

Depends on `story:supersession-verdict-fixes-replay-safe` (profile v4).

## Build

1. A new log event kind: a profile move, forward only (v1 or v2 to v3 or v4, v3 to v4), recorded at
   a revision on an operator's explicit command (a CLI verb), refused otherwise.
2. Replay validates each retained proposal under the profile in force at its position: the seed
   profile, then each recorded move. The reopening compare of the anchor (`authority.rs:39-41`)
   becomes "seed profile plus recorded moves". A checkpoint hashes the profile in force.
3. Both providers (the file and SQLite stores, and PostgreSQL where the other events are).
4. The d7 fixture: a store written by the 0.0.39 binary (built once from the tag), holding a D7, a D8
   and a Q7 verdict, checked in; reopened with and without `--full-replay` on both providers; then
   moved to v4, after which a new proposal of each kind gets the v4 verdict.

## Acceptance

- The fixture store reopens and fully replays unedited before and after the move; proposals before
  the move's revision replay under their profile, after it under v4.
- A move backwards, to the same profile, or without the operator's command is refused.
- A store with no move keeps its knowledge root and checkpoint bytes.
- 0.0.41 refuses a store holding a move (stated in the CHANGELOG as Breaking).

## Out of scope

Any new verdict; automatic moves.
