---
format: aep.planning-md/3
id: review-result:next-waves-1001-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, next waves 2026-10-01, round 2
relations:
- reviews: release-plan:next-waves-2026-10-01
- reviews: story:commit-cost-flat-with-store-size
- reviews: task:validate-cost-flat-with-store-size
revision: 1
---
needs-revision

task:validate-cost-flat-with-store-size — the new "counting test" bullet names no bound and no counted quantity ("whatever the profile names"), so nobody can tell pass from fail until the Build profile runs, unlike the sibling bullet's explicit 1.2× figure in the same section — .engineering/planning/task/validate-cost-flat-with-store-size.md:47-48

Both round-1 findings are addressed: the story's revision note now discloses the dropped validate-specific bound and the retitling; the task carries its own revision sentence and the Build section no longer asks for an idle machine.

Out of lane: the story moved to implemented with only a prose link to the task (a traceability question for the scope critic).

What I read: `show` on the 11 artifacts; `git diff` against their committed revisions; `relations`; `lifecycle task`.

```findings
- file: .engineering/planning/task/validate-cost-flat-with-store-size.md
  line: 47
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the new counting-test bullet names no bound and no counted quantity (whatever the profile names), so nobody can tell pass from fail until the Build profile runs"
```
