# Generated knowledge contracts

The ESS specification in `systems/ekr` owns these models. `ekr-contracts` is the complete Rust
workspace synthesis; the product depends only on its `ekr-types` library, under the Cargo alias
`ekr-contracts`. `ekr-contract-data` is ESS's named-type serde library. Its public types are exposed
through `ekr_sdk::contracts` and `ekr_core::contract_data` without handwritten model copies.

Run `cargo run --locked -p xtask --bin xtask -- contracts-check` to verify the exact generator
binary, regenerate both artifact trees, compare every generated path and byte, and compile the
synthesized workspace. Missing, changed and extra artifacts fail. ESS's `.ess-output/state.json`
is a local ownership journal, containing machine paths and inode data; it is not a source
artifact and is neither committed nor compared. Unknown entries in that directory fail the gate.
Never run Cargo directly in the committed generated workspace: its lockfile and build outputs
would become unexpected artifacts. The gate compiles its disposable regeneration instead.

`ess-generator.json` is the generator provenance record. A null `release` explicitly identifies
a development candidate; the normal gate refuses it. `--development-candidate --ess <binary>`
permits development verification only and does not establish story or release acceptance. An
accepted pin must name the verified release tag, archive, archive SHA-256, executable SHA-256
and exact source revision. Generated manifests and sources must remain unedited.

The generated `PLAN.md`, `plan.json` and `types-report.json` retain all implementation obligations
and generation refusals. Generated serde parsing does not by itself establish UUID, hash, base64,
exclusive-union, time, trust-policy, cross-record or kernel validity. The checked UUID adapter in
`ekr_core::generated_identity` is one boundary check; it does not turn an arbitrary deserialized
document into an admissible transaction. No inbox, proposal or upgrade behavior is implemented by
adopting these types, and synthesis counts are not executed conformance counts.
