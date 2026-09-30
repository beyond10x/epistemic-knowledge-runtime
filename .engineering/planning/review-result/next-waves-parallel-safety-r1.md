---
format: aep.planning-md/3
id: review-result:next-waves-parallel-safety-r1
kind: review-result
status: active
title: Parallel-safety critic, next waves 2026-09-30, round 1
relations:
- reviews: release-plan:next-waves-2026-09-30
- reviews: story:extraction-document-applies-to-a-store
- reviews: story:fact-quality-by-judged-sample
- reviews: story:explain-reads-an-index
- reviews: task:one-event-type-rule
- reviews: task:projection-carries-per-type-property-definitions
- reviews: story:extraction-verb-shares-the-sdk-path
- reviews: story:sdk-store-checks
- reviews: story:preparation-blobs-are-reclaimed
- reviews: task:divergence-is-a-typed-store-error
- reviews: task:seed-document-bounds-alias-expansion
revision: 1
---
`needs-revision`

- `task:one-event-type-rule` — this task (cited, `systems/ekr/domains/views.yaml`) and `story:fact-quality-by-judged-sample` (inferred, same file, its own body already names "any unit changing `views.yaml`" as a collision risk) both land on `systems/ekr/domains/views.yaml` in wave extract-06, and neither this task's body nor the release-plan's wave extract-06 collision note (which names only the three stories' verb-registry files and conformance suites) says so — `.engineering/planning/task/one-event-type-rule.md:25`
- `task:projection-carries-per-type-property-definitions` — this task (cited, `ekr.graph-projection/1` in `systems/ekr/domains/views.yaml`) and `task:one-event-type-rule` (cited, same file) both edit `systems/ekr/domains/views.yaml` in wave extract-06, and neither body nor the release-plan's collision note for that wave says so — `.engineering/planning/task/projection-carries-per-type-property-definitions.md:18`
- `task:divergence-is-a-typed-store-error` — this task's inferred surface (`crates/ekr/src/cli/session.rs`, the `DIVERGED` const and reopen path at `:622`, found by grep since the body names no path) and `story:extraction-verb-shares-the-sdk-path`'s cited surface (same file, `respond`/`admit`) both land in wave extract-07, which carries no collision note at all in the release-plan (unlike extract-06) — `crates/ekr/src/cli/session.rs:622`
- `task:divergence-is-a-typed-store-error` — this task's inferred surface (`crates/ekr-store/src/eventlog.rs`, found by grep) and `story:preparation-blobs-are-reclaimed`'s inferred surface (same file, the reclaim hook at `:1361`–`:1378`) both land in wave extract-07, unaddressed by any collision note — `.engineering/planning/story/preparation-blobs-are-reclaimed.md:83`
- `story:extraction-verb-shares-the-sdk-path` — this (inferred, "Also likely (conformance)") and `story:preparation-blobs-are-reclaimed` (inferred, `:1876`) both claim `crates/ekr/src/conformance.rs` in wave extract-07, and neither body names the other, nor does the release-plan, which gives extract-07 no collision note comparable to extract-06's — `.engineering/planning/story/extraction-verb-shares-the-sdk-path.md:95`

What I read: 11 store artifacts (`release-plan:next-waves-2026-09-30` plus the 10 wave items: 3 stories + 2 tasks in extract-06, 3 stories + 2 tasks in extract-07) via `aep plan artifact show <id>`; the computed pairwise collisions via `aep plan artifact waves --kind story --status proposed` (5 of 6 stories — `story:sdk-store-checks` is `status: draft` and was silently excluded from that computation); the full plan graph via `aep plan artifact graph --format json`; `grep -n` in the planning-store `.md` files and `grep -rn` in `crates/` for the two tasks whose bodies cite no path (`task:divergence-is-a-typed-store-error`, `task:seed-document-bounds-alias-expansion`).

What I could not establish: `task:divergence-is-a-typed-store-error`'s body names no file at all — its surface in F3/F4 is inferred purely from a grep of the literal string `"history diverged from this handle's observed history"`, so I cannot confirm whether the fix's edit region (session-reopen logic) actually overlaps the same lines `story:extraction-verb-shares-the-sdk-path` (`respond`/`admit`, `:240`/`:680`) or `story:preparation-blobs-are-reclaimed` (reclaim hook, `:1361`–`:1378`) name, only that both land in the same file. `story:sdk-store-checks` (`crates/ekr-sdk/src/checks.rs`, `crates/ekr-sdk/tests/checks.rs`, both self-declared inferred) I checked by hand against the rest of wave extract-07 and found no shared file with any other item — not in my findings, but worth noting since the CLI's own tool never assessed it either. `task:seed-document-bounds-alias-expansion` (cited: `crates/ekr/src/cli/seed.rs`, `crates/ekr-kernel/src/document/shape.rs`) collides with nothing else in either wave. Whether waves extract-06 and extract-07 run sequentially or could overlap is outside what the release-plan states explicitly; I treated them as sequential (the "why now" line for `story:preparation-blobs-are-reclaimed` sequences it "after explain-index" across the wave boundary) and did not raise cross-wave overlaps such as `mod.rs`/`docs/cli.md` between extract-06 and extract-07 as findings — flag if that assumption is wrong.

```findings
- file: .engineering/planning/task/one-event-type-rule.md
  line: 25
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: this task (cited, systems/ekr/domains/views.yaml) and story:fact-quality-by-judged-sample (inferred, same file) both land on systems/ekr/domains/views.yaml in wave extract-06, and neither this task's body nor the release-plan's wave extract-06 collision note (which names only the three stories' verb-registry files and conformance suites) says so
- file: .engineering/planning/task/projection-carries-per-type-property-definitions.md
  line: 18
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: this task (cited, ekr.graph-projection/1 in systems/ekr/domains/views.yaml) and task:one-event-type-rule (cited, same file) both edit systems/ekr/domains/views.yaml in wave extract-06, and neither body nor the release-plan's collision note for that wave says so
- file: crates/ekr/src/cli/session.rs
  line: 622
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: task:divergence-is-a-typed-store-error's inferred surface (crates/ekr/src/cli/session.rs, the DIVERGED const/reopen path, found by grep since the body names no path) and story:extraction-verb-shares-the-sdk-path's cited surface (same file, respond/admit) both land in wave extract-07, which carries no collision note at all in the release-plan
- file: .engineering/planning/story/preparation-blobs-are-reclaimed.md
  line: 83
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: task:divergence-is-a-typed-store-error's inferred surface (crates/ekr-store/src/eventlog.rs, found by grep) and this story's inferred surface (same file, the reclaim hook at :1361-:1378) both land in wave extract-07, unaddressed by any collision note
- file: .engineering/planning/story/extraction-verb-shares-the-sdk-path.md
  line: 95
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: this story (inferred, "Also likely (conformance)") and story:preparation-blobs-are-reclaimed (inferred, :1876) both claim crates/ekr/src/conformance.rs in wave extract-07, and neither body names the other, nor does the release-plan, which gives extract-07 no collision note comparable to extract-06's
```
