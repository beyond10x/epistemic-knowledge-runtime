---
format: aep.planning-md/2
id: architecture-decision-record:0001-own-kernel-on-eventlog
kind: architecture-decision-record
status: accepted
title: ADR 0001 — Own kernel, eventlog persistence (D1)
relations:
- decides: initiative:epistemic-knowledge-runtime
revision: 2
---
## Status

Accepted 2026-09-21 with the roadmap. Reopening it is a new ADR.

## Context

The design specifies a trusted kernel (§ 9), an ontology whose evolution is itself a transaction
(§ 26, § 51), assertions as the fundamental primitive (§ 13) with bitemporal semantics (§ 14), and a
one-way membrane between canonical and transient knowledge that should be unrepresentable to cross
(§ 6.1, § 23). v2 built its engine on the entity-runtime kernel. The question is whether v3 does too.

## Decision

The kernel, ontology, graph and assertion model are this workspace's own crates. Persistence goes
through `beyond10x/eventlog`: a committed revision root (§ 34) is an event, the canonical graph is a
fold, snapshots are eventlog snapshots, and § 60 deletion requests use eventlog's redaction contract.
entity-runtime is not a dependency of any runtime crate; it remains in the picture only where AEP
governance requires it (driver maps, the planning store).

## Why

entity-runtime's kernel evaluates `definition + instance + operation → Decision` over definitions
that are authored YAML validated at registration (`entity-runtime/README.md`). It has no graph roots,
no canonical/transient distinction, no bitemporal assertion, no schema-as-runtime-transaction and no
assertion primitive. v2 therefore stored whole-entity records with field-level provenance only as
`unverified[]` / `absent[]` (`docs/predecessors.md` § 6, conflict 9). Building § 13, § 22 and § 26 on
top of it means fighting the substrate. eventlog gives exactly the properties § 34–36 and § 60 need
and carries no domain concept, which is its own invariant.

## Consequences

- P1 implements validators, identity and the transaction boundary in Rust rather than adopting
  them; that is the cost.
- Per-type lifecycles and named operations, which v2 got from entity-runtime, are added to the
  ontology as amendment 87 (A13) and enforced by the ontology-constraint validator.
- The v2 instance's definitions are imported as the local schema of a transient root, not executed.
- eventlog's provider contracts (SQLite, PostgreSQL, file) are the storage surface; no product-specific
  storage fork.
