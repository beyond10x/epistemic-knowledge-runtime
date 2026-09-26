---
format: aep.planning-md/2
id: architecture-decision-record:0004-timestamp-in-ekr-core
kind: architecture-decision-record
status: accepted
title: ADR 0004 — Timestamp is an ekr-core newtype over i64 milliseconds
relations:
- decides: story:graph-model-and-assertions
revision: 3
---
## Status

Proposed 2026-09-21, before wave p1-04 dispatched. It is a coordinator decision taken ahead of the
implementation because every later crate inherits the type, and a choice made in passing inside one
crate would be re-litigated in four.

## Decision

`Timestamp` is a newtype in **`ekr-core`**, over `i64`, counting **milliseconds since the Unix
epoch in UTC**. It carries a `Canonical` impl that is structural over the integer — no
discriminant, per `task:canonical-newtype-discriminant` — and a decimal text form through `Display`
and `FromStr`, matching how `RevisionNumber` is treated in `crates/ekr-core/src/identity.rs`.

`ekr-ontology` adopts it at its two existing sites: `Value::Timestamp`
(`crates/ekr-ontology/src/value.rs:208`) and `SchemaVersion::created_at`
(`crates/ekr-ontology/src/schema.rs:31`), both of which are bare `i64` today.

## Why `ekr-core` and not `ekr-graph`

The roadmap's plan for wave p1-04 said `ekr-graph` would define it. Reading the specification
before dispatch overturned that: `systems/ekr/domains/ontology.yaml:100-101` declares
`SchemaVersion.created_at` of type `Timestamp`, and **`ekr-ontology` does not depend on
`ekr-graph`** — the workspace order is `ekr-core <- ekr-ontology <- ekr-graph <- ekr-store <-
ekr-kernel`. A `Timestamp` in `ekr-graph` would therefore leave one domain scalar represented by
two unrelated Rust types, which is the drift `crates/ekr-ontology/tests/domain_projection.rs`
exists to catch.

Four of the five ESS domains declare a `Timestamp` field — `kernel.yaml:175`, `graph.yaml:133`,
`ontology.yaml:101`, `store.yaml:40` — so the type is used at every level of the stack and belongs
at the bottom of it.

## Why milliseconds, and why a hand-rolled newtype

- **No time crate is available.** No workspace manifest declares `chrono`, `time` or `jiff`, and
  `story:workspace-crate-skeleton` fixed the dependency set. Adding one is outside P1.
- **Milliseconds rather than nanoseconds.** `i64` nanoseconds spans 1678–2262. Valid time in
  design § 13–14 is the time a fact was true in the world, which for an organizational memory
  includes dates outside that window. `i64` milliseconds spans roughly ±292 million years.
- **Milliseconds rather than seconds.** Transaction time orders records written inside one
  process; second resolution loses that order. Commit order itself is carried by `RevisionNumber`
  rather than by the clock, so millisecond collisions are not a correctness problem.
- **Signed rather than unsigned.** Valid time before 1970 is ordinary in this system: a claim
  about when a company was founded is one.

## What this decision does not settle

- No formatting to or from RFC 3339. That needs a calendar, which needs a dependency.
- No clock. Nothing in `ekr-core` reads the system time; a `Timestamp` arrives from its caller.
  Design § 19–20 makes the transaction the thing that stamps a record, and that is P1's later
  waves.
- No leap-second or monotonicity claim. The type is an integer with an origin and a unit.


## Correction, 2026-09-21

This record said **three** `ekr-ontology` sites when it was accepted. There are two. The third it
named, `ValueKind::Timestamp` / `ValueType::Timestamp` (`value.rs:78`, `value.rs:149`), are unit
variants carrying no payload, so there was nothing there to change. Measured by the implementor of
wave p1-04 against the tree; corrected above.

`Value::Duration(i64)` is left as a bare integer and is not covered by this decision. A duration is
a length of time rather than an instant, and no `Duration` newtype exists.
