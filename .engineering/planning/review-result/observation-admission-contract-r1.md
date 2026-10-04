---
format: aep.planning-md/3
id: review-result:observation-admission-contract-r1
kind: review-result
status: active
title: Knowledge three exact application support contract review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 findings, 0 coordinator, 0 delegated implementor.

Bounded independent specification/design review of the uncommitted knowledge/3 amendment over e2b3c4992. Inspected systems/ekr/domains/kernel.yaml, systems/ekr/domains/integrate.yaml, design section 105.18 and `<retained-evidence>/schema-application-kernel/observation-admission-scope.md`. This is source review only: no source/AEP edits, Cargo, ESS validation, generation, tests, planted mutation, agents or publication. The ESS hardening design-review guidance was used within this assigned scope; no tool-produced verification result is claimed.

No concrete contract gap found in the proposed boundary:

- `docs/epistemic-knowledge-runtime-design.md:6063` defines the complete knowledge/3 ruleset/application/provenance tuple, reviewed transition eligibility, original-profile historical replay and pending-validation invalidation. The profile declarations in `systems/ekr/domains/kernel.yaml:471` and `:487` identify the same boundary. The version field alone cannot enable admission.
- Design `:6070` permits only exact Observation support reconstructed for an authenticated elected application. Ordinary and seed AddEvidence remain HumanStatement-only, other source kinds remain refused, all validators still run, and cached validation must bind the verified support context or be bypassed. Kernel `:1077` expressly carries that application-only exception rather than silently changing older provenance rules.
- Design `:6080` selects explicit proposal support, evidence of the proposal's selected source facts, direct proposal observations and the human statement. It does not authorize admitting every evidence record in a referenced document. Conflicting original or canonical identities refuse. Integrate `:1831` incorporates this support rule into ApplySchemaProposal.
- Design `:6088` distinguishes immutable source records from newly attributed wrappers. It specifies stable elected IDs, exact copied source fields, the authenticated proposer, deterministic wrapper ordering, direct-observation coordinates and zero confidence, and preservation across retries/Stale successors. Canonical reuse requires complete source-record equality and emits no addition. Design `:6100` reconstructs the source-to-wrapper correspondence from the immutable selection and elected additions, constrains the exact manifest and keeps mapping/derivation references tied to actual admitted IDs.
- Design `:6108` and kernel `:2492` require independently verified observation records and payloads for explanation. Historical/cold/warm replay cannot rely on a live incubation root or on a caller-mutated graph/capture. The existing Evidence link is explicitly selected as the public projection; no additional persistent carrier is assumed.

The named cases at design `:6116` are explicitly unexecuted obligations. They cover positive reviewed admission, old-profile and unsigned refusal, wrapper identity/attribution, selection and collisions, explanation without live source roots, and cold/warm tamper refusal. Their eventual implementation must also retain the stated ordinary validation, review-revocation and immutable-preparation recovery requirements. Merely validating or regenerating ESS cannot establish these outcomes.

Limits: approval is for this proposed admission contract, not implementation, complete executable conformance coverage, full F or PR64 delivery. No claim is made that the named provider cases currently exist or pass. The scope report's source observations were treated as implementation context, not independently executed evidence. The rest of design 105 and the already accepted application protocol were not re-reviewed. No runtime cache, provenance, attribution or replay implementation is approved by this report.

```findings
[]
```
