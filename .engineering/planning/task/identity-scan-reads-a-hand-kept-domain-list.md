---
format: aep.planning-md/2
id: task:identity-scan-reads-a-hand-kept-domain-list
kind: task
status: draft
title: The id-type scan reads a hand-kept list of domain files
relations:
- serves: vision:o5
revision: 1
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
