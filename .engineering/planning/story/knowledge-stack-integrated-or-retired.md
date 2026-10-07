---
format: aep.planning-md/3
id: story:knowledge-stack-integrated-or-retired
kind: story
status: draft
title: The unmerged knowledge-inbox and schema-application stack is integrated or retired
revision: 1
---
## What is there

Work on the knowledge inbox, evidence-led schema learning and authored schema application ran on
2026-10-03 and 2026-10-04 in 18 worktrees on stacked branches and was never merged. On 2026-10-07
the worktrees were removed and their work published:

| what | where |
|---|---|
| every commit of the 18 branches, 110 commits over the merge base `4b3c2eb789` | `origin/ekr/application-conformance-20261004`, tip `bad9247ea1` plus one commit of files left uncommitted |
| uncommitted files left in 9 other worktrees, committed as found and unreviewed | one `wip:` commit on each tree's branch under `origin/ekr/*-2026100[34]` |
| uncommitted files of `ekr/schema-proposals-20261003` (3 files) | not pushed: Gates refused the commit with 3 findings; kept as the local worktree archive `ekr-schema-proposals-20261003` |
| the open pull request | https://github.com/beyond10x/epistemic-knowledge-runtime/pull/64 (`feat/knowledge-inbox-schema`; the local branch of that name is 15 commits ahead of it, all inside the tip) |

The stack carries its own planning artifacts and wave pages
(`.engineering/waves/knowledge-*.md` on the tip). They are not in `main`'s store.

## What it is waiting on

The stack's `knowledge-contract-adoption` wave page says that final acceptance and release-plan
clearance still require a published ESS release and artifacts, final regeneration, exact
version, checksum and revision pins, and the complete gate. The stack generated its contracts with
an unreleased ESS candidate (`126c2b3905d0f4279086b9d3030096147955dfec`); `main` pins ESS 0.36.0.
`main` has moved 129 commits past the merge base since.

## Acceptance

One of these, recorded in the store:

- the stack is rebased or merged onto `main` against a released ESS, passes `task check`, and lands
  through one pull request; or
- an operator decision retires it, naming what is kept.
