---
format: aep.planning-md/3
id: review-result:viewer-progress-r2
kind: review-result
status: active
title: Captured application viewer progress rereview
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new coordinator/viewer implementor findings; 0 delegated implementor findings. The captured-boundary finding in viewer-progress-review-r1 is closed within this bounded source/log rereview.

`crates/ekr/src/cli/inbox.rs:373-380` now derives unfinished selected items and pending corrections from the same admitted commit set used for the schema and per-item labels. It no longer lets `shown.application.remaining_items` or `corrections_pending` from a later projection override the supplied historical read. Mapping completion uses the exact same qualified-key predicate for the overall status and individual labels. Corrections are complete only when the supplied read contains the matching actual final correction commit. Commit-command metadata with a nonmatching event ID still contributes no progress.

Inspected implementor evidence under `<retained-evidence>/schema-application-kernel/`:

- `viewer-progress-capture-red.log`: terminal one failed test. With genuine later correction-complete application state and a genuine historical schema-only read at revision 2, the old renderer falsely displayed Complete and corrections committed.
- `viewer-progress-capture-green.log`: terminal one passed, zero failed in 19.07 seconds. The source loops through schema-only and real correction applications on both File and SQLite. It verifies current completion, historical partial/pending correction rendering, full-replay reopening, escaped text, retained history and no events appended by page rendering. The nonmatching publication-event helper control remains present.

This is a deterministic historical-read/later-projection reproduction of the race's input condition, not a scheduled concurrent-thread test. No real mapping-item branch is rendered by this fixture; that branch's correction was source-inspected through its shared exact-key predicate. The malformed-event control is not evidence of executing a real Stale attempt. Earlier screenshot inspection concerned the schema-only completed page and is not claimed as a fresh browser run for this correction.

Inspected SHA-256 fingerprints:

- `crates/ekr/src/cli/inbox.rs`: `a285e05b2d2f9365ba8e24fa35658f16784a43c088a9af96c694ded3090bb03e`
- `docs/cli.md`: `981a53fcaefc02e02604aca12c73e3899ad96c26c1e0c7251f9c62966a984fb6`

Limitations: source/log review only; executions belong to the implementor. This reviewer ran no builds, tests, mutations or browser sessions and made no source/AEP/publication changes. No new concrete defect was established in this correction. Approval does not cover every viewer branch, whole F or the full gate.

```findings
[]
```
