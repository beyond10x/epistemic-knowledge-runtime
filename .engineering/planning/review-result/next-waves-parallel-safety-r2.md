---
format: aep.planning-md/3
id: review-result:next-waves-parallel-safety-r2
kind: review-result
status: active
title: Parallel-safety critic, next waves 2026-09-30, round 2
relations:
- reviews: release-plan:next-waves-2026-09-30
- reviews: task:one-event-type-rule
- reviews: story:sdk-store-checks
- reviews: story:extraction-verb-shares-the-sdk-path
- reviews: story:extraction-document-applies-to-a-store
- reviews: story:fact-quality-by-judged-sample
- reviews: story:explain-reads-an-index
- reviews: task:projection-carries-per-type-property-definitions
- reviews: story:preparation-blobs-are-reclaimed
- reviews: task:divergence-is-a-typed-store-error
- reviews: task:seed-document-bounds-alias-expansion
revision: 1
---
`needs-revision`

- `task:one-event-type-rule` — this task's body cites `docs/cli.md` § Roles and § `ekr ocel` (cited) as places the event-type rule is documented; the three extract-06 stories already claim `docs/cli.md` in the wave's verb-registry collision (cited, release-plan), but the extract-06 collision note names only the three stories for `docs/cli.md`, omitting this task — `.engineering/planning/task/one-event-type-rule.md:27`
- `story:sdk-store-checks` — its Surface (cited, `crates/ekr-sdk/src/checks.rs`, a new module) and `story:extraction-verb-shares-the-sdk-path`'s Surface (cited, `crates/ekr-sdk/src/extraction.rs`, also new) each need a new `pub mod` line in `crates/ekr-sdk/src/lib.rs` (inferred from its existing `pub mod` list), and neither body nor the extract-07 collision note names `lib.rs` — `crates/ekr-sdk/src/lib.rs:7-16`

Round 1's five findings are resolved: the extract-06 note names the three-way `views.yaml` collision; the extract-07 note names `session.rs`, `eventlog.rs` and `conformance.rs`; `task:divergence-is-a-typed-store-error` carries a cited Surface section; the `depends_on` edge from `story:preparation-blobs-are-reclaimed` to `story:explain-reads-an-index` is present.

What I read: `release-plan:next-waves-2026-09-30` (revision 2) and its 10 items via `aep plan artifact show`; `review-result:next-waves-parallel-safety-r1`; `aep plan artifact waves --kind story --status proposed` (4 waves, 37 collisions, 0 unassessed; tasks and the draft `story:sdk-store-checks` excluded, as in round 1); `grep -n` in `docs/cli.md` (verb table 141-166, `### ekr ocel` at 448, `#### Roles` at 770) and `crates/ekr-sdk/src/lib.rs`.

What I could not establish: whether the extract-07 phrase "the verb registry by the extraction verb and `story:sdk-store-checks`' SDK side only" was meant to cover `lib.rs`; whether the task's `docs/cli.md:448`/`:770` edits fall in the lines the three stories touch or only the same file (treated as same-file, different-section).

```findings
- file: .engineering/planning/task/one-event-type-rule.md
  line: 27
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this task cites docs/cli.md § Roles and § ekr ocel as places the event-type rule is documented, and the three extract-06 stories already claim docs/cli.md in the wave's verb-registry collision, but the extract-06 collision note names only the three stories for docs/cli.md, omitting this task"
- file: crates/ekr-sdk/src/lib.rs
  line: 7
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:sdk-store-checks (new crates/ekr-sdk/src/checks.rs) and story:extraction-verb-shares-the-sdk-path (new crates/ekr-sdk/src/extraction.rs) each need a new pub mod line in crates/ekr-sdk/src/lib.rs, and neither body nor the extract-07 collision note names lib.rs"
```
