---
format: aep.planning-md/3
id: task:checkpoint-cadence-costs-each-commit
kind: task
status: draft
title: A replay checkpoint on every commit and a pointer after every verb cost about 45 ms
relations:
- serves: vision:o5
- derived_from: task:write-verbs-cost-most-of-an-ingest
revision: 1
---
## What is wrong

Measured in wave read-01 unit W (commit `659a913f`, release profile of propose/validate/commit in
one `ekr session` on a 19,971,175-byte file store): after that unit, a commit still costs 123–212 ms
in a session. About 45 ms of it on tmpfs is the replay checkpoint written on every commit (a 4 MB
blob hashed three times, then a delete) and the checkpoint pointer written after every verb, each
its own atomic group of 8–10 `fsync` calls. `crates/ekr-kernel/tests/replay_checkpoint.rs` pins one
pointer per verb and one blob per commit, which is the cadence design § 96.3 describes.

## What closes this

A dated design amendment after § 98 that sets a cheaper cadence (for example a checkpoint every N
commits or when the replayed suffix exceeds a size, and no pointer write for a verb that did not
move the head), `replay_checkpoint.rs` updated to that rule, and a measurement showing commit in a
session under 100 ms on the same store, with a reopen after any number of commits still reaching
the same roots.
