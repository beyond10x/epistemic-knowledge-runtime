---
format: aep.planning-md/3
id: decision-blocker:ess-0-66-commit-stale-unreachable
kind: decision-blocker
status: open
title: ess 0.66 cannot reach Commit/stale without a specification change, and its suite format needs the runner crate moved
relations:
- blocks: story:ekr-on-ess-0-66
revision: 1
---
## What stops the work

U1 of wave 2026-10-21a stopped under the coordinator's decision 1 ("if the branch is unreachable by
design, the unit stops and says so"):

- `ekr.kernel.Commit/stale` is unreachable under ess 0.66.0's branch precedence: `committed` claims
  every Validated transaction (`systems/ekr/domains/kernel.yaml:1709-1710`,
  `when_subject_state: Validated`), a held-state branch ess selects before the external `stale`
  (`kernel.yaml:1744-1746`), whose move starts only from Validated (`kernel.yaml:1376-1378`). An
  authored Propose -> Validate -> Commit scenario expecting `stale` is accepted, and synthesis still
  refuses 3 `ESS-SYNTH-003` and 2 `ESS-SYNTH-004` in each of the three components (kernel 75
  scenarios, below the floor of 80).
- ess 0.66.0 writes suite format `ess-conformance/35`; the pinned runner crate (`Cargo.toml`
  `ess-conformance` rev `be44a336`, ess 0.36.0) reads up to `/19`, and
  `crates/ekr/tests/conformance.rs` `every_ess_pin_site_names_one_release` ties every pin site to
  one release. Moving the crate to 0.66.0 (`145a1b9d`) needs harness changes of unmeasured size.
- The `ESS-AUTHOR-042` rewrite drops 1,083 exact values (56 kernel, 1,027 views); the views suite
  then checks no page counts or grouping in its paging scenarios.

U1's changes stay uncommitted in tree `ekr-w21a-u1`; its scripts and logs are under
`<home>/.cache/ekr-w21a-scratch/`.

## What clears it

A decision by the operator, relayed by `b10x-chef-orchestrator`: hold on 0.36.0 (proposed
default), or approve a change to Commit's branch shape in `kernel.yaml` and the runner crate move
(sized first), or wait on an ess change to generated-field pinning or branch precedence.
