---
format: aep.planning-md/2
id: decision-blocker:checkpoint-unit-cardinality
kind: decision-blocker
status: open
title: Nobody has decided how many checkpoints a source unit has, or which of the two outlives the other
relations:
- blocks: epic:p2-observation-layer
revision: 2
---
## Question

For `SourceCheckpoint → SourceUnit`:

- Does a unit have exactly one current checkpoint that is replaced as it advances, or an append-only history of checkpoints (one per completed poll)?
- May a checkpoint exist before its unit is declared?
- When a unit is retired, is its checkpoint deleted, kept, or frozen?

## Why nobody can read the answer

- No ess/1 document declares a checkpoint entity.
- Design § 55 says only that "checkpoint state should be durable but not confused with canonical knowledge". `docs/predecessors.md` § 9 rule 5 says checkpoints "go to the checkpoint store", and § 6 row 6 places that store in `ekr-observe`. Neither states the cardinality or the lifecycle.
- A8 ("a successful poll proves only its window") argues for keeping each window. That is an argument, not a decision.
- The answer also depends on `decision-blocker:source-unit-granularity`: until a unit is defined, a per-unit checkpoint has no subject.

## Options

1. **One current checkpoint per unit, replaced in place.** Poll history lives only in poll-health records.
2. **Append-only checkpoint history per unit.** The current checkpoint is the latest entry, and each entry records the window it proves.
3. **One current checkpoint per unit**, plus a separate immutable poll record per attempt that carries its window.

## What it stops

Checkpoint persistence, and the "resume from checkpoint" behaviour of `SourceAdapter`. `story:observe-domain-model` carries this as an `UNMAPPED:` marker.
