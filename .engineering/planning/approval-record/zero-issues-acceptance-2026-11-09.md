---
format: aep.planning-md/3
id: approval-record:zero-issues-acceptance-2026-11-09
kind: approval-record
status: draft
title: Work under epic:zero-open-issues-2026-11 accepted with no operator present
tags:
- non-interactive
relations:
- decides: story:view-historical-changes-and-cancelled-reads
- decides: story:search-live-typing-waits-on-the-page
- decides: story:explain-names-the-merge
- decides: task:migration-marker-cannot-be-evidence
- decides: task:seed-authority-refusal-cli-and-kernel-disagree
- decides: task:streamed-page-shows-far-valid-times
- decides: task:sdk-consumer-yaml-dependency
- decides: task:no-test-asserts-wall-clock-cost
- decides: task:seed-race-test-names-its-signal
- decides: task:held-file-payload-overwrite
- decides: epic:zero-open-issues-2026-11
- decides: story:changes-since-by-time-within-deadline
revision: 1
---
## Which stop

The acceptance of the work under `epic:zero-open-issues-2026-11`: moving the epic, four stories
and seven tasks from `draft` to the accepted status (`active`). Accepting a story is the
operator's act; a coordinator never does it on its own judgement.

## Why non-interactive

This session is a sub-agent dispatch on 2026-11-09 with no operator turn; a Stop hook holds it
until the goal is met. The brief it runs under states: "The operator's goal is the acceptance of
these stories and the approval of each wave."

## The operator's words

Goal, verbatim: "zero remaining issues on github, all bugs fixed from planning store".
Method, verbatim: "use /planning and /wave and /worktree:managing-worktrees - but in the end
provide 1 single PR to gh. make sure all are below 1 big epic for integration via bottoken from
./atlas".

## What was decided in the operator's absence

These move `draft -> proposed -> active` (`story:changes-since-by-time-within-deadline` is already
`active`):

- `epic:zero-open-issues-2026-11`
- `story:view-historical-changes-and-cancelled-reads` (GitHub #96)
- `story:search-live-typing-waits-on-the-page` (GitHub #104)
- `story:explain-names-the-merge`
- `task:migration-marker-cannot-be-evidence`
- `task:seed-authority-refusal-cli-and-kernel-disagree`
- `task:streamed-page-shows-far-valid-times`
- `task:sdk-consumer-yaml-dependency`
- `task:no-test-asserts-wall-clock-cost`
- `task:seed-race-test-names-its-signal`
- `task:held-file-payload-overwrite`

Which artifacts count as "bugs" is the coordinator's reading of the goal: a non-terminal story or
task whose body describes wrong behaviour. Artifacts that describe cost only, upstream ESS
questions and capabilities not yet built were left where they are; the epic's body lists them.

Not accepted, because something is missing rather than unasked:

- `story:supersession-verdict-fixes-replay-safe` and `story:merge-entity-review-follow-ups`:
  `decision-blocker:rules-era-for-changed-verdicts` is open.
- `task:replaced-store-close-keeps-the-restored-file`:
  `dependency-blocker:eventlog-close-without-checkpoint` is open.

Two decisions the coordinator took where an artifact left a choice open, both reversible in the
pull request:

- `task:seed-authority-refusal-cli-and-kernel-disagree`: the kernel moves to the authority
  mismatch, so the CLI, `Runtime::seed` and the conformance target give one answer and
  `docs/cli.md`'s rule for every store verb stays true.
- `task:streamed-page-shows-far-valid-times`: `/projection` and `/roles` stay as documented API
  routes with a test; they are not removed.

## What the operator would have been asked

Whether these eleven artifacts are the bugs the goal means, and whether the two decisions above
are the ones wanted.
