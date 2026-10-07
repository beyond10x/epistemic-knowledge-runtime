---
format: aep.planning-md/3
id: dependency-blocker:atlas-lists-ekr-as-independent
kind: dependency-blocker
status: open
title: Atlas does not yet list this repository as documented independently
relations:
- blocks: task:remove-unified-site-manifest
revision: 1
---
## What is needed

Atlas, the organisation index, lists this
repository as documented independently: an entry in its independent-documentation roster (`INDEPENDENT_DOCUMENTATION_REPOSITORIES` in `src/docs.rs`) and the
catalog rows its documentation procedure names for that step.

## Why it blocks

`task:remove-unified-site-manifest` deletes `b10x.docs.yaml`. The documentation procedure deletes
that file only after the index lists the repository as independent; deleting it first once made the
organisation portal refuse every pull request in the organisation.

## Observed

On 2026-10-07 Atlas `main` (`5a2c04e463`, committed 2026-10-05) names nine independent
repositories and not this one, and its catalog has no row for this repository.

## Cleared when

Atlas `main` lists `epistemic-knowledge-runtime` in its independent-documentation roster (`INDEPENDENT_DOCUMENTATION_REPOSITORIES` in `src/docs.rs`).
