---
format: aep.planning-md/3
id: task:migrate-reads-a-current-store
kind: task
status: proposed
title: ekr migrate reads a store that took evidence after its seed
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:42:14Z", actor: "human:timo", revision: 5}
---
## What is wrong

A consumer reported on 2026-10-01, and corrected the same day. `ekr migrate` fails on any store that
holds a committed `!AddEvidence`, with `ekr: a stored document could not be read:
required-object-missing` and exit 1. It fails the same way under 0.0.24 and 0.0.25, so 0.0.25 did
not cause it; it dates from the `AddEvidence` operation or earlier.

- Smallest failing case: the consumer's seed plus one transaction holding only an `!AddEvidence`.
  It fails on both 0.0.24 and 0.0.25.
- Passing cases: a seed only; a seed plus `CreateNode`s; a seed plus an assertion that cites seed
  evidence, with no `!AddEvidence`.
- The reproduction is synthetic data at `~/.cache/ekr-migrate-repro/cb3-store/`. `run.sh <ekr>
  <list>` rebuilds the store with `ekr seed`, `propose`, `validate` and `commit`, then runs
  `migrate`. `case/minimal.txt` is the smallest case, `case/tx-order.txt` the full one (a seed plus
  11 transactions), and `minimal-<version>.out` the outputs.

Who reaches it: every consumer store that took evidence after its seed, including the consumer's
live store, so none of them can be migrated.

Mechanism read from the code at 0.0.25 (`e92f7341`), not yet run: `Migration::migrate_into`
(`crates/ekr-kernel/src/migrate.rs:139`) publishes each occurrence with its record object only
(`:256–260`). It carries every other object, the Provenance payloads an `!AddEvidence` commit stored
among them, after the loop (`:278`, "Every other object"). Inside the loop each later occurrence is
re-derived in the destination: `destination.authority.reconstruct(&held, …)` (`:243–246`) and then
`migrated_decision` (`:256`). That needs the payload of an evidence entry the history added, and it
is not there yet. A seed-only store, and a store whose assertions cite only seed evidence, never
reach that state (seed payloads are published with the seed occurrence, `:418–424`), which fits
the passing cases.

## Build

First a failing test in the default gate: seed, one `!AddEvidence` commit, `ekr migrate`, on both
providers. Then the fix. The migrated store must keep every added evidence entry and its bytes.

## Acceptance

- The reproduction's minimal and full cases migrate under the fixed release, and `ekr explain` of
  an assertion citing added evidence answers the same before and after migrating.
- A store with envelope /2 still migrates.
