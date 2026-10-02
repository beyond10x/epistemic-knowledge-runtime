---
format: aep.planning-md/3
id: task:identity-scan-reads-a-hand-kept-domain-list
kind: task
status: active
title: The id-type scan reads a hand-kept list of domain files
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 4}
---
## Defect

`crates/ekr-core/tests/identity_serde.rs` (`every_ess_id_type_exists_in_the_crate`, around line 151–165)
lists the ESS domain files it scans by hand. A domain added under `systems/ekr/domains/` is not scanned
unless somebody remembers to add it, so an id newtype declared there (`kind: newtype, of: Uuid`) and
missing from `crates/ekr-core/src/identity.rs` passes the suite.

Found by the adversary of wave p2p3p4-01 unit O (`review-result:adversary-p2p3p4-01-observe-pass-1`):
`systems/ekr/domains/observe.yaml` declares `SourceUnitId` and `SourceCheckpointId`, the scan does not
read it, and `ekr-core` declares neither. Unit I added `integrate.yaml` to the list by hand, which is the
pattern this task removes. `crates/ekr-core/tests/rename_stability.rs` keeps a second hand list of the
same types.

## Acceptance

The scan reads every `*.yaml` under `systems/ekr/domains/` (a directory listing, no hand-kept list),
and a domain file that declares a `Uuid` newtype absent from `ekr-core` makes the test fail; a test
proves that by a fixture domain declaring a missing id.

## Scoped correction

The scoper verified that observe.yaml already declares SourceUnitId and SourceCheckpointId,
but the core identity carrier and the hand-selected scans omit them. This adds Rust carriers
for existing ESS nouns, not a new domain. Cited surfaces: crates/ekr-core/src/identity.rs,
crates/ekr-core/src/lib.rs, crates/ekr-core/tests/identity_serde.rs and
crates/ekr-core/tests/rename_stability.rs. Inferred shared test helper under core/tests if needed.

Discover every domain YAML file deterministically and compare its UUID newtypes with the
typed serde/mint test inventory. A fixture domain declaring an absent identity must fail the
actual scanner. Exercise the added identities through the same serde and mint/rename cases.
Avoid a second stale manually selected domain list. These cases all assert invariants that
remain valid after the repair. Coordinate lib.rs exports with the YAML unit before merging.

## Scope

Confirmed by the input-08 identity implementor public-report.md against its candidate:
crates/ekr-core/src/identity.rs, crates/ekr-core/src/lib.rs,
crates/ekr-core/tests/identity_serde.rs, crates/ekr-core/tests/rename_stability.rs,
and crates/ekr-core/tests/support/identity_types.rs are the actual surfaces. The helper inferred
at scoping is now present and supplies the typed inventory to both serde and mint tests.
The existing observe domain declares the added identities; no domain entity or encoding changed.
The ontology specification copied into the unit is coordinator-owned shared input-wave work.

VERIFIED for the unit claim: the actual scanner's unknown-domain regression and the existing
observation-identity carrier regression both failed on the opening base and passed after the
correction. Their named cases are
an_unknown_domain_filename_with_a_missing_identity_fails_the_actual_scan and
existing_observe_identities_have_typed_contract_cases. Raw baseline and treatment logs are
first-scanner-red.log versus scanner-green.log and first-carriers-red.log versus carriers-green.log
under the identity scratch. Candidate: e2aa1b16c196fa78bd8e5ebdbd7c67be05e57773.
This verifies the focused behavior; combined-wave gates and integration remain outstanding.
