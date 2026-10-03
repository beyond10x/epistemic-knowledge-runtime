# Knowledge contract adoption

The operator's 2026-10-03 execution request authorizes implementation of this scoped prerequisite. AEP implementing skill 0.19.1. Unit story:adopt-generated-knowledge-contracts serves vision:o2. Stage: accepted, preparing dispatch. No feature implementation story is complete.

The active-story waves command selects exactly story:adopt-generated-knowledge-contracts, inferred scope. Collisions [], unassessed [], cycles []. Its declared paths are .github/workflows/ci.yml, Cargo.toml, Taskfile.yml, crates/ekr-core, crates/ekr-integrate, crates/ekr-sdk, crates/ekr-store, crates/ekr-views, generated, systems/ekr and xtask. The single unit serializes all shared product/gate edits; upstream ESS work occurs independently in its own repository. Coordinator owns every AEP write.

Integration branch feat/knowledge-inbox-schema; managed integration tree ekr-knowledge-inbox-20261003. Unit branch and managed tree ekr-contract-adoption-20261003 will be recorded at creation. Build directory is unit target/ (bounded jobs=2, incremental off, debug disabled). Scratch <cache>/ekr-knowledge-prereq-20261003/adoption. Keep caches and temporary generators separate from byte-owned generated output.

Source contract head 195edf5d7d adds typed signed human proofs to 07604bf1a. Final source spec_digest 6e6b51b6bd6e58fd549ea5d5d5562997e603f9e0012f19b9285bfadec393cabc; contract_digest a0934f66cde7acd61a5a40778b7848f6896e36e413fcc19560b8f9a78863f051. ESS0.51 validates nine files and synthesizes115 scenarios (zero authored); this is not runtime conformance.

Development generation may use the exact upstream compiling candidate, initially09df87a612, recording its provenance. Upstream adversary found a bare carriage-return comment defect after ordinary multiline fixtures passed; its fix and review are in progress. No candidate is called released. Final acceptance and release-plan clearance still require published ESS release/artifacts, final regeneration, exact version/checksum/revision pins and the complete gate.

Implementation scope: generated semantic workspace plus generated serde data library, legitimate generated-ID/dependency guards, existing wire-alias readers, Rust/clap regeneration drift check and tool/conformance pins. Red-first tests must catch modified/missing/extra generated files, not merely compare the helper with itself. No hand-copied domain models, false HTTP components, softened conformance execution baseline, or ignored report obligations.

Commits will record generated contracts, adapters/guards, drift gate and actual verification evidence. Root integrates and publishes bot commits after review; no worktree is retired without published recovery proof.
