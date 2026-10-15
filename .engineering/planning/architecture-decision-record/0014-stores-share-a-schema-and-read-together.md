---
format: aep.planning-md/3
id: architecture-decision-record:0014-stores-share-a-schema-and-read-together
kind: architecture-decision-record
status: accepted
title: ADR 0014 — A store imports another store's ontology by schema revision, and a read may span stores
summary: 'Proposed: the runtime takes on schema import pinned to a source revision and a read over an ordered list of stores, with no content copied between them'
relations:
- decides: story:stores-share-a-schema-and-read-together
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-15T20:22:47Z", actor: "agent:ekr-controller", revision: 2}
---
## Status

Proposed on 2026-10-15 with `story:stores-share-a-schema-and-read-together`. Not accepted: no
implementing wave starts on that story until the operator accepts this record.

## Context

A store answers for itself. Its ontology mints its own type and property ids, and every read tool
(MCP, `ekr view`, the session verbs) answers over exactly one store. A consumer that keeps a shared
store and per-person private stores needs two things the runtime does not do: the private store
reusing the shared store's types by id, and one query answered over both with each result saying
where it came from, without private content entering the shared store.

Two facts constrain the answer:

- Names are not identities (invariant 3). Two stores that each declare a type with the same name
  hold two different types; any join by name is a guess.
- Schema changes are transactions that cite their basis (design § 109) and evolve never removes a
  type, so a schema revision is a stable thing to pin to.

`epic:type-packs` already proposes one route to shared ids — seeding from a shipped pack — but a
pack is authored ahead of time, and the shared store here grows its schema from its own evidence.

## Decision (proposed)

The runtime takes on two responsibilities it has not had:

1. **A store may import another store's ontology, pinned to a schema revision.** The import is an
   ordinary schema transaction in the importing store, committed by its kernel, keeping the source's
   type and property ids and recording the source store and revision as provenance. The source store
   is only read.
2. **A read may name an ordered list of stores.** Each store is opened read-only, the read is
   answered per store, every result carries its store's tag, and an id held by more than one store
   resolves to the first listed and names the others. Stores that do not share the schema revision
   the read names are refused, not merged.

No content moves between stores, and no write ever spans two stores.

## Alternatives considered

- **Leave it to the consumer.** Each consumer maps types by name and merges answers itself. Rejected
  in this proposal because the mapping is exactly the name-as-identity guess invariant 3 rules out,
  and every consumer would rebuild it.
- **Copy the private content into the shared store under an access label.** One store, one read.
  Rejected because it puts private content where the shared store's readers and maintenance act on
  it, which the request rules out, and because retention and purge would then have to reason about
  two owners of one store.
- **Type packs only (`epic:type-packs`).** Shared ids from a shipped pack. Kept, and complementary:
  it covers schemas authored ahead of time, not one learned in a running store, and it says nothing
  about reading across stores.
- **A federation service outside the runtime.** A separate process that fans a query out. Rejected
  for now because it needs the same pinned-id guarantee from the runtime to be correct, and the
  read tools already own result formats and refusals.

## Consequences

- The ontology domain gains an import command and an imported-from record; the views domain gains a
  store list as a read input and a store tag on results. Both are specified before code.
- Read answers grow a field (the store tag) only when more than one store is named; single-store
  answers stay byte-identical.
- A type's id becomes something two stores can share by design, so a conflicting declaration of an
  imported id is a named refusal the runtime must hold.
- Opening several stores in one reader multiplies open cost and held bytes; a federated read counts
  its work per store, and its bounds are part of the views specification.
- Access control over which stores a reader may list stays with the caller; this record does not move
  identity or authorisation into the runtime.
