---
format: aep.planning-md/3
id: story:extraction-document-applies-to-a-store
kind: story
status: active
title: An extracting agent's document applies to a store through one verb
relations:
- serves: vision:o5
- decomposes: epic:p3-incubation-integration
- depends_on: story:add-evidence-operation
scope:
- confidence: inferred
  path: crates/ekr-integrate/Cargo.toml
- confidence: inferred
  path: crates/ekr-integrate/src/extraction.rs
- confidence: inferred
  path: crates/ekr-integrate/src/lib.rs
- confidence: inferred
  path: crates/ekr-integrate/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/cli/examples/extraction.yaml
- confidence: inferred
  path: crates/ekr/src/cli/mod.rs
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
- confidence: cited
  path: systems/ekr/domains/integrate.yaml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:26:22Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:40Z", actor: "human:timo", revision: 10}
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

Derived 2026-09-30 by `story-scoper` against main 0.0.24 (`ec6bbf38`). Every line is **cited** (read
from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr` publication (`ekr example`, `ekr schema`) plus the `ekr.integrate` ESS domain — cited, story § Build
- **ESS:** `systems/ekr/domains/integrate.yaml`, new types after `ekr.integrate.ResolutionOutcome` (:73–80) and before `entities:` — cited (file); placement inferred
- **Files, publication:** `crates/ekr/src/cli/agent.rs` (`ExampleFormat` :661, `ExampleDocument` :681, `example()` :706, guide text :156–158), `crates/ekr/src/cli/schema.rs` (`run` :11–22) — cited
- **Files, reader and types:** `crates/ekr-integrate` — cited; `crates/ekr-integrate/src/extraction.rs` (new) and `src/lib.rs` — inferred
- **Example:** `crates/ekr/src/cli/examples/extraction.yaml` (new) — inferred
- **Tests:** `crates/ekr/tests/agent_cli.rs` (`FORMATS` :19–24), `crates/ekr/tests/schema_cli.rs` (:29–35, :101, :444, :589), `crates/ekr/tests/docs_cli.rs` (:377) — cited; `crates/ekr-integrate/tests/domain_projection.rs` — inferred
- **Docs:** `docs/cli.md` (`ekr example` :161, `ekr schema` :164, :597, :617, a new format section) — cited
- **Also likely:** `crates/ekr/src/cli/mod.rs` (the `Schema` help :224–225) and `crates/ekr-integrate/Cargo.toml` (`serde_yaml_ng` as a real dependency) — inferred
- **Reused, not changed:** `ekr_integrate::TypedReference` (`lib.rs:108`), `ResolutionOutcome` (`lib.rs:214`), `ekr.graph.Evidence` (`graph.yaml:568`); the ontology section matches `ekr_sdk` `OntologySpec` (`crates/ekr-sdk/src/document/ontology.rs:187`) — cited
- **Conformance:** none for the format alone; the first integrate scenarios come with the apply verb — inferred
- **Confidence:** medium — the publication surface, tests and ESS file are named by the story or read from the tree and match the typed-reference precedent (`5ce93aff`); the reader's file split is not fixed
- **Would collide with:** any unit adding an `ExampleFormat` variant or editing the format lists in `agent_cli.rs`, `schema_cli.rs`, `docs_cli.rs` or `docs/cli.md`; any unit editing `integrate.yaml` or `ekr-integrate/src/lib.rs`; dependency changes in `Cargo.toml`/`Cargo.lock`
- **Safety fact:** `crates/ekr-integrate/src/lib.rs` cites `integrate.yaml` by line span (:105–211) and `domain_projection.rs:169` fails if a span moves; new types after line 80 and before `entities:` move none — inferred
- **Not established:** the JSON Schema's home (for `typed-reference` it is `crates/ekr/src/cli/resolve.rs:239` because `ekr-integrate` derives no `JsonSchema`); where "a type absent from the store" is checked; the new refusal test's file name
