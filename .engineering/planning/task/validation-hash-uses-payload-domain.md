---
format: aep.planning-md/1
id: task:validation-hash-uses-payload-domain
kind: task
status: implemented
title: Hash validation receipts in the canonical value domain
relations:
- blocks: story:commit-and-revision-lineage
- serves: vision:o2
- derived_from: story:version-persisted-contracts
revision: 4
---
## Finding

The independent P1 core review found crates/ekr-kernel/src/transaction.rs hashes canonical transaction and validation-basis bytes with ContentHash::of_bytes, which selects the raw payload domain. The archived review_p1_validation_hash_domain.rs preserves the witness. This is separate from the already implemented canonical-newtype-discriminant task.

## Acceptance

A private Canonical wrapper over the complete validation basis is hashed with ContentHash::of. A regression proves equality with the canonical value-domain encoding and inequality with the old payload-domain hash, plus transaction/revision sensitivity and determinism. Keep the global payload/value domain labels unchanged. Bind any old-to-new validation-address mapping to explicit format dispatch; never reinterpret a retained old hash as a new one.

## Scope

Cited: crates/ekr-kernel/src/transaction.rs and crates/ekr-kernel/tests/validation.rs. Inferred: a private canonical wrapper in transaction.rs. Versioned receipt migration and full root-hash validation basis coordinate with story:commit-and-revision-lineage. This task remains open until executable evidence exists.
