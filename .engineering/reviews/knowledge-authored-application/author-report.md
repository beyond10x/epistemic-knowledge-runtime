# Finite authored E/F application scenarios

## 1. Scope and result

Authored admission only, not runtime conformance. Owners: this unit owns the three new scenario sources; the coordinator owns the native kernel target, dependency and gate changes, runtime execution, review and commit. Base commit: `e55fdd655559d052273cda4669b4437da00b5b8b`.

The scoped patch adds exactly three `ess-scenario/3` YAML files under `crates/ekr/tests/fixtures/conformance/application-scenarios/`. No existing source, domain declaration, generated committed artifact, Cargo file, gate or planning record changed. No Cargo invocation or remote write occurred.

## 2. Authored contract and fixture inventory

`fixture-inventory.json` records every fixture name and exact ESS type, grouped by admitted scenario ID. Only finite proof/basis, scalar, UUID and scalar-list fixtures are used; no recursive document fixture and no JSON widening.

- `ekr.integrate/authored/schema-proposal-requires-exact-human-approval`: two refused approvals (corrupted signature, changed proposal digest), one exact signed approval, proposal read and current snapshot. Positive review rows assert exact proposal/digest/basis/operator/proof digest; exactly one review, no application receipt, unchanged current canonical revision/root. `HumanReviewer` approves; `KnowledgeProposer` reads; `kernel.Operator` snapshots. The fixture's proposal is genuinely submitted before the timeline under independently enrolled authority, without prior approval. Both proof attempts use the same exact statement fixture. `wrong-proposal-digest` must differ from the retained digest; `invalid-signature-proof` must differ only in invalid signature material. `reviewer` is derived from the pinned policy, not caller content.
- `ekr.integrate/authored/interrupted-integration-resumes-without-duplicates`: Apply, Show and exact repeated Apply under `KnowledgeProposer`. The setup reconstructs authentic schema plus first-mapping commits, elects both authentic mapping steps/attempts, and omits processing/application receipts. The second frozen attempt remains Proposed-capable. The fixture source has two facts, `facts[0]` and `facts[1]`, one evidence citation each. Existing immutable source import receipts remain present. The assertions require a Complete application receipt, no remaining items/pending corrections, both exact integrated transaction/assertion references, both mapping digests and source document digest, two admitted mappings, two derivations and both assertions. Total ProcessingReceiptRecords is **four**, including the two immutable import-time Parked receipts and two Integrated receipts. `first-assertions` and `second-assertions` are singleton lists equal to their matching scalar assertion fixture. Assertion/transaction IDs come from genuinely elected frozen templates, never predictions from names. `schema-revision` is the reconstructed actual schema commit revision.
- `ekr.kernel/authored/canonical-derivation-has-no-transient-dependency`: `kernel.Operator` executes Explain on an actual cold full-replay capture after only the fixture's temporary provider is detached. Event assertions use the declared assertion ID and link count. Existing mapping, derivation and evidence views assert exact retained IDs and source/proposal/mapping/content digests, with exactly one mapping and derivation. Setup must elect only one selected mapping with one support, even if its source document contains other facts. `mapping-evidence` is exactly the singleton `evidence-id`; `observation-id` is the actual independently retained observation. Derivation IDs are minted UUIDs, not derived labels. `explanation-link-count` must come from independently inventoried expected retained links, not the result of the command under test.

## 3. Authoritative observations and remaining target obligations

Every row must project actual verified retained records. Review refusal must compare the retained review/proposal stream before and after each call, not just count final visible rows. All normal reads reopen with full replay. Mapping/derivation rows represent admitted commits, never elected but uncommitted templates. Do not filter out immutable parked receipts to make the authored count pass.

ESS 0.52 does not admit a partial nested ApplicationReport literal. Consequently the Apply events assert actual result-event presence, while flat retained view assertions assert completion and exact materialized records. The target's independent report/retention checks must additionally enforce:

- First resumed response is the completed actual application with `already_complete == false`; repeat returns the same application with `already_complete == true`.
- Repeated Apply changes neither the entire published occurrence feed nor retained review/application/receipt state. Same canonical head alone is insufficient.
- Every Integrated receipt's basis digest is its actual ordinary commit record hash; schema and each mapping have only their real winning committed transactions.
- Resume preserves original schema/first-fact identities and checks the genuinely interrupted prefix before running the timeline.

The detached Explain target must compare capture content to independently supplied original observation/document bytes and verify their digests. It must neither open the detached store nor invent a product deletion event. Explain result links provide mapping/derivation rows; verified graph evidence provides the evidence row. This proves retained capture independence, not reopening a deleted provider or implementing GC.

These are explicitly adapter integrity obligations, not hidden assertions claimed to appear in the suite. Root should run inert-command, duplicate-write and missing-provenance controls through the admitted runner. Retained-history corruption/deletion checks remain separately executed authority controls. Native test names are not aliases for authored execution.

## 4. Admission evidence

Released ESS `0.52.0`, binary SHA256 `7c0f35fd2c2365c2390113f756eba1275ea25902d04f7ca5a8f8f6ab30395be7`.

Commands from the unit checkout:

```console
ess specify validate --path systems/ekr
ess specify compile --path systems/ekr --format json
ess conform author --path systems/ekr --scenarios crates/ekr/tests/fixtures/conformance/application-scenarios --out <evidence>/suite.json
```

Observed: validation exited 0, `ekr v1 — 9 file(s), valid`; compile exited 0. Final authored compilation exited 0, `3 authored scenario(s) from 3 file(s), 0 refusal(s), suite ess-conformance/18`. The suite contains exactly the three IDs in section 2. Authoring is the supported compilation route for these authored scenarios; this unit does not regenerate or replace the obligated generated suite.

Final provenance: spec digest `2289ab6b66263f553d91e328b960af9f6934eef272c699c1b9ad967900abb2f4`; contract digest `a468fb767edcd11ae49e6758907a635f9a371e456b8690b65bee6007ab27151d`. Suite SHA256 `ef48ad329961af7b33e5ead77a0be4ab28ca1dd30583c4bea21acce84e63217a`.

`author-probe.log` and `suite-probe.json` preserve the first compiler refusal: nine diagnostics, partial nested receipt fields (`ESS-AUTHOR-014`) and fixture mappings nested inside scalar lists (`ESS-AUTHOR-015`). The final source uses whole typed list fixtures and flat positive view assertions. Those red probes are **compiler admission evidence**, not runtime negative tests. Raw `validate.log`, `compile.log`, `model.json`, `author.log` and final `suite.json` are retained outside source.

## 5. Limits and unchanged blockers

Runtime conformance has not run. The authored sources have not been exercised on File or SQLite. Full replay, nonduplication, proof verification, detached explanation and negative controls remain the native target's execution work. Full `task check` is not claimed.

ESS issue 416 remains open for the generated recursive-fixture witness. These finite authored cases neither discharge that generated obligation nor lower generated execution floors. No entities or wire names changed and no new `UNMAPPED` model decision was introduced. Target implementation and final suite/spec digest reconciliation remain required before acceptance.

## 6. Handoff and recovery

Branch: `ekr/authored-application-20261004`; managed tree ID: `ekr-authored-application-20261004`. Sources are uncommitted and handed to the coordinator for independent review and bot commit. No tree removal is requested. The worker releases only its own lease after preserving the scoped patch and archive.

Deliverables beside this report: `source.patch`, `fixture-inventory.json`, `suite.json`, compiler logs and the original refusal probe. Scoped patch SHA256: `27dc8d10a5a0107b3eebeceb731d56b01d66319ed6a98d215e9dbb3320069f9f`.
