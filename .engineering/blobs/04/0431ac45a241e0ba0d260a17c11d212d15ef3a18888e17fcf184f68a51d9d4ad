---
format: aep.planning-md/1
id: task:agent-cannot-add-evidence
kind: task
status: draft
title: An agent using the CLI cannot add evidence for a new claim
relations:
- serves: vision:o5
- derived_from: story:agent-discoverable-cli
revision: 1
---
## What is wrong

Measured by blind agent trial 2 (wave p4-01): an agent using only the binary cannot add evidence for a new claim. P1 admits evidence only through the seed, no operation adds any, and the seed entry needs a `content_hash` whose algorithm the binary never states; the refusal `seed-evidence-payload-mismatch` names neither the algorithm nor the expected value. The agent cited unrelated seed evidence and validation accepted it, because provenance checks that evidence is retained, not that it supports the claim.

## What closes this

Short term: `ekr` prints the content hash of a payload (or the refusal names the expected hash) so a seed can carry new evidence. Proper route: observations as evidence through the P2 observation layer (`epic:p2-observation-layer`).
