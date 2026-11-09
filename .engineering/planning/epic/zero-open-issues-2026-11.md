---
format: aep.planning-md/3
id: epic:zero-open-issues-2026-11
kind: epic
status: draft
title: Every open GitHub issue and every open defect in the store is fixed or blocked by name
summary: 'One epic, one pull request: GitHub issues #96 and #104 and the store''s open defects.'
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- serves: vision:o2
- serves: vision:o5
revision: 1
---
## Outcome

The repository has no open GitHub issue, and every non-terminal artifact in this store that
describes wrong behaviour is either fixed by a merged change or stopped by a typed blocker that
names what would clear it.

Operator's goal, 2026-11-09, verbatim: "zero remaining issues on github, all bugs fixed from
planning store". Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but
in the end provide 1 single PR to gh. make sure all are below 1 big epic for integration via
bottoken from ./atlas".

## What is under it

Read from `gh issue list` and from the store at `origin/main` `57244640` on 2026-11-09.

GitHub issues (2 open):

- #96 — `story:view-historical-changes-and-cancelled-reads`
- #104 — `story:search-live-typing-waits-on-the-page`

Defects already in the store (non-terminal, describing wrong behaviour):

- `task:migration-marker-cannot-be-evidence`
- `task:seed-authority-refusal-cli-and-kernel-disagree`
- `task:streamed-page-shows-far-valid-times`
- `task:sdk-consumer-yaml-dependency`
- `task:no-test-asserts-wall-clock-cost`
- `task:seed-race-test-names-its-signal`
- `task:held-file-payload-overwrite`
- `story:explain-names-the-merge`
- `story:changes-since-by-time-within-deadline`

Defects that code in this repository cannot resolve today; each keeps or gets a typed blocker:

- `task:replaced-store-close-keeps-the-restored-file` — `dependency-blocker:eventlog-close-without-checkpoint` (an upstream Eventlog release).
- `story:supersession-verdict-fixes-replay-safe`, and rows H6 and H7 of
  `story:merge-entity-review-follow-ups` — the rules-era architecture decision the story's Build
  step 1 reserves for the operator.

## Not under it (the coordinator's reading, not a measurement)

Artifacts that describe cost without a wrong answer (`task:eventlog-rehash-and-fsync-per-write`,
`task:eventlog-statements-prepared-once`, `task:store-open-verifies-blobs-once`,
`task:ontology-at-reads-only-the-schema`, `task:selected-revision-loads-read-one-event-per-call`,
`task:validate-cost-flat-with-store-size`, rows H8 to H12 of
`story:merge-entity-review-follow-ups`), the two conformance-suite gaps that their own bodies call
upstream ESS questions (`task:conformance-cannot-observe-command-responses`,
`task:current-revision-ranking-is-unsynthesizable`), and every artifact that describes a capability
not yet built.

## Acceptance

- `gh issue list -R beyond10x/epistemic-knowledge-runtime --state open` prints nothing, or only
  issues an open typed blocker in this store names.
- Every artifact listed under "Defects already in the store" is `implemented` on evidence from the
  one pull request, or is named by an open typed blocker.
- One pull request holds every wave. No release is cut.
