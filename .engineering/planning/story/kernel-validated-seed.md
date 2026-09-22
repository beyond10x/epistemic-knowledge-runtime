---
format: aep.planning-md/1
id: story:kernel-validated-seed
kind: story
status: draft
title: Initialize and reopen seeds through kernel validation
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:p1-transaction-membrane-repair
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
- depends_on: story:refuse-discarded-ontology-semantics
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/ekr-graph/src/assertion.rs
- confidence: cited
  path: crates/ekr-graph/src/canonical.rs
- confidence: cited
  path: crates/ekr-graph/src/edge.rs
- confidence: cited
  path: crates/ekr-graph/src/evidence.rs
- confidence: cited
  path: crates/ekr-graph/src/node.rs
- confidence: cited
  path: crates/ekr-graph/src/root.rs
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate
- confidence: cited
  path: crates/ekr-kernel/tests/commit_path.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/fixtures/seed-minimal.yaml
- confidence: inferred
  path: crates/ekr-kernel/tests/seed.rs
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/src/lib.rs
- confidence: cited
  path: crates/ekr-store/src/log.rs
- confidence: cited
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-store/tests
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 16
---
## Context

Split the seed admission portion out of story:seed-and-explain so it can close the canonical crossing before durable transaction application. Existing design sections 8 and 21 and ekr.kernel.Seed settle kernel ownership. This adds no new persistent domain noun: it implements the already declared Seed command, canonical graph and ontology.

## Acceptance

A seed with a dangling endpoint, an undeclared type, or caller-supplied Accepted-with-no-validator assertion receives a named refusal through each real backend. Invalid input writes neither a seed object nor a Seeded event. A valid minimal seed produces revision zero and survives close/reopen through the real kernel. Repeated or concurrent initialization cannot replace the existing lineage. The public Commit API exposes no raw store writer.

## Bootstrap contract

Input is a versioned YAML document containing an OntologyDocument and proposed graph data. Kernel checks load the complete ontology, canonical root filing, matching schema, map identities, references, types, required properties, multiplicities, initial lifecycle states, retained evidence and authority. Empty graph seeds are allowed without changing ordinary empty-transaction refusal. Caller-supplied verdicts do not grant acceptance: the actual kernel validation supplies acceptance attribution. Seed operator and validator identity are supplied by execution context, not by payload claims.

Bootstrap has no previous revision and may introduce seed evidence directly. These are explicit bootstrap differences, not permission to waive graph checks. Unsupported constraints refuse. Persist enough seed input and validation attribution to repeat deterministic checks on reopen.

## Store boundary

Move canonical graph construction from untrusted bytes behind kernel validation. Store replay asks the injected kernel authority to admit the seed document; it must fail closed without that authority. Do not implement another process-local set of seed hashes. Conditional initialization uses the eventlog empty-stream expectation so concurrent first writers cannot both seed. Existing externally supplied ontology, while retained for constructor compatibility, must agree with the full persisted seed ontology.

Replace Commit::store with read methods for head, snapshot and replay. Provider tests may keep explicitly labeled substitute authorities, but they do not satisfy kernel acceptance. Test-only raw handles can inject corrupt events without a production raw-writer accessor.

## Scope

Cited: kernel commit.rs and lib.rs; kernel validation helpers; store snapshot.rs, eventlog.rs, log.rs and lib.rs; kernel commit_path tests; store fixture and lineage helpers; store membrane boundary tests; systems/ekr/domains/kernel.yaml.
Inferred additions: kernel seed.rs, tests/seed.rs and tests/fixtures/seed-minimal.yaml. The story-scoper traced the production replay crossing and shared fixture callers. Coordinator owns ESS and planning writes.

## Verification

Promote the three archived seed cases into real kernel tests, observing red first. Add valid minimal and evidence-bearing controls, determinism, repeated/concurrent initialization, wrong ontology, invalid lifecycle, unsupported constraint, tampered payload and missing-authority cases. Run both SQLite and file backends. Preserve relevant existing filing, transient-space, float and self-parent tests at the new boundary. Replace the old test that demonstrates successful dangling conversion with a refusal regression.

## Remaining work

Post-seed transaction application, durable transaction receipts and historical queries remain in the writer story. Explain stays in story:seed-and-explain. Full configurable agent roles and validator policy must not be claimed implemented by hard-coded bootstrap checks; bind their eventual populated state to the agent root.

## Implementation decisions from preparation

The implementor's read-only preparation traced the exact pinned eventlog API. Use AtomicEventStore::append_group to publish the retained seed envelope and Seeded together, with Expected::NoStream on the revision stream. A precheck or put-then-append is insufficient for the declared already-seeded outcome that writes nothing. Handle typed conflicts before converting provider failures to text; preserve content-addressed deduplication without weakening the revision expectation.

Recompute the loaded seed payload's ContentHash and verify record metadata before authority admission. Persist the original versioned input and actual bootstrap attribution; replay revalidates both. Compare loaded Ontology equality with the compatibility ontology, so no export API is needed merely to compare full content.

Bootstrap validation produces a private kernel capability consumed by the same kernel-owned persistence boundary. This represents initialization with no prior revision; do not invent a previous committed revision or weaken ordinary Pipeline::validate. Reuse its deterministic invariant checks where meaningful. Clarify AGENTS invariant 1's initialization wording with the actual executable seed cases when implemented.

Legacy raw GraphDocument seeds have no retained ontology or bootstrap attribution. Preserve their bytes and refuse explicitly with migration-required; do not invent missing history or reinitialize the lineage. The approved preservation-first migration policy remains binding.

These implementation decisions are not yet executed. Their acceptance is the real-backend seed suite in this story. Additional cited surface: crates/ekr-store/src/lib.rs for named store refusals, crates/ekr/tests/story_contract.rs for the canonical-writer ownership guard, and AGENTS.md for the verified bootstrap boundary. New private helpers use explicit restricted visibility.

## Retained bootstrap evidence and decoding

The seed's existing story selects HumanStatement evidence for bootstrap, and design sections 6.1, 6.2, 16, 21 and 37 require retained, resolvable support. Retain exact statement bytes in the versioned seed envelope, keyed by content hash. For every declared Evidence record, including uncited ones, require a matching payload and verify ContentHash::of_bytes against its content_hash before any publication. Recheck the binding on replay and expose a read-only content lookup for explain.

Bootstrap admits HumanStatement sources only. Other source variants, including GraphAssertion and Observation, receive named unsupported-source refusals until their referenced targets and independent support can be resolved by the corresponding subsystem. A HumanStatement's optional free-text identity is metadata, not authenticated authority; actual bootstrap attribution comes from execution context. Empty evidence is valid only when the seed has no assertions requiring it. This is a bounded bootstrap rule, not a general source-credibility policy.

This payload map is part of the same atomic seed publication. Invalid or tampered payloads write nothing; replay of tampered content fails. Legacy seeds without verifiable statement content remain preserved and refuse migration rather than receiving synthetic evidence. P6 erasure rules also apply to seed evidence bytes; embedding them does not exempt them from deletion or dependent-assertion handling.

Strict seed decoding must refuse unknown semantic fields throughout its graph records, as the ontology decoder now does. Add record/envelope strictness only where seed input reaches the type, preserving legitimate user-defined map keys. The graph source files are explicitly scoped for these decode attributes and correction of the obsolete seed-boundary documentation; this does not authorize unrelated graph-shape changes.
