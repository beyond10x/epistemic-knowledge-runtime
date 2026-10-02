---
format: aep.planning-md/3
id: task:ocel-export-prints-its-counts
kind: task
status: implemented
title: ekr ocel prints the counts of its export
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-02T16:21:30Z", actor: "agent:codex-ekr-x7b-root", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## What is wrong

`ekr ocel` prints only the `ekr.ocel/1` document. A consumer (2026-10-01) recounts the export's
events, objects and relationships itself. Those counts are what `ekr.views.OcelExported` already
carries (`crates/ekr-views/src/ocel.rs`).

## Build

The `OcelExported` counts appear in the document's `meta`, in the format its spec names, or on
stderr. The spec says which, and the one-shot and session lanes agree.

## Acceptance

- The counts printed equal `ekr_views::export_ocel`'s `OcelExported` for each OCEL fixture.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr/src/cli/ocel.rs`
- `systems/ekr/domains/views.yaml`
- `docs/cli.md`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.

## Resume decision

Use the existing OcelExported summary on stderr, preserving the export document's bytes.
The session reply carries the same counts in its per-request stderr field; a successful
session request must not leak counts to process-global stderr. SDK OCEL reads expose both.
The named-time task preserves default stdout relative to the released 0.0.26 document.

## Second correction verified (2026-10-02)

Coordinator read the final source-only correction 848e023ec against 4ed8d5a3b.
Each explicit event name now travels as one --events=<name> argument, matching the corrected
--event-time=<selector> encoding. The test diff between those commits is empty: no assertion
was dropped, weakened or re-pinned. The implementor's on2-correction-exact.log and
on2-correction-suite.log retain the unchanged adversary matrix and complete SDK lane passing;
on2-correction-clippy.log, formatting and diff checks are green. Both bot identities verified.
No third attack was opened: the coordinator verified the second correction as the wave procedure
requires. These unit results permit integration; artifact completion still requires the combined gate.

The two recorded passes each found one introduced boundary defect. The CLI computes their trend:
carried 0, new 1, resolved 1

```json
{
  "artifact": "task:ocel-export-prints-its-counts",
  "reviews": 2,
  "from": "review-result:adversary-extract-07b-on-pass-1",
  "from_reviewer": "unattributed",
  "to": "review-result:adversary-extract-07b-on-pass-2",
  "to_reviewer": "unattributed",
  "carried": [],
  "new": [
    {
      "file": "crates/ekr-sdk/src/read/ocel.rs",
      "line": 204,
      "category": "boundary",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "The SDK events list still passes leading-dash type names as separate argv values, so both transports refuse admitted names accepted by the CLI equals form."
    }
  ],
  "resolved": [
    {
      "file": "crates/ekr-sdk/src/read/ocel.rs",
      "line": 207,
      "category": "boundary",
      "severity": "blocker",
      "verdict": "NEEDS-CHANGE",
      "origin": "introduced",
      "message": "The SDK passes valid leading-dash event-time selectors as separate argv values, so both transports refuse names the CLI accepts with equals syntax."
    }
  ]
}

```

## Combined source correctness gate

The complete `task check` passed on frozen source
`570cb34cf157e0703d3a48c7ce6d102933cb380d`. The retained coordinator log
`<cache>/ekr-extract-07b/coordinator/full-serialization-combined-check.log` ends with:

```text
CHECK_EXIT=0
Fri Oct  2 16:19:40 UTC 2026
```

This covers formatting, workspace clippy and tests, benchmark-feature compilation, rustdoc,
vendored YAML compatibility, pinned specification validation, generated-suite freshness and
planning validation. Historical prose-only planning review warnings remain. The previously
recorded ext4 temporary directory is used without changing the inode-reuse test. All feature
acceptance and retained review corrections are exercised on the combined source. Implementation
status does not claim publication: wave release remains held on the separate performance task.
