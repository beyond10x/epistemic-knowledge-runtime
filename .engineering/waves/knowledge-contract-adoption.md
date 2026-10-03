# Knowledge contract adoption

The operator's 2026-10-03 execution request authorizes implementation of this scoped prerequisite. AEP implementing skill 0.19.1. Unit story:adopt-generated-knowledge-contracts serves vision:o2. Stage: ESS 0.52.0 pinned and regenerated; full conformance bindings and complete gate remain open. No feature implementation story is complete.

The active-story waves command selected story:adopt-generated-knowledge-contracts with no collisions, unassessed stories or cycles. Its current typed scope corrects the inferred CI path to .github/workflows/correctness.yml and includes README.md and the CLI story-contract/conformance guards. The story is the authoritative path inventory. The single unit serializes all shared product/gate edits; upstream ESS work occurs independently in its own repository. Coordinator owns every AEP write.

Integration branch feat/knowledge-inbox-schema; managed integration tree ekr-knowledge-inbox-20261003. Unit branch ekr/contract-adoption-20261003; managed unit tree ekr-contract-adoption-20261003 based on 08b8a7f2ea5d551e4bf0da7912e85c648efc3b46. Build directory is unit target/ (bounded jobs=2, incremental off, debug disabled). Scratch <cache>/ekr-knowledge-prereq-20261003/adoption. Keep caches and temporary generators separate from byte-owned generated output. The coordinator resumed the worker's dirty source and running verification after its execution allowance was exhausted; this continuation is not independent review.

Source contract head 195edf5d7d adds typed signed human proofs to 07604bf1a. Final source spec_digest 6e6b51b6bd6e58fd549ea5d5d5562997e603f9e0012f19b9285bfadec393cabc; contract_digest a0934f66cde7acd61a5a40778b7848f6896e36e413fcc19560b8f9a78863f051. ESS0.51 validates nine files and synthesizes115 scenarios (zero authored); this is not runtime conformance.

Development generation uses upstream candidate 126c2b3905d0f4279086b9d3030096147955dfec, with executable digest retained in generated/ess-generator.json. The upstream adversary's bare carriage-return comment case now passes unchanged after its fix. The release integrator reports that ESS pull request 398 preserved the frozen source and superseded the now-closed 403; 398 and 404 are merged. Source validation of e68684ef is green, but publication remains held by the operator identity decision and Gates' delivery-ancestry refusal. No candidate is called released, and EKR does not bypass that refusal or control ESS release refs. Final acceptance and release-plan clearance still require published ESS release/artifacts, final regeneration, exact version/checksum/revision pins and the complete gate.

Implementation scope: generated semantic workspace plus generated serde data library, legitimate generated-ID/dependency guards, existing wire-alias readers, Rust/clap regeneration drift check and tool/conformance pins. Red-first tests must catch modified/missing/extra generated files, not merely compare the helper with itself. No hand-copied domain models, false HTTP components, softened conformance execution baseline, or ignored report obligations.

Commits will record generated contracts, adapters/guards, drift gate and actual verification evidence. Root integrates and publishes bot commits after review; no worktree is retired without published recovery proof.

## Released generator adoption checkpoint (2026-10-03)

The coordinator resumed this owned tree at integration 1929510ed53cbfc74807a6d13d489f39d0f444e5 after preserving and reconciling duplicate synchronized edits. Builds use the owned tmpfs target, one compiler job, no debug information or incremental compilation.

Verified ESS 0.52.0 at 4d6a4ecafc0feb4e11e4bee777b19c7351fa3647, the released Linux archive and executable digests, and exact release workflows. Both generated trees pass exact regeneration and synthesis compilation without the development-candidate flag. The Cargo libraries, lock, CI archive, local guards, README and conformance provenance now name that release. The old ESS publication hold above is historical; this task made no ESS release changes.

Typed timestamp adapters preserve millisecond validation and old answer-record correction spellings during full replay; the regression fails when the compatibility comparison is removed. Public observation construction now takes the generated timestamp type. Existing authored scenarios also require controls that retain their explicitly prepared fixtures; the corrected target observes real handler results.

The current complete inventories require 72 kernel, 78 views and 18 integrate scenarios per provider, with zero unavailable allowed. All original kernel and extraction scenarios pass; ten new kernel and sixteen new integrate command branches remain unsupported in those component targets. All 78 views scenarios pass on both providers. No floor was lowered or new scenario removed to hide these gaps. Named A/B/C real and inert-target conformance remains green. See .engineering/reviews/knowledge-contracts-052 for actual reports, release metadata and test logs.

This is a local implementation checkpoint, not independent review, completed adoption, feature acceptance or the full task check. Finish the new component bindings, D–F, both demonstrations, independent review and complete gates before undrafting PR64.
