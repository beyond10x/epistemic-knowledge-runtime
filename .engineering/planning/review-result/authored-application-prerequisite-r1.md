---
format: aep.planning-md/3
id: review-result:authored-application-prerequisite-r1
kind: review-result
status: active
title: Finite authored application prerequisite review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 coordinator/kernel implementor findings; 0 delegated implementor findings. The delegated author owns the three scenario sources; the coordinator owns dependency/gate integration and the separately scoped native target implementation.

Bounded independent source/log review of the uncommitted authored-application prerequisite over `61667c4fab6d0a1bcb6ceda90116ab7ad29a0efe`. Approval covers admission of the three authored scenarios, their compiled suite, and the two existing ESS dev-dependency edges. It does not approve a runtime target, full F delivery, generated conformance attainment, or the full gate.

```findings
[]
```

The acceptance sources distinguish authentic retained state from fixture expectations:

- `crates/ekr/tests/fixtures/conformance/application-scenarios/schema_proposal_requires_exact_human_approval.yaml:19` specifies invalid-signature refusal, changed-digest refusal, then exact approval. Lines 73–85 require the retained proposal/digest/basis/operator/proof digest, one review and no application receipt. The fixture inventory and author report require independently enrolled signing authority, genuinely different invalid material and a genuinely submitted proposal.
- `crates/ekr/tests/fixtures/conformance/application-scenarios/interrupted_integration_resumes_without_duplicates.yaml:5` requires an actual schema-and-first-mapping committed prefix with missing application/processing receipts and a second elected frozen mapping attempt. Lines 54–90 require actual completion, exact winning transaction/assertion references, both mapping digests, both assertions, two mappings and two derivations. Four processing receipts correctly include two immutable import-time Parked records plus two later Integrated records. The original receipts must not be filtered or rewritten to manufacture that count.
- `crates/ekr/tests/fixtures/conformance/application-scenarios/canonical_derivation_has_no_transient_dependency.yaml:5` requires a cold full-replay capture followed by detachment of only its temporary provider. Lines 25–54 check the explained assertion/link count and retained mapping, derivation, observation, evidence and content identities. The author report explicitly requires independently inventoried expected links and original document/observation bytes. This is retained-capture independence, not reopening a deleted provider or product GC.

Explicit acceptance obligations for the next native target, agreed with the coordinator during this review:

1. Fixture resolution must execute authentic setup, inspect the interrupted prefix before the timeline, and retain the elected identities independently of command outputs. Do not pre-complete the second mapping, obtain expected explanation counts by calling the command under test, or project elected-but-uncommitted templates as admitted rows.
2. The compiled approval scenario contains `configure_external_outcome` steps for refusal. Those steps must not fabricate a refusal or bypass the real command. Execute the exact fixture input and observe the actual kernel result. Invalid signature and mismatched digest are already independently supplied inputs.
3. Apply events assert result presence, not a complete nested report. Independently compare the entire returned generated report against verified retained election/receipt/commit state, including item dispositions, stop reason, remaining items and corrections. First resumed completion must report `already_complete == false`; exact repeat must report `true`. A second application call must not serve as the oracle for the first report.
4. Compare canonical occurrences, canonical head and retained operational state around every approval. Successful approval may add only its genuine review-related records; refused approval must add no occurrences. Show, Snapshot, Explain and view reads must add no occurrences. An ordinary Propose side write with unchanged head must fail these checks. Repeat Apply must preserve the entire published feed and retained review/application/receipt state, as the author report already requires. Detached reads must use only the retained capture and must not recreate or reopen the detached provider.
5. Require real native execution through the admitted runner on both providers and behavioral negative controls for inert commands, duplicate writes, report corruption and missing provenance. The source assertions and current compiler evidence do not discharge these observer obligations.

`Taskfile.yml:118` adds supported `ess conform author` plus byte comparison against `systems/ekr/conformance/application-suite.json`; it does not replace or lower any existing suite. `crates/ekr-kernel/Cargo.toml:25` adds only workspace-pinned `ess-conformance` and `ess-primitives` dev dependencies. `Cargo.lock` changes exactly those two dependency edges, with no package/version change; `crates/ekr/tests/story_contract.rs:177` keeps the dependency inventory explicit. Domain contracts are unchanged in the inspected diff.

Inspected implementor evidence: `<retained-evidence>/schema-application-kernel/authored-application/author.log` reports 3 authored scenarios, 0 refusals, `ess-conformance/18`; `validate.log` reports 9 valid files. `dependency-guard.log` reports 14 passed, 0 failed. The committed-location candidate, retained `suite.json` and independently reauthored `root-suite.json` have identical SHA256 `ef48ad329961af7b33e5ead77a0be4ab28ca1dd30583c4bea21acce84e63217a`. Suite provenance matches the author report: spec digest `2289ab6b66263f553d91e328b960af9f6934eef272c699c1b9ad967900abb2f4`, contract digest `a468fb767edcd11ae49e6758907a635f9a371e456b8690b65bee6007ab27151d`. The earlier nine compiler refusals are admission probes, not behavioral red tests.

Limits: I performed source inspection and read retained logs; I did not run Cargo, ESS, native scenarios or mutations. No native target exists in this reviewed prerequisite. Scenario timestamps are fixture timeline values, not current execution evidence. The unchanged generated SubmitSchemaProposal witness blocker remains separate; these authored cases neither discharge it nor establish a green full gate.
