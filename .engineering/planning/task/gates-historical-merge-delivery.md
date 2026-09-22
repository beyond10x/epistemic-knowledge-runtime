---
format: aep.planning-md/1
id: task:gates-historical-merge-delivery
kind: task
status: draft
title: Repair verified historical merge delivery before publishing P1 work
relations:
- derived_from: story:kernel-validated-seed
- blocks: epic:p1-kernel-ontology-core
revision: 1
---
## Reproduced publication blocker

The integrated seed gate passes, but b10x-gates publish rejects the already
published App-created merge 250aecaa0d3598a6354d22cd7a0f86b491468424 because
GitHub is its committer. A fresh common check signed the candidate successfully.
All newly introduced commits have exact bot author and committer.

The workspace rule permits an authenticated App-created GitHub merge after exact
authority, merge-action and candidate-tree verification. Gates currently applies
the strict direct-commit check to every ancestor since adoption. The upstream
story:published-merge-delivery now owns a bounded shared publish/pre-push verifier.

Keep the adoption baseline, protections, hooks and scanners intact. Clear this
task only after the reviewed/released binary and coordinated hooks verify the
actual published merge, then EKR publication and required checks succeed.
Pure frozen-format work can continue without claiming main integration.
