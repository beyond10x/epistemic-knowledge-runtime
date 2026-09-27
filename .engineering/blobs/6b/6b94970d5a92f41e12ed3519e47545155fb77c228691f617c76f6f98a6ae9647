---
format: aep.planning-md/1
id: task:readme-status-case
kind: task
status: implemented
title: Harden the README status case to compare against the member list
relations:
- informed_by: review-result:adversary-skeleton-pass-2
- derived_from: story:workspace-crate-skeleton
- derived_from: story:source-guard-debt
- serves: vision:o2
revision: 4
---
## Context

Adversary pass 2 of wave p1-01 added `crates/ekr/tests/adversary_docs_contract.rs`. Its case `the_readme_status_matches_the_workspace_members` computes the member list and then asserts only the absence of three hard-coded stale phrases; a README with no Status section passes. Found by the implementor in correction round 2, left standing because a case an implementor thinks is wrong is a finding for a person, not an edit.

## Acceptance

The case fails when README § Status names a member the workspace lacks or omits one it has, and passes on the current README.

## Scope

- `crates/ekr/tests/adversary_docs_contract.rs`
