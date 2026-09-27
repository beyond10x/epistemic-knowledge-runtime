---
format: aep.planning-md/2
id: story:fixture-records-become-observations
kind: story
status: implemented
title: A synthetic JSONL fixture maps each record to one deterministic observation
relations:
- depends_on: story:observe-domain-model
- decomposes: epic:p2-observation-layer
- serves: vision:o5
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: README.md
- confidence: cited
  path: crates/ekr-observe
- confidence: inferred
  path: crates/ekr/tests/adversary_docs_contract.rs
revision: 12
---
## Context

This is the first code in the observation layer, and it produces observations at message level. Today evidence is one whole payload per seed entry (`story:seed-evidence-content-hash`), so an assertion can cite only a whole file. This story makes the unit of observation one source record.

A new workspace crate, `ekr-observe` (roadmap § 3), reads a **synthetic** JSONL fixture. It maps each line (one message: source identity, source-native id, timestamp, text) to exactly one `ekr_graph::Observation` with these values:

- `content_hash`: `ContentHash::of_bytes` over that line's bytes (`crates/ekr-core/src/hash.rs:74`, domain `ekr.payload.v1` at :53);
- `source` and `source_native_id`: taken from the record;
- `captured_at`: taken from the record;
- the id: derived from the idempotency key that `story:observe-domain-model` declares, so the same record always yields the same id.

Fixture text uses the runtime's own vocabulary (AGENTS.md, "What must not happen here"). It contains no customer or personal data, and no real source names.

Out of scope, each behind a filed blocker:

- persisting or deduplicating observations: `decision-blocker:observation-retention-path`;
- evidence citing them: `decision-blocker:evidence-entry-after-seed`;
- the `SourceAdapter` trait and per-unit checkpoints: `decision-blocker:source-unit-granularity`, `decision-blocker:checkpoint-unit-cardinality` and `decision-blocker:adapter-unit-ownership`.

Design sections served: § 15 (observations are immutable), § 54 (preserve source-native identifiers), § 56 (deterministic observation ids), § 57 (content addressing).

Package: `crates/ekr-observe` (new), plus its entry in the workspace `Cargo.toml` members.

## Domain relations

- `ekr.graph` owns `Observation`, so this crate uses `ekr_graph::Observation` and does not define its own. From `systems/ekr/components.yaml:41` (component `ekr-graph` owns domain `ekr.graph`) and `systems/ekr/domains/graph.yaml:590` (entity `ekr.graph.Observation`, fields `source`, `source_native_id`, `kind`, `content_hash`, `captured_at`).

## Acceptance

A test in `crates/ekr-observe` maps the synthetic fixture to exactly one `Observation` per JSONL line, each carrying that line's `ContentHash::of_bytes` hash, source, native id and timestamp, and mapping the same fixture a second time yields byte-identical observations, ids included.

## Scope

Derived 2026-09-27 by `story-scoper`. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- `crates/ekr-observe` — cited
- `Cargo.toml` — cited
- `Cargo.lock` — inferred
- `README.md` — inferred
- `crates/ekr/tests/adversary_docs_contract.rs` — inferred
- **Would collide with:** any unit adding or removing a workspace member (`Cargo.toml`, `Cargo.lock`, `README.md` § Status)
- **Confidence:** high for the paths the story names; medium for the inferred lines


## Scope learned (wave p2p3p4-02, implementor confirmation)

- `crates/ekr-observe` — confirmed (src/lib.rs, Cargo.toml, tests/{fixture_observations,adversary_observe,adversary_pass2}.rs, tests/fixtures/source-records.jsonl)
- `Cargo.toml` (root), `README.md`, `crates/ekr/tests/adversary_docs_contract.rs` — **wrong** for the unit: the opening commit covered them, or nothing needed changing
- `Cargo.lock` — confirmed
- **missed by the scoper:** `crates/ekr-core/src/identity.rs` module doc (derived observation ids), changed by the coordinator at the close
