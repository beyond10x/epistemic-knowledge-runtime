---
format: aep.planning-md/3
id: review-result:next-waves-design-r1
kind: review-result
status: active
title: Design critic, next waves 2026-09-30, round 1
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
- reviews: epic:read-and-storage-cost
revision: 1
---
**needs-revision**

story:preparation-blobs-are-reclaimed — its own scope section names `crates/ekr-kernel/src/explain.rs:227,381,401-406` as readers of the `receipt.proposal` field this story changes — the same lines story:explain-reads-an-index cites as the per-call parse it replaces (`explain.rs:381`) — and the release plan sequences it "after explain-index... since both change `replay.rs`, `explain.rs` and `kernel.yaml`", yet the story's `relations` carry no `depends_on` edge to `story:explain-reads-an-index` — .engineering/planning/release-plan/next-waves-2026-09-30.md:46, .engineering/planning/story/preparation-blobs-are-reclaimed.md:8-9,84,87

What I read: all 10 drafted items plus `epic:read-and-storage-cost` and `release-plan:next-waves-2026-09-30`, whole body, via `aep plan artifact show <id>` (12 calls); `aep plan artifact relations`; `aep plan artifact graph --format json` (walked `depends_on` edges from each of the 10 items and their full dependency closure — 6 outside artifacts, all `status: implemented` — no cycle found, 17 edges walked with revisits); `aep plan artifact validate` (reports `valid`, 14 pre-existing prose-only-findings notices unrelated to this set — not mine).

What I could not establish: whether the coordinator intends `story:preparation-blobs-are-reclaimed` to ever be scheduled outside a release plan that already sequences it after `story:explain-reads-an-index` by wave number alone — the release-plan text currently supplies the order, so this is the artifact-graph gap the rubric asks me to name, not a live scheduling break in this specific plan.

```findings
- file: .engineering/planning/story/preparation-blobs-are-reclaimed.md
  line: 9
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: its own scope section names crates/ekr-kernel/src/explain.rs:227,381,401-406 as readers of the receipt.proposal field this story changes — the same lines story:explain-reads-an-index cites as the per-call parse it replaces — and the release plan sequences it "after explain-index" for that reason, but no depends_on edge records the order
```
