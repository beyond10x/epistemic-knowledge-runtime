# Schema evidence presentation and independent review corrections

The ESS-generated SchemaEvidenceEntry is shared by the CLI, SDK and views. Schema manifests are read at a verified revision, and later citations are excluded. The Rust-rendered inbox displays their retained evidence. Empty support lists are omitted for earlier stores.

Verification collected by the coordinator:

- cargo test --locked -p ekr-views -p ekr-sdk: 365 passed, 0 failed, 4 ignored across the reported targets.
- cargo test --locked -p ekr --lib: 65 passed, including the named presentation case across both providers and normal/full replay.
- contracts-check: both complete generated artifact trees match fresh released ESS output; the synthesized workspace compiles.
- clippy over views, SDK and CLI targets passed before the later authority-history correction; subsequent correction checks are recorded separately.
- Headless Chromium rendered the file-provider fixture. The retained screenshot was inspected: schema history cards, revision/transaction identities and expandable evidence sections are present. The fragment-only screenshot attempt was blank; the full-page screenshot retained here rendered correctly. Escaping and both providers are held by the executable presentation case.

Independent review records live in the integration planning store: schema-evidence-kernel-independent-r1, schema-evidence-presentation-independent-r2, schema-evidence-authority-fix-independent-r3. They are source/log reviews, not independent test executions.

The first review identified a masked mixed-data test. Its manifest is now empty and the specific refusal is required. Both provider cases fail when only the knowledge/2 mixed-data guard is disabled, then pass after byte-exact restoration (review-guard-mutant.log and review-correction-restored.log). No mutant source remains.

The second review found that a second authority upgrade discarded the historical knowledge/1 boundary used by explanations. The native fixture regression reproduces validation-profile-disagrees, then passes after retaining every verified authority boundary and selecting the last boundary at or before the validation revision. It checks the explicit knowledge/1 profile in current and historical explanations on both providers, with normal/full replay. The final bounded independent review approved this correction.

This checkpoint does not complete story D or PR64: the authored ESS schema-evidence scenario, complete integrated gate and remaining A–F delivery are still required. The earlier --bin presentation invocation ran no tests and is not used as evidence.

After the authority-history fix, schema evidence, authority upgrades, explanations, replay checkpoints and verified reads passed 44 tests with 0 failures and 0 ignored. Clippy over all kernel, views, SDK and CLI targets passed with warnings denied. These focused checks still do not substitute for the full integrated gate.

Specification validation and regeneration of all committed suites passed. Source digest: 76cac94520a78179089e10b0871c7979eacd35197c8e7b584616fac60d49e705. Contract digest: c0793b41517a2b54756a5ae9c6d0c85aa53885defe0f88328a27a2c46c880d14. No conformance floor was lowered.
