---
format: aep.planning-md/3
id: task:rebuilt-revision-reuses-retained-verdicts
kind: task
status: draft
title: Rebuilding a released revision reuses its retained verdicts
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
revision: 1
---
## Context

Wave ops-05 unit W made a `validate` command build one candidate view against a revision the session
holds. Its adversary found that `validate --against` a revision whose graph was released rebuilds that
graph by replaying from the seed (`crates/ekr-kernel/src/replay.rs:743`), and the replay revalidates
every retained validation: 9 candidate views and 9 edge indexes at revision 7 of 8, growing with the
history (`crates/ekr-kernel/tests/adversary_validate_once.rs`, ignored case
`a_validate_command_against_an_older_released_revision_builds_one_candidate_view`).

## Build

Rebuilding a released revision's graph for a read or a validation reuses the retained validation
records instead of rerunning the pipeline for each, where invariant 7 allows (the records are the
kernel's own sealed outcomes), or starts from the nearest checkpoint.

## Acceptance

- The ignored case passes: one candidate view for `--against` a released revision.
- Refusals and roots byte-identical (the differentials).
