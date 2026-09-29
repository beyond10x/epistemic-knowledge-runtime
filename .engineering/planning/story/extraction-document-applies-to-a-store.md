---
format: aep.planning-md/3
id: story:extraction-document-applies-to-a-store
kind: story
status: draft
title: An extracting agent's document applies to a store through one verb
relations:
- serves: vision:o5
- decomposes: epic:p3-incubation-integration
- depends_on: story:add-evidence-operation
scope:
- confidence: inferred
  path: crates/ekr-integrate/src/extraction.rs
- confidence: inferred
  path: crates/ekr-integrate/src/lib.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/cli/examples/extraction.yaml
- confidence: cited
  path: crates/ekr/src/cli/schema.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: crates/ekr/tests/schema_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/integrate.yaml
revision: 4
---
## Context

A consumer instance carries 1,686 lines that turn an extracting agent's output (named things and
facts, each citing one source item) into transactions: resolve before every `CreateNode`, attach
per-item evidence, batch, and grow the ontology (reported 2026-09-29). Any store fed by an
extracting agent needs the same path; design § 24 and § 63 place interpretation and integration in
the runtime.

Re-scoped 2026-09-29 by the coordinator: `epic:consumer-sdk` decided the apply verb runs on the
SDK's resolve and batch code (`story:extraction-verb-shares-the-sdk-path`). This story is the
document format and its apply report, specified and published; applying moves to that story.

## Build

- In ESS, `systems/ekr/domains/integrate.yaml`: the extraction document (types and relations by
  name, references with aliases reusing `TypedReference`, facts citing an evidence item whose shape
  is `ekr.graph.Evidence`, the one `AddEvidence` carries) and the apply report (committed
  transactions, rejected operations with their issues, ambiguous references reusing
  `ResolutionOutcome`).
- A reader and types for the document, in `crates/ekr-integrate`.
- Publication: `ekr schema` and `ekr example` serve the document; `docs/cli.md` documents it.
- The consumer's extractor, its agent and its sandbox stay the consumer's; the engine starts no
  agent (consumer input, 2026-09-29).

## Acceptance

- `ekr example <extraction format>` output is accepted by the reader, and `ekr schema` validates it
  (`crates/ekr/tests/schema_cli.rs` holds the schema to the reader in both directions).
- `crates/ekr/tests/agent_cli.rs` lists the new format, and `ess specify validate --path systems/ekr` is valid.
- A document naming a type absent from the store and a fact citing no evidence are each refused by
  the reader with a named code.

## Scope

Derived 2026-09-29 by `story-scoper`. **cited** = read from the tree, **inferred** = a reading that
could be wrong.

- **Publication:** `crates/ekr/src/cli/agent.rs` (`ExampleFormat`, `ExampleDocument`, `example()`, :560–620; guide text near :142), `crates/ekr/src/cli/schema.rs:11-22` — cited
- **ESS:** `systems/ekr/domains/integrate.yaml` — inferred; `ekr.graph.Evidence` reused from `graph.yaml:557` — cited
- **Reader:** `crates/ekr-integrate/src/extraction.rs` (new), `crates/ekr-integrate/src/lib.rs` — inferred
- **Example:** `crates/ekr/src/cli/examples/extraction.yaml` (new) — inferred
- **Tests:** `crates/ekr/tests/agent_cli.rs:322`, `crates/ekr/tests/schema_cli.rs` — cited; `crates/ekr-integrate/tests/domain_projection.rs` — inferred
- **Docs:** `docs/cli.md` (`ekr example`, `ekr schema` rows :133, :136, a format section), `crates/ekr/tests/docs_cli.rs` — cited
- **Conformance:** none for the format alone; the first integrate scenarios come with the apply verb — inferred
- **Confidence:** medium — publication surface read from the tree and the typed-reference precedent (5ce93aff); domain and reader crate are choices
- **Would collide with:** any unit adding an `ExampleFormat` or schema (`story:fact-quality-by-judged-sample`); any unit editing `integrate.yaml` or the `docs/cli.md` format sections
