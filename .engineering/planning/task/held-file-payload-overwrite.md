---
format: aep.planning-md/3
id: task:held-file-payload-overwrite
kind: task
status: active
title: Investigate a held payload after an out-of-band file overwrite
relations:
- informed_by: task:held-bytes-notice-deleted-blobs
- informed_by: story:viewer-evidence-reuses-admitted-revision
- serves: vision:o2
- decomposes: epic:zero-open-issues-2026-11
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:55:01Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:55:02Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
---
## Observation

The evidence-cache implementor's compatibility baseline, retained at `<cache>/ekr-evidence-cache/unit/compat-baseline.log`, reproduced a pre-existing discrepancy before the viewer handler changed: warm a retained evidence read through a held File runtime, overwrite its immutable payload blob without a log or manifest publication, then request that evidence again. The held runtime returned the earlier verified bytes with status 200 where the probe expected refusal. The exact probe is retained at `<cache>/ekr-evidence-cache/unit/out-of-band-probe.rs.txt`.

This is not fixed or weakened by the viewer-cache correction. That correction keeps `Runtime::content` and the existing provider authority; its corruption coverage concerns a fresh or replaced store whose bytes are verified again. The discrepancy was directly reproduced against the unchanged handler, rather than inferred from code age.

## Contract and next step

The implemented `task:held-bytes-notice-deleted-blobs` points to the decision in `task:sqlite-store-replaced-in-place`: a live handle never answers bytes a fresh handle would refuse. Investigate whether the raw overwrite case violates that decision, distinguish out-of-band corruption from supported withdrawal publication, and restore the decided invariant with a deterministic regression if required. Do not silently narrow the prior decision or claim this observation is resolved by revision index reuse.

## Scope

Provider payload revalidation and its declared immutable-byte trust boundary; generic synthetic fixtures only. This is separate from the viewer handler performance patch and does not block that patch's unchanged provider semantics.

## Surface (story-scoper, 2026-11-09)

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-store` object memo (held bytes) of `EventlogStore` — inferred, the task names no file; its neighbour cites `crates/ekr-store/src/eventlog.rs`
- **Files:** `<tree>/crates/ekr-store/src/eventlog.rs:2497` (`get`), `:1195` (`required`), `:1310` (`verified_objects`), `:221` (`HeldObject`) — cited (tree)
- **Serving site:** `Runtime::content` (`crates/ekr-kernel/src/runtime.rs:622`) -> kernel `content` (`crates/ekr-kernel/src/commit.rs:492`) -> store `get`; a held hash is returned at `eventlog.rs:2508-2513` from `required`, which hands back `HeldObject::retained()` (`eventlog.rs:230`) — cited (tree)
- **Re-verification:** none of the bytes per read. A held read confirms the revision prefix (`confirm_held`, `eventlog.rs:2504`) and looks through the log for a moved object stream (`eventlog.rs:1202`); the blob is re-read and re-hashed only for hashes not held (`eventlog.rs:1272-1300`), and a moved held object reuses its held bytes with `|_| true` (`eventlog.rs:1290`) — cited (tree)
- **Symbols:** `Runtime::content` — cited (task); `EventlogStore::get`, `required`, `read_required`, `retained_object`, `checked_object`, `look_through_log`, `crate::verified` — inferred
- **Also likely:** `crates/ekr-store/src/verified.rs` (process registry of verified bytes, shared across handles) — inferred
- **Also likely:** new File-provider regression beside `crates/ekr-store/tests/eventlog_object_memo.rs` or `adversary_x6_c.rs:248`; viewer-level case in `crates/ekr/src/cli/view.rs` tests (`runtime.content` at `:2026`) — inferred
- **Documents:** `systems/ekr/domains/store.yaml:715-778` (held bytes; rule 3 at `:773`), `docs/epistemic-knowledge-runtime-design.md` (§ 100 or a new section), `CHANGELOG.md` — inferred
- **Decision conflict to settle first:** rule 3 (`store.yaml:773`) already records "a redaction or blob deletion made through a provider directly, outside the runtime, without that event" as not seen. A raw overwrite is not named there, so the task is either a one-line extension of rule 3 (documents only) or a per-read blob re-check (code) — cited (tree)
- **Confidence:** medium — the read path and the missing re-check are read from the tree; which of the two outcomes the investigation picks is open
- **Would collide with:** any unit touching `crates/ekr-store/src/eventlog.rs` (memo, `get`, `required`), the held-bytes section of `store.yaml`, or `eventlog_object_memo.rs` — inferred
- **Safety fact:** every held answer funnels through `verified_objects` -> `required` (`eventlog.rs:1196`, `:2508`), so a re-check placed there covers `get` and history loads alike; the cost is one blob read and hash per held non-canonical read, which `story:commit-cost-flat-with-store-size` removed on purpose — level 2 (pointed at file:line), unproven
