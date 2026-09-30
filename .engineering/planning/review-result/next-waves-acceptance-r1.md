---
format: aep.planning-md/3
id: review-result:next-waves-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, next waves 2026-09-30, round 1
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
`needs-revision`

story:preparation-blobs-are-reclaimed — the third acceptance bullet's condition, "a proposal that needs it is open," is undefined; the story's own Scope section lists `what "a proposal that needs it is open" means` among what is not established, so two people checking the same reclaim test can reach different answers — `.engineering/planning/story/preparation-blobs-are-reclaimed.md:66` (admission at `:89`)

task:seed-document-bounds-alias-expansion — the acceptance is split across two sections: `## Acceptance` (lines 29–31) covers only the alias/size limit, and a second, independently checkable requirement — refusing a seed or transaction document nested past a stated depth — is added afterward under `## Also: nesting depth`, whose text says "the acceptance extends to" rather than living in the Acceptance section itself; a reader checking only the named section can mark the task done without that refusal ever existing — `.engineering/planning/task/seed-document-bounds-alias-expansion.md:28` (extension at `:37`)

What I read: all 11 artifacts, whole body, via `aep plan artifact show <id>` — story:extraction-document-applies-to-a-store, story:fact-quality-by-judged-sample, story:explain-reads-an-index, task:one-event-type-rule, task:projection-carries-per-type-property-definitions, story:extraction-verb-shares-the-sdk-path, story:sdk-store-checks, story:preparation-blobs-are-reclaimed, task:divergence-is-a-typed-store-error, task:seed-document-bounds-alias-expansion, epic:read-and-storage-cost. Also `aep plan artifact kinds`, `aep plan artifact lifecycle epic|story|task`, one precedent artifact (`story:add-evidence-operation`, implemented, to check whether this store's 2–3-bullet Acceptance convention is store-wide) and one precedent epic (`epic:ingestion-throughput`, implemented, which carries no `## Acceptance` section at all, relying on `## Outcome` and `## Stories`). I read `crates/ekr-views/src/index.rs:590-610` (`timing.event` rule) in the tree to check `task:one-event-type-rule`.

What I could not establish: whether `task:one-event-type-rule`'s acceptance ("agree on the event types of every views fixture") could pass trivially today without the fix — I found the timeline's rule (`index.rs:599`) but could not locate the `/roles` structural rule's definition in `view_roles.rs` in the time available, so I could not confirm or rule out that the two rules already agree on every current fixture. I did not flag it; this is a gap in my check, not a finding. I judged the store-wide convention of 2–3 bullets per `## Acceptance` section (present in 8 of the 10 stories/tasks, and in the already-implemented `story:add-evidence-operation`) as pre-existing, conjunctive and each-bullet-independently-checkable rather than a defect, and out of lane where it would be a scope/split-story question rather than a checkability one.

```findings
- file: .engineering/planning/story/preparation-blobs-are-reclaimed.md
  line: 66
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: 'the third acceptance bullet''s condition, "a proposal that needs it is open," is undefined — the story''s own Scope section lists what that phrase means among what is not established, so two readers checking the same reclaim test can reach different answers'
- file: .engineering/planning/task/seed-document-bounds-alias-expansion.md
  line: 28
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: 'the acceptance is split — "## Acceptance" covers only the alias/size limit, and the nesting-depth refusal requirement is added afterward under "## Also: nesting depth" ("the acceptance extends to"), so a reader checking only the named section can mark the task done without that refusal existing'
```
