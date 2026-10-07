---
format: aep.planning-md/3
id: dependency-blocker:atlas-lists-ekr-as-independent
kind: dependency-blocker
status: cleared
title: Atlas does not yet list this repository as documented independently
relations:
- blocks: task:remove-unified-site-manifest
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-07T02:27:45Z", actor: "agent:claude-ekr-controller", revision: 3}
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

## Cleared

On 2026-10-07 Atlas `main` `1cfb16a7dbfb` (committed 2026-10-07T01:57:14Z) lists
`epistemic-knowledge-runtime` in `INDEPENDENT_DOCUMENTATION_REPOSITORIES` and holds 8 catalog
rows for it.
