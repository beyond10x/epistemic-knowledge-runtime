---
format: aep.planning-md/3
id: approval-record:zero-issues-wave-2-proposal-2026-11-09
kind: approval-record
status: draft
title: Wave 2 of plan 2026-11-09 and the dispatch order accepted with no operator present
tags:
- non-interactive
relations:
- decides: story:view-historical-changes-and-cancelled-reads
- decides: task:seed-authority-refusal-cli-and-kernel-disagree
- decides: task:streamed-page-shows-far-valid-times
revision: 1
---
## Which stop

The stage-1 proposal of wave 2 of the plan on `.engineering/waves/wave-2026-11-09-zero-issues.md`
(units H, I, J), and the order in which the remaining units of waves 1 and 2 are dispatched.

## Why non-interactive

A coordinator session took over this run on 2026-11-09 from one that stopped; it runs with no
operator turn, held by a Stop hook until the goal is met. The brief states: "The operator's goal is
the acceptance of these stories and the approval of each wave."

## The operator's words

Goal, verbatim: "zero remaining issues on github, all bugs fixed from planning store".

## What was decided in the operator's absence

- Wave 2 runs: H `story:view-historical-changes-and-cancelled-reads` (#96), I
  `task:seed-authority-refusal-cli-and-kernel-disagree`, J `task:streamed-page-shows-far-valid-times`.
- The taking-over coordinator runs at most one sub-agent at a time (its caller's limit). No two
  units are therefore ever in flight together, and each unit's branch is forked from the
  integration head current at its dispatch. The collision the verb printed
  (`story:changes-since-by-time-within-deadline story:view-historical-changes-and-cancelled-reads
  crates/ekr-views/src/changes.rs (inferred)`) keeps C and H apart in time, which sequential
  dispatch guarantees in either order.
- Dispatch order: H first (the caller named #96 first), then B, E, F, C, I, J, then K (wave 3).
  Unit A's implementor from the previous coordinator is still running in `ekr-zi-ua`; it is settled
  when its lease ends or its work is committed. D and G are verification runs the coordinator makes
  itself in `ekr-zi-verify`.
- Every implementor and adversary is dispatched as `aep:implementor` / `aep:adversary` on model
  `opus`.

## What the operator would have been asked

Approve wave 2 and the dispatch order above.

## Commits this authorises

One or more commits per unit on its branch, the merges into `wave/20261109-zero-issues`, the store
commits, the push of that branch as b10x-bot, one pull request, and its squash merge into `main` on
green checks. No tag, no version bump, no release.
