---
format: aep.planning-md/3
id: review-result:next-waves-1001-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, next waves 2026-10-01, round 1
relations:
- reviews: release-plan:next-waves-2026-10-01
- reviews: story:commit-cost-flat-with-store-size
- reviews: task:validate-cost-flat-with-store-size
revision: 1
---
`needs-revision`

task:migrate-reads-a-current-store, story:extraction-verb-shares-the-sdk-path, task:divergence-is-a-typed-store-error, task:sqlite-store-replaced-in-place, task:held-bytes-notice-deleted-blobs, task:read-only-open-passes-under-load, task:rendering-cost-test-passes-under-load, task:seed-document-bounds-alias-expansion — none (each checkable; V covers all six adversary findings; G's narrowing is disclosed in its own bullet).

story:commit-cost-flat-with-store-size — the revision note (lines 54–56) discloses only a measurement-statistic change and does not say the acceptance also dropped the title's validate-specific flatness for an aggregate bound that passes at 1.40–1.43× while validate alone grew 3.28× in the same run, and the story moved to `implemented` with no edge holding it on `task:validate-cost-flat-with-store-size` — .engineering/planning/story/commit-cost-flat-with-store-size.md:54-70

task:validate-cost-flat-with-store-size — the acceptance was rewritten from a pointer that closed the parent story's bound to an independent 1.2× bound with no revision sentence, and its Build section (line 32) still asks for a profile "on an idle machine", the condition the sibling story's revision says is unavailable — .engineering/planning/task/validate-cost-flat-with-store-size.md:30-43

Out of lane: the held-bytes task's Decision section self-references (not an acceptance defect).

```findings
- file: .engineering/planning/story/commit-cost-flat-with-store-size.md
  line: 54
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the revision note names only a statistic change, not that the acceptance dropped the title's validate-specific flatness for an aggregate bound that passes while validate alone grows 3.28x, with no edge holding the story on task:validate-cost-flat-with-store-size"
- file: .engineering/planning/task/validate-cost-flat-with-store-size.md
  line: 37
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance was rewritten to an independent 1.2x bound with no revision note, and the Build section still asks for a profile on an idle machine"
```
