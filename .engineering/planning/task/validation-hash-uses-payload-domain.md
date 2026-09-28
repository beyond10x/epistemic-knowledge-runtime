---
format: aep.planning-md/3
id: task:validation-hash-uses-payload-domain
kind: task
status: implemented
title: Hash validation receipts in the canonical value domain
relations:
- serves: vision:o2
- derived_from: story:version-persisted-contracts
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-23T00:17:30Z", actor: "agent:claude-579ad7f9-p1-12", revision: 2, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "proposed", to: "active", at: "2026-09-23T00:17:30Z", actor: "agent:claude-579ad7f9-p1-12", revision: 3, decided_on: {"recorded":{"test_result":1}}, imported: true}
- {from: "active", to: "implemented", at: "2026-09-23T00:17:30Z", actor: "agent:claude-579ad7f9-p1-12", revision: 4, decided_on: {"recorded":{"test_result":1}}, imported: true}
---
## Finding

The independent P1 core review found crates/ekr-kernel/src/transaction.rs hashes canonical transaction and validation-basis bytes with ContentHash::of_bytes, which selects the raw payload domain. The archived review_p1_validation_hash_domain.rs preserves the witness. This is separate from the already implemented canonical-newtype-discriminant task.

## Acceptance

A private Canonical wrapper over the complete validation basis is hashed with ContentHash::of. A regression proves equality with the canonical value-domain encoding and inequality with the old payload-domain hash, plus transaction/revision sensitivity and determinism. Keep the global payload/value domain labels unchanged. Bind any old-to-new validation-address mapping to explicit format dispatch; never reinterpret a retained old hash as a new one.

## Scope

Cited: crates/ekr-kernel/src/transaction.rs and crates/ekr-kernel/tests/validation.rs. Inferred: a private canonical wrapper in transaction.rs. Versioned receipt migration and full root-hash validation basis coordinate with story:commit-and-revision-lineage. This task remains open until executable evidence exists.
