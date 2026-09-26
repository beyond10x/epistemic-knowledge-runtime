---
format: aep.planning-md/2
id: task:store-snapshot-and-its-id-are-declared-not-implemented
kind: task
status: implemented
title: ekr.store.Snapshot and SnapshotId are declared by the domain and bound to no Rust type
relations:
- serves: vision:o2
revision: 6
---
## What is wrong

`systems/ekr/domains/store.yaml` declares `ekr.store.Snapshot` and `ekr.store.SnapshotId`. Wave
p1-05's unit 1 implemented neither, deliberately, and said why in its report.

`ekr.store.SnapshotId` is `newtype of: Uuid`. All fifteen id newtypes this workspace has are
declared in `crates/ekr-core/src/identity.rs` by one macro that owns `mint()`, the single-text-form
rule, `FromStr`, `Display` and `Canonical`, and `ekr-core` is the crate carrying the `uuid`
dependency. Declaring a sixteenth inside `ekr-store` means either a second `uuid` dependency and a
copy of that macro, or an id that is not shaped like the other fifteen. `ekr-core` was not that
unit's file.

`ekr.store.Snapshot` is the materialised fold at a revision, which is `GraphDocument`'s neighbour in
`crates/ekr-store/src/snapshot.rs` — but it carries a `SnapshotId`, so it waits on the same thing.

## Why nothing is broken today

Neither is reached by `story:eventlog-store`'s acceptance or by any of the three tests it ships.
`crates/ekr/tests/public_surface.rs` holds that no public item arrives untested, so adding either
without a caller would have turned the gate red rather than passing quietly.

`ess specify validate` passes: the ESS document declaring an entity no Rust type binds is not a
refusal, which is the same gap `ekr.graph.TypedValue` sits in.

## What closes this

`SnapshotId` beside the other fifteen in `crates/ekr-core/src/identity.rs`, through the existing
macro, then `ekr.store.Snapshot` in `crates/ekr-store/src/snapshot.rs` beside `GraphDocument`, with
the cases the coverage guard requires.

It belongs to whichever story first needs to name a materialised fold — `story:seed-and-explain` or
`story:ekr-cli` are the candidates — rather than to a wave that would add a type with no caller.

## The wider gap this is the second instance of

`crates/ekr-graph/tests/domain_projection.rs` binds each declaration in `graph.yaml` to the Rust
types that project it, and carries an explicit list of the ones it binds to nothing. `store.yaml`
has no such guard, so a declaration nobody implements is invisible rather than listed. Two are
invisible right now and it took an implementor's report to say so.

## Wave p1-14 rescope

Rescoped in wave p1-14 (2026-09-23): both homes this task named shipped without it.
`ekr.store.Snapshot` and `SnapshotId` are declared in `systems/ekr/domains/store.yaml`, no Rust type
binds them, and the `snapshot` verb returns `ekr.kernel.SnapshotResult`. Decision owed: remove the
two declarations, or bind them with a projection case. Coordinator default: remove them in wave p1-14.

## Scope

Derived 2026-09-23 by `story-scoper` (wave p1-14). Coordinator default: remove both declarations.

- `systems/ekr/domains/store.yaml:14-16` (`ekr.store.SnapshotId`) and `:213-234` (`ekr.store.Snapshot`, the only user) — cited
- stale after removal: `crates/ekr-store/src/snapshot.rs:20` doc names `ekr.store.Snapshot`; `crates/ekr-core/tests/identity_serde.rs:150` comment says `SnapshotId` arrives with `ekr-store` — cited
- not affected: `components.yaml` and `kernel.yaml` name `ekr.kernel.Snapshot`, a separate declaration — cited
- `crates/ekr-store/tests/domain_projection.rs` covers only `StorageClass` and events; no case holds `types:` or `entities:` declarations, so a declaration-coverage case is the one that stops this recurring — inferred
- Confidence: high for the removal set, medium for the test line
