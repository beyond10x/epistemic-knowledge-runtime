---
format: aep.planning-md/3
id: task:historical-projection-carries-the-head
kind: task
status: active
title: A projection of a past revision changes bytes when the head moves, against the views determinism rule
summary: 'Medium: meta.head is emitted in every views format while rule (5) forbids a field that differs between renders of one revision'
tags:
- review-2026-09-28
- severity-medium
relations:
- serves: vision:o5
- derived_from: story:graph-projection-renderer
- derived_from: story:view-streams-overview-and-expansion
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T18:16:21Z", actor: "agent:claude-coordinator", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T18:56:02Z", actor: "agent:claude-coordinator", revision: 3}
---
## Severity

Medium (the review's rating, kept). A cached or exported projection of a past revision stops
matching a fresh render of the same revision after any commit, so a content hash of a projection
does not identify the revision it projects.

## The review (external review of EKR, 2026-09-28, verbatim)

> Medium — historical projections violate the stated determinism contract. Rendering revision 0
> before and after an unrelated commit produces different bytes and hashes because the projection
> includes the current head. `crates/ekr-views/src/lib.rs:167`

## What was verified (code reading at `wave/p2p3p4-06` 69e5b994; not run)

- **Verified, the contract**: the `ekr.views` domain says two renders of one revision are
  byte-identical, and rule (5) says no field carries "anything else that differs between two
  renders of one revision" (`systems/ekr/domains/views.yaml:17-18`, `:36-37`). The
  projection outcome's summary repeats it: "byte-identical to every other render of that revision"
  (`views.yaml:962`). The renderer story's exit criterion is the same
  (story:graph-projection-renderer, "two renders of the same revision are byte-identical").
- **Verified, the field**: the same domain declares `ProjectionMeta.head`, "the store's newest
  committed revision when the projection was rendered" (`views.yaml:92-103`). `load` records the
  head at load time (`crates/ekr-views/src/lib.rs:164`, `:185-190`) and `render` writes it into
  `meta.head` (`crates/ekr-views/src/document.rs:561`). The specification contradicts itself; the
  code follows the field.
- **Verified, the same shape in the bounded formats**: `OverviewMeta`, `SliceMeta`, `DetailMeta` and
  `MatchesMeta` also carry `head` (`views.yaml:622-631`, `:699-706`, `:781-788`, `:849-856`) under the same rule (5)
  (`views.yaml:39-41`). Those formats belong to story:view-streams-overview-and-expansion, now in
  flight.
- **Verified, why no test caught it**: `crates/ekr-views/tests/determinism.rs:75-96` renders one
  revision three times with the head fixed at 5; no case commits between two renders.
- **Inferred**: the byte difference itself. It follows from `meta.head` being emitted; I did not
  render.

## Reproduce

Render revision 0 of a store with `ekr_views::project`, commit one unrelated transaction, render
revision 0 again, and compare the bytes or their SHA-256.

## Acceptance

Rendering revision N before and after a later commit yields byte-identical
`ekr.graph-projection/1` documents, and likewise for the four bounded formats at revision N, on
both providers; or, if `head` stays, `views.yaml` states determinism as a function of revision and
head, and the story's exit criterion is amended to match. Either way `ess specify validate` passes
and a test in `crates/ekr-views/tests/determinism.rs` that commits between two renders fails
before the change and passes after it.
