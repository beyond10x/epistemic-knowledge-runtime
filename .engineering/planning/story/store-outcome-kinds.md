---
format: aep.planning-md/3
id: story:store-outcome-kinds
kind: story
status: active
title: Store outcomes carry machine-readable kinds
summary: Unseeded lineage and uncertain publication get named kinds and exit codes so callers stop matching English text (review 0.0.42 finding 18)
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T16:16:50Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T16:16:51Z", actor: "human:timo", revision: 3}
---
## Outcome

A caller tells an unseeded lineage and an uncertain publication apart by a stable machine-readable
kind, not by matching English text on stderr.

## Work

Review of 0.0.42, finding 18:

- For an existing store with a newly named, unseeded tenant, `ekr head` exits 1 with "the lineage
  has no seed", while `ekr quality` returns the named `ekr.views.NotSeeded` refusal with exit 2
  (File and SQLite).
- `UnknownCommit` (publication outcome unknown) becomes a generic fault; the cortex consumer
  searches stderr for "publication outcome is unknown" to decide whether it may continue a write.
- Every command that can meet an unseeded lineage reports one named kind (`NotSeeded`) with one exit
  code; an uncertain publication reports its own named kind and exit code, in the JSON error
  envelope and on stderr.
- The existing human messages stay as they are.
- docs/cli.md lists both kinds and their exit codes.

## Acceptance

`ekr head` and `ekr quality` on an unseeded tenant, on File and on SQLite, return the same kind and
exit code; a store test double that answers `UnknownCommit` makes a write command return the
uncertain-publication kind and its exit code; the human message text of both is unchanged.
