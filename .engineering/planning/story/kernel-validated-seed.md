---
format: aep.planning-md/2
id: story:kernel-validated-seed
kind: story
status: implemented
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
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_p1_08_seed.rs
- confidence: cited
  path: crates/ekr-kernel/tests/commit_path.rs
- confidence: cited
  path: crates/ekr-kernel/tests/fixtures/seed-minimal.yaml
- confidence: cited
  path: crates/ekr-kernel/tests/seed.rs
- confidence: cited
  path: crates/ekr-ontology/src/schema.rs
- confidence: cited
  path: crates/ekr-ontology/tests/ontology_load.rs
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
revision: 25
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

Cited from the implementor's stable handback and full inventory in
`.engineering/reviews/p1-08-seed-implementation.md`: kernel `commit.rs`, `lib.rs`,
`seed.rs` and `validate/mod.rs`; graph seed-reachable records in `assertion.rs`,
`canonical.rs`, `edge.rs`, `evidence.rs`, `node.rs`, `root.rs`; store `eventlog.rs`,
`lib.rs`, `log.rs`, `snapshot.rs`; kernel seed/commit tests and the minimal YAML
fixture; store provider fixtures and migrated boundary tests; the root ownership
guard in `crates/ekr/tests/story_contract.rs`; `AGENTS.md` and the coordinator-owned
`systems/ekr/domains/kernel.yaml` comments/outcome.

The opening inferred `crates/ekr-kernel/src/seed.rs`,
`crates/ekr-kernel/tests/seed.rs` and
`crates/ekr-kernel/tests/fixtures/seed-minimal.yaml` were confirmed as new files
before implementation. Their entries are now cited from the returned diff.
The exact original-to-replacement case map is retained separately at
`.engineering/reviews/p1-08-seed-case-migration.md`; source boundary tests were
replaced by kernel behavior, not silently dropped. Coordinator retains ownership
of normative comments, planning and final AGENTS wording. Independent review and
the full integration gate remain pending at this scope writeback.

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

## Seed review correction: ontology property filing

Independent executable review found that an ontology may file a property under
a map key different from PropertyDefinition.id. The public SeedDocument path
accepted the malformed ontology and reopened it through both providers. The
coordinator verified the red log and the reachable caller; schema.rs currently
iterates properties.values() and the shared loader source is unchanged from the
wave opening. This is an inherited loader defect exposed at the seed boundary.

Expand this unit to crates/ekr-ontology/src/schema.rs and its ontology_load tests.
Check map key against definition identity for node and edge property declarations
at their common load boundary, with a named typed error containing both identities.
Preserve correct declarations and legitimate inherited-property behavior.

The adversary's original case stages acceptance. Its post-fix replacement must
still submit malformed seed input against a valid compatibility ontology and
assert a seed-ontology refusal with no object or Seeded event. Separately named
loader cases assert the exact error variant. A compatibility-constructor panic,
blanket skip, or merely different-ontology refusal does not close the seed case.
The reviewer owns the independent regression; the implementor must not silently
narrow it. Full correction and re-review remain unexecuted at this scope expansion.

## Integrated acceptance

The complete repository gate passed in the coordinator checkout: 403 passed, 0 failed, 0 ignored.
Each Taskfile step (format, clippy, test, documentation, ESS and planning) has
its own successful exit status in the retained integration logs.
The source unit is 0fb7951c6d5d5803493d33d80a34c53c7070fe4d.

The independent correction review at
.engineering/reviews/p1-08-seed-correction-review.md verified the original
kernel regression unchanged (SHA-256 a3152f04d6a02c28be6dc3b4327a7f8dec3fcf1ec1177c98a7d4fba78c4920ce) and the exact ontology loader
refusals. Its prior blocker is corrected; the review's historical origin remains
undecided because no opening-commit execution established it. The earlier
source-only attribution in this story is not historical execution evidence.

The coordinator disabled only the new property-key equality check and ran that
independent kernel case. It failed with all malformed node/edge combinations
admitted through both backends (exit 101). The exact source was restored before
the full gate. See the wave closure and retained mutation log.
No post-seed durable application or object/blob migration is implied.

## Publication and main integration

Required repository correctness and shared security/privacy checks pass for
a4b21d54e3838efbf924cb60666073b2bf493b67. The bot fast-forwarded main to that exact
checked source; https://github.com/beyond10x/epistemic-knowledge-runtime/pull/8
reports merged with the same merge commit. The primary checkout is synchronized.
Gates release adoption and actual publication evidence close the delivery blocker.
The local full seed gate and adversarial correction evidence remain above.
