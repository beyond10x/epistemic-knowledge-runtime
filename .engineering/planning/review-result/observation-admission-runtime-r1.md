---
format: aep.planning-md/3
id: review-result:observation-admission-runtime-r1
kind: review-result
status: active
title: Observation application support allocation review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
# Independent knowledge/3 runtime source review

Status: HELD for the concrete source-derived correctness issue below. This is a bounded read-only audit of the knowledge/3 working changes in `ekr-schema-proposals-20261003`, not scope approval, test execution, or full F acceptance. No Cargo, source/AEP edits, publication or agents. The coordinator's running green checks were not treated as this review's evidence. Paths and line numbers are repository-relative at inspection; the coordinator is concurrently editing the working tree.

## Finding R1 — current-head allocation can persist an election that historical reconstruction refuses

Severity: high correctness/availability. Evidence is source inspection; the proposed reachable sequence has not been executed by this reviewer.

`schema_application.rs:151` captures the current graph and compares material digests (`:152`–160), then passes that current read to `schema_template` (`:198`–204). `application_support.rs:61`–83 chooses between reusing a canonical Evidence ID and minting a wrapper based on that graph. However, `application_auth.rs:583`–600 reconstructs the same template using the **initial approval's observed revision**. These two source-evidence presence sets can differ without the current review-basis comparison noticing.

Concrete sequence using valid public operations:

1. Import an interpretation with a selected fact supported by a HumanStatement evidence record whose `extracted_by` is the ordinary authenticated proposer. Do not make that source Evidence ID canonical yet. Submit/approve a source-backed additive proposal selecting the fact; leave the proposal's explicit `evidence` list empty. The source fact itself carries the support ID.
2. Through ordinary Propose/Validate/Commit, add that exact original HumanStatement Evidence record and payload to canonical state. An AddEvidence-only transaction may have an empty manifest, as ordinary evidence admission already permits. This changes neither schema nor mapping effects.
3. Apply the approved proposal. `schema_proposals.rs:324`–359 binds the immutable source documents, but its separate canonical evidence material includes **only explicit proposal.evidence**, not evidence IDs selected indirectly by source facts. With no mappings/corrections, this evidence-only canonical advancement leaves the compared material unchanged.
4. Current-head planning now reuses the original canonical ID and emits no wrapper (`application_support.rs:62`–71). Historical template reconstruction at the initial review revision sees the ID absent and requires a wrapper (`:73`–83; `application_auth.rs:526`–543). Its allocator encounters no matching wrapper and refuses with `schema support wrapper is missing` (or a related exact-template disagreement).

This is more serious than a harmless application refusal: `schema_application.rs:215`–225 writes the election first; `ekr-store/src/applications.rs:548`–589 performs physical retention and appends at `:587` without semantic template admission. Later verified history checks every retained election (`application_auth.rs:835`–842). The invalid immutable election can therefore make subsequent verified reads refuse. The authority remains fail-closed; this finding is not a demonstrated unauthorized canonical write.

Required correction: choose and document one stable support-allocation basis shared by planning and replay, and verify the complete candidate election semantically against that basis **before retention**. Using the initial approved capture consistently preserves its frozen wrapper decisions even if an identical original source record becomes canonical later; alternatively bind a distinct typed election basis and its effects explicitly. Do not fix this by ignoring missing wrappers, accepting both arbitrary plans, or dropping evidence from the manifest. A pre-retention check alone prevents corruption but should not turn unrelated evidence-only advancement into an unnecessary renewed-human-review requirement.

Required red-capable test on File and SQLite: execute the above sequence, assert application succeeds or a specifically justified refusal occurs before any election/step append, then reopen/full-replay and verify ordinary reads remain available. On the intended successful path, assert exact source kind/payload and stable repeated wrapper IDs. Also test changed-record collision under the same source ID refusing before election.

## Inspected boundaries with no additional concrete bypass found

These are source observations, not measured passes or correctness proof:

- Immutable input authentication: `application_inputs.rs:32`–47 validates actual retained observation imports; `:50`–73 fetches Provenance content by the immutable interpretation digest, checks coordinates and calls `incubation::checked_document`. That checker binds Observation evidence to declared retained source ID/hash/exact payload (`incubation.rs:54`–65). Live planning's `retained_interpretation` also calls the checker (`incubation.rs:227`–237).
- Source selection/attribution: `application_support.rs:22`–45 takes explicit proposal evidence plus selected-fact evidence, rejects competing source records, copies missing source evidence into fresh wrappers and changes only wrapper ID/extracted_by (`:73`–83). Direct observation wrappers use the real ID/hash/bytes, captured time and specified zero confidence (`:86`–114). The original interpretation bytes remain unchanged. Canonical/source disagreement refuses (`:62`–69).
- Capability containment: EvidenceAdmission's fields are private (`application_auth.rs:11`–20). The sole construction path requires knowledge/3, an exact elected attempt, verified election/template, and equality of the entire encoded transaction (`:25`–86). It admits only exact Observation additions. Provenance still hashes every payload and applies ordinary assertion rules (`validate/provenance.rs:125`–193). The profile alone does not grant admission.
- Cache separation: both command decision and replay verdict paths bypass the context-free thread-local cache when a support capability exists (`replay.rs:787`–793, `:824`–828). Guard/review verification still occurs before replaying each ordinary occurrence (`replay.rs:1151`; `application_auth.rs:775`–830). This avoids the initially identified missing authorization-input cache key.
- Historical profile boundary: the new exact knowledge/3 value has provenance/2 (`authority.rs:123`–145), while seed-supported exact profiles remain the three original values (`:190`–195). Upgrade replay still accepts the exact original knowledge/1 and /2 profiles and reconstructs their original destination previews (`upgrade.rs:113`–130, `:391`–401). `evidence_admission` returns no capability before knowledge/3. Historical hashes and pending-validation/recovery behavior still require the coordinator's actual regression evidence.
- Mapping/correction authority remains explicitly refused (`schema_application.rs:21`–27; `application_auth.rs:465`–468). This audit does not represent it as implemented. Explain is known pending work assigned to the coordinator, so its current HumanStatement-only termination is not reported here as a newly discovered blocker.

## Limits

No attempted exploit, mutation harness, provider run or historical fixture run was executed in this unit. R1 is a concrete reachable source-level hypothesis with a minimal public-operation test, not a measured failure. Review was restricted to the requested admission changes; the pending explanation implementation and full F mapping/correction execution were outside the inspected unit.
