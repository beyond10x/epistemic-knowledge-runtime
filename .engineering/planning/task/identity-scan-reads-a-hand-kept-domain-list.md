---
format: aep.planning-md/3
id: task:identity-scan-reads-a-hand-kept-domain-list
kind: task
status: implemented
title: The id-type scan reads a hand-kept list of domain files
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T18:02:03Z", actor: "agent:codex-ekr-next-three-root", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-02T21:12:11Z", actor: "agent:codex-ekr-next-three-root", revision: 8, decided_on: {"recorded":{"test_result":2,"verification":2}}}
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

Confirmed by the identity public-report.md and adversary public-report.md:
crates/ekr-core/src/identity.rs and crates/ekr-core/src/lib.rs add/export the existing observation
identity carriers; crates/ekr-core/tests/identity_serde.rs discovers domain YAML and checks the
typed UUID inventory; crates/ekr-core/tests/rename_stability.rs shares that inventory through
crates/ekr-core/tests/support/identity_types.rs. The earlier inferred helper now exists and the
hand-kept domain lists are removed. The ESS observation domain already declares these identities;
no domain entity or encoding changed. The ontology specification copied into the unit is
coordinator-owned shared input-wave work.

VERIFIED for the focused claim from the retained first-scanner-red.log versus scanner-green.log
and first-carriers-red.log versus carriers-green.log: an_unknown_domain_filename_with_a_missing_identity_fails_the_actual_scan
and existing_observe_identities_have_typed_contract_cases failed before the correction and pass
after it. The final adversary adds renamed-domain, alternate-YAML, removed-declaration and
canonical-identity cases. Its immutable review record and retained report supply the results.

## Final combined verification

The earlier pending paragraphs record intermediate states; this section supersedes their
admission status. Final reviewed unit heads: YAML eb807fd7e70c73b388d805f733874300170a66f6 and identity 14e43ddd7c4949212aab25dd01e257844641e624.
They are merged in combined source candidate 5ab56194a7ef25965ffc5d4eb790080e235f1708.
All task-check steps applicable there exited zero, recorded individually under
<cache>/ekr-next-three/coordinator/input-08-gate. This includes workspace tests and bench-feature
compilation, format, clippy, rustdoc, vendor checks, ESS validation, conformance freshness and
planning validation. The existing ignored cases include measurements, subprocess helpers and
known defects; this wave changes no ignore annotation and claims none of those defects fixed.

Current-main documentation was subsequently merged at 401a56d43e4b617a078a0dda7b384c4f7a1ef908. Supplementary format,
workspace all-target clippy, cross-crate story_contract/public_surface/docs_cli/agent_cli/
temporal_reads, xtask tests, YAML ingress inventory, rustdoc and the newly added site-check all
exited zero under <cache>/ekr-next-three/coordinator/input-08-supplement. Required CI will run
the complete current task check on the published candidate. Source gate logs remain retained.

The focused baseline/treatment claim is VERIFIED by the previously cited red and green logs.
Final adversary records are review-result:adversary-input-08-yaml-pass-2 and
review-result:adversary-input-08-identity-pass-1. They report no remaining findings. The YAML
pass-one inventory finding is corrected and has its fixed review_outcome. No further attack ran.
Public integration and cleanup remain coordinator responsibilities before reporting the release
checkpoint ready. No version change or tag is part of these implementation moves.

Measured test-result totals below sum report lines (including subprocess executions); they are
executions, not unique test identities:
- Combined workspace log: 2138 passed, 0 failed, 14 ignored.
- Vendor log: 183 passed, 0 failed, 0 ignored.
- Supplementary test logs: 98 passed, 0 failed, 0 ignored.
