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
scope:
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
  path: systems/ekr/domains/kernel.yaml
revision: 3
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
