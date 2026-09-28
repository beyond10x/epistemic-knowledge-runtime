---
format: aep.planning-md/3
id: review-result:adversary-read-02-h-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit H: views without head'
relations:
- reviews: task:historical-projection-carries-the-head
revision: 1
---
unit: H, task:historical-projection-carries-the-head, commit 7b8c4c95 on impl/views-without-head (worktree ekr-r2-h), plus the adversary's untracked test file
verdict: nothing found
cases: executed 56→60 (ekr-views), red 0
origin: introduced 0 / pre-existing 0 / undecided 0

## Cases added (all green; kept as crates/ekr-views/tests/adversary_views_head_pass1.rs)

On the file and SQLite providers: revisions 0–5 answered through one handle, then 3 commits, then a
new handle — the projection and every bounded answer (overview at two limits, every node's
detail, every expansion page from every node with its `next`/`remaining` cursors, searches at
limits 1 and 100, timelines for every subject, hops 1–3 and both buckets) keep their bytes; an
`IndexCache` holding every earlier revision across 3 commits keeps serving the same `Arc`, and each
held index equals a fresh load.

## What held

`ess specify validate` (ess 0.36.0) is valid and the recorded synthesize commands reproduce
`suite.json` and `views-suite.json` byte for byte; no field of a past revision varies with later
commits; a revision beyond the head is still refused and refusals are never cached; `/head` goes
through the Host, method and body checks, a query is 400 and an unseeded store 404 `NotSeeded`;
both viewer pages show the larger of `/head` and the shown revision; `ekr mcp` carries no `head`;
no `meta.head` reader remains outside the viewer's adapted `meta`.

```findings
[]
```
