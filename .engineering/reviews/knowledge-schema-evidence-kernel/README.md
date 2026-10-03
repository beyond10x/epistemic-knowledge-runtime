# Schema evidence kernel checkpoint

Scope: kernel admission, immutable schema support history and versioned authority transition for
story:schema-transaction-cites-evidence. This is a partial implementation checkpoint, not completed
D, independent review or PR64 acceptance.

`red-behavior.log` is the original two-provider failing run: retained evidence in a schema
transaction was refused as evidence-set-mismatch. `kernel-checkpoint.log` reports the actual
target results for add_evidence, attention_explanations, authority_upgrade, schema_evidence,
schema_evolution and schema_evolution_replay: 42 passed, zero failed or ignored.

The new schema_evidence target admits retained and inline supporting evidence, checks the exact
manifest in kernel schema history, and reopens/full-replays both providers. Unknown support,
uncited inline evidence, mismatched payloads and a mixed data operation are refused without
changing the canonical head. Existing historical schema-profile tests also pass.

`legacy-writer-verified.log` records the old implementation actually writing knowledge/1 native
stores. The retained test fixtures include a rejected schema-evidence transaction and a validated
pending ordinary transaction. `legacy-read.log` records the new implementation opening those
stores, upgrading from the active knowledge/1 predecessor, invalidating that pending validation,
committing a supported schema change, and preserving original roots and the rejection through
normal reopening and full replay. Exact transition retry is unchanged after further advancement.
The fixture README documents the capture and the discarded stale-library attempt.

`clippy-kernel.log` is `cargo clippy --locked -p ekr-kernel --all-targets -- -D warnings`, exit zero.
Product formatting and specification/conformance freshness were checked separately. Fresh
released ESS synthesis and data generation are byte-identical to the committed generated trees;
the specification changes here are contract comments, not model shapes. No generated file was
hand edited and no conformance floor changed.

The typed SDK builder, ontology CLI/viewer history, named real ESS schema-evidence scenario,
additional upgrade adversarial review and full integrated gate remain required. E–F, demonstrations
and the existing component conformance gaps also remain open.
