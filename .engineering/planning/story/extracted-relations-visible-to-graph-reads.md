---
format: aep.planning-md/3
id: story:extracted-relations-visible-to-graph-reads
kind: story
status: implemented
title: A relation applied through extraction is visible to graph reads
tags:
- consumer:cortex
relations:
- serves: vision:o5
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: crates/ekr-sdk/src/extraction.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/extraction_document.rs
- confidence: inferred
  path: crates/ekr-views/src/index.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/tests/extraction_cli.rs
- confidence: cited
  path: docs/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:51:49Z", actor: "agent:claude", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-05T11:51:49Z", actor: "agent:claude", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-06T06:08:54Z", actor: "agent:claude", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A relation applied through `ekr apply-extraction` is visible to graph reads: `search` counts it in a
node's `degree`, and `expand` walks it.

## Observed (ekr 0.0.30, 2026-10-05)

A store built only by `apply-extraction` from 8 web documents (consumer: `beyond10x/cortex`):

| read | result |
|---|---|
| `ekr snapshot` | 26 assertions: 25 with a `Relation` predicate, 1 `Property`; `graph.edges` holds 0 entries |
| MCP `search` `{"text":"Qdrant"}` | both `Qdrant` nodes answer `"degree": 0` |
| MCP `expand` `{"seeds":[<node>],"depth":1,"limit":50}` on the node that is the subject of 18 relation assertions | `node_total: 1`, `edge_total: 0` |

So the relations reach canonical state as assertions, and every read that walks the graph misses
them.

## Not established

The extraction path writes a relation fact as one `AddAssertion` with `Predicate::Relation` and
`Object::Node` (`crates/ekr-sdk/src/extraction.rs:560-583`) and never a `CreateEdge`, so
`graph.edges` stays empty. A consumer that writes its own transactions instead adds a
`CreateEdge` per relation beside the assertion, and its relations are walked. Whether the fix is
"extraction creates the edge" or "reads count relation assertions" is the owner's design choice;
the first matches what such consumers already do.

## Acceptance

An extraction document with one `!Relation` fact applies, and afterwards `search` reports `degree` 1
for both ends and `expand` from either end returns the other node and one edge.

## Scope

Derived 2026-10-05 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-sdk` extraction apply path — cited (story: `crates/ekr-sdk/src/extraction.rs:560-583`)
- **Files:** `crates/ekr-sdk/src/extraction.rs:545-597` (`Run::go`: the `Said::Relation` arm at 572-583 builds the `AddAssertion`; `groups.push(vec![cited.into()])` at 596 is where a `CreateEdge` would join the group) — cited
- **Files:** `crates/ekr-sdk/src/extraction.rs:31-41` (module doc, step 3: "one `!AddAssertion` each") — cited
- **Symbols:** `Run::go`, `Said::Relation(TypeId, usize)` (extraction.rs:216-221), `EdgeDraft::new` (`crates/ekr-sdk/src/document/graph.rs:91`) — cited
- **Documents:** `docs/cli.md:358-370` (`ekr apply-extraction` step 3) and `docs/cli.md:1992-1996` (Relations: assertion, edge or both) — cited, both describe the assertion-only behaviour; `CHANGELOG.md` `[Unreleased]` — inferred
- **Also likely:** `crates/ekr/tests/extraction_cli.rs` or `crates/ekr-sdk/tests/extraction_document.rs` for the acceptance test (search `degree` 1 and `expand` from both ends) — inferred
- **Also likely:** `crates/ekr/src/cli/agent.rs:108-115` (RELATIONS guide text) — inferred, only if the guide is to say what extraction writes
- **Alternative design (not recommended by the story):** `crates/ekr-views/src/index.rs:130-149` (degree and adjacency are built only from `graph.edges`) — cited as the read-side site, inferred as the landing site
- **Confidence:** high for the write-side design: the story names the defect site, and `index.rs:135-149` shows degree and adjacency come only from `graph.edges`
- **Would collide with:** any unit touching `ekr-sdk` `extraction.rs` `Run::go` (fact-to-operation building, held/claim logic), or the `ekr apply-extraction` section of `docs/cli.md`
- **Safety fact:** a `CreateEdge` is checked against the edge type's endpoint types and cardinality (`crates/ekr-kernel/src/validate/cardinality.rs:64`, `docs/cli.md:1994-1995`), and the assertion alone is not. If the edge goes in the same group as its assertion, a relation that applies today could be rejected after this change. Re-applying a document holds the fact (extraction.rs:589-595) before the push, so no duplicate edge is written. Step 2, unproven
- **Open for the implementor:** whether one rejected `CreateEdge` drops its assertion in the same group (`Batcher::commit_with_evidence`, `crates/ekr-sdk/src/batch.rs:283`, not read); whether `expand` reads the same index as `degree`; whether stores already built by extraction need an edge backfill (re-applying holds every fact, so none is written today)
- **Coordinator decisions (2026-10-05):** the write-side design: extraction emits a `CreateEdge` beside the relation's `AddAssertion`. A relation whose edge the kernel refuses (endpoint types, cardinality) is reported as that fact's refusal; the implementor reads `Batcher::commit_with_evidence` (`crates/ekr-sdk/src/batch.rs:283`) and states whether the assertion survives. No backfill of stores already built by extraction in this story.
