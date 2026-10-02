---
format: aep.planning-md/3
id: review-result:next-waves-1001-design-r2
kind: review-result
status: active
title: Design critic, next waves 2026-10-01, round 2
relations:
- reviews: release-plan:next-waves-2026-10-01
- reviews: story:evidence-attaches-to-a-held-assertion
revision: 1
---
needs-revision

story:evidence-attaches-to-a-held-assertion — its relations carry no `depends_on` edge to `task:migrate-reads-a-current-store`, even though the plan states that exact dependency for this item ("M: a store with attachments must migrate") and in M's own row, and records the identical M-dependency as a formal edge on the sibling `story:preparation-blobs-are-reclaimed` — .engineering/planning/release-plan/next-waves-2026-10-01.md:37,68,83 (graph: the story has only decomposes and serves)

What I read: the release plan (revision 2) and the 13 items its waves schedule; `relations`; `graph --format json` (395 artifacts) and a depends_on/blocks cycle walk (none); `validate`; the plan's history; the round-1 reviews. The S and F pairings remain sound. Not established: whether "S's decision on withdrawn bytes" in storage-08 is meant as an edge.

```findings
- file: .engineering/planning/release-plan/next-waves-2026-10-01.md
  line: 68
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:evidence-attaches-to-a-held-assertion carries no depends_on edge to task:migrate-reads-a-current-store, though the plan states that dependency and records the same one as an edge on story:preparation-blobs-are-reclaimed"
```
