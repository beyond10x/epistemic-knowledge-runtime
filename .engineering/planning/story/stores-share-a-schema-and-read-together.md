---
format: aep.planning-md/3
id: story:stores-share-a-schema-and-read-together
kind: story
status: draft
title: A private store imports a shared store's ontology, and one read answers over both
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: architecture-decision-record:0014-stores-share-a-schema-and-read-together
- informed_by: epic:type-packs
revision: 1
---
## Context

A consumer keeps a shared store and wants a person's private store to reuse the shared store's
ontology, and to answer one query over both, without copying private content into the shared store.
Today each store's ontology is its own: two stores that declare "the same" type hold different type
ids (invariant 3 — names are not identities), so neither a reader nor an agent can join their
answers, and every read tool answers over exactly one store.

`epic:type-packs` (draft) reaches equal type and property ids by seeding stores from one pack
version. This story reaches them by importing from a store, pinned to one of its schema revisions,
and adds the read across stores, which neither the pack epic nor any other artifact covers.

Requested:

- `ekr ontology import --from <store>`, pinned to a schema revision of the source store;
- MCP `search` and `node` over an ordered list of stores, each result tagged with the store it came
  from.

This moves a responsibility into the runtime: until now a store answered only for itself, and
relating stores was a consumer's concern. `architecture-decision-record:0014-stores-share-a-schema-and-read-together`
proposes that move, with its alternatives. **No implementing wave starts on this story until the
operator has accepted that ADR**; while it is proposed this story stays a draft.

## Invariants this story holds

- Invariant 1: the import is a schema transaction in the importing store, built and committed by
  that store's kernel and citing the source store and schema revision as its basis
  (design § 109); nothing writes into the source store.
- Invariant 2 and 3: an imported type keeps the source's type id, so equal ids mean the same type
  by construction; a name match never makes two types equal.
- Invariant 5: the import adds; a later import of a newer source revision is a new schema version,
  not a rewrite.
- No content crosses: the federated read opens each store read-only and copies nothing between them.

## Build (spec first)

1. `systems/ekr/domains/ontology.yaml` first, green under `ess specify validate --path systems/ekr`
   and its conformance suite before any code: the import command, the record of which source store
   and schema revision an imported type came from, and the refusals (a source revision that does not
   exist, an id already declared differently in the importing store, an import that would remove a
   type, which evolve's no-removal rule forbids).
2. `systems/ekr/domains/views.yaml`: a store list as a read input, the store tag on every result,
   ordering and de-duplication rules (the first store in the list wins on an equal id, and the
   answer says so), and the refusal when the listed stores do not share the schema revision the
   query names.
3. `systems/ekr/domains/cli.yaml` and `docs/cli.md`: `ekr ontology import`, and the option that gives
   `ekr mcp`, `ekr mcp-http` and `ekr view` more than one store.

## Acceptance

- `import_pins_source_schema_revision`: importing from a synthetic source store at schema revision N
  gives the importing store byte-equal type and property ids for every type of revision N, and its
  schema history names the source store and N.
- `import_refuses_a_conflicting_declaration`: an importing store that already declares an imported id
  differently refuses the import by name and is unchanged.
- `reimport_adds_only`: importing revision N+1 after N adds exactly what N+1 added, as a new schema
  version.
- `source_store_is_never_written`: the source store's head and bytes are identical before and after
  the import and after any federated read.
- `federated_search_tags_each_result`: `search` over the private and the shared store answers results
  from both, each tagged with its store, in the list's order.
- `federated_node_resolves_by_order`: `node` for an id present in both answers the first listed
  store's node and names the other store as also holding it.
- `federated_read_refuses_mismatched_schema`: two stores that do not share the named schema revision
  are refused by name rather than merged.
- `single_store_reads_unchanged`: every existing read with one store answers the same bytes as before.

## Out of scope

Writing across stores; copying content from one store to another; access control over which reader
may see which store (the caller decides which stores it lists); federation across processes or
hosts; seeding from a type pack, which `epic:type-packs` owns.
