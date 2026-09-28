---
format: aep.planning-md/3
id: task:checkpoint-graph-root-is-unbound
kind: task
status: implemented
title: A replay checkpoint with a changed graph root is admitted, and the commits made after it do not replay
summary: 'High: restored checkpoint''s GraphRoot is outside every check; later receipts record it and full replay refuses them'
tags:
- review-2026-09-28
- severity-high
relations:
- serves: vision:o5
- derived_from: story:eventlog-0-4-batched-reads
- informed_by: architecture-decision-record:0010-replay-checkpoints-record-verified-history
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T02:51:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T02:51:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T06:19:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Severity

High (the review's rating, kept). The damage is not a wrong read that `--full-replay` corrects: the
kernel goes on to publish new retained records from the forged state, and those records are
immutable, so the store's own canonical history stops replaying.

## The review (external review of EKR, 2026-09-28, verbatim)

> High — checkpoint corruption can lead to unreplayable commits. Changing only the cached
> graph-root ID is accepted. Subsequent ordinary propose/validate/commit succeeds, but full replay
> then rejects the history. Reproduced on both providers.
> `crates/ekr-kernel/src/checkpoint.rs:414`

## What was verified (code reading at `wave/p2p3p4-06` 69e5b994; not run)

- **Verified**: `restored` builds the head graph with `root: document.root` straight from the
  checkpoint (`crates/ekr-kernel/src/checkpoint.rs:415`; the review's `:414` is the line before).
  Besides its revision number (`checkpoint.rs:413`), the only check on the restored graph is
  `knowledge_root` and `evidence_root`
  (`checkpoint.rs:438-443`), and `knowledge_root` hashes nodes, edges and assertions only
  (`crates/ekr-store/src/log.rs:10-19`). No field of `GraphRoot` (`id`, `space`,
  `schema_version_id`, `parent`, `created_at`; `crates/ekr-graph/src/root.rs:29-39`) is bound.
- **Verified**: the forged id becomes every revision's `graph_root` (`checkpoint.rs:444`, `:456`),
  which `replay::basis` writes into each new validation basis as `graph_root_id`
  (`crates/ekr-kernel/src/replay.rs:297`), and which the reference validator compares every
  `root_id` against (`crates/ekr-kernel/src/validate/reference.rs:204`, `:231`).
- **Verified**: full replay takes `graph_root` from the seed (`replay.rs:117`) and re-derives the
  basis (`replay.rs:543`, `:576`, `:644`), so a receipt that recorded the forged id disagrees.
- **Verified**: the existing forgery test covers a dropped assertion, another host, another history
  and an earlier head, and no root field
  (`crates/ekr-kernel/tests/replay_checkpoint.rs:296-353`).
- **Inferred, not observed by me**: that ordinary propose/validate/commit then succeeds and full
  replay refuses. The review says it reproduced this on both providers.
- ADR 0010 accepts that a writer who forges a self-consistent history and checkpoint is read as
  canonical until a full replay. This finding is a different case: only the cache is changed, and
  the kernel itself then writes history that no replay admits. ADR 0010 also says a checkpoint
  that fails its binding is ignored; this one passes because the root is outside the binding.

## Reproduce

Add a fifth forgery beside the four at `replay_checkpoint.rs:309`, changing the root id, for
example `forged["graph"]["graph"]["root"]["id"] = <a fresh GraphRootId>` (path inferred from the
assertion forgery at `:311`). Install it, open without full replay, propose, validate and commit
one transaction, then open with `open_in_full` and read the answers.

## Acceptance

On both providers, a checkpoint whose graph root differs from the root the seed admitted in any
field is ignored: the open replays in full, a propose/validate/commit made after it succeeds, and a
following `--full-replay` open returns the same answers as the non-full open. A forgery case in
`crates/ekr-kernel/tests/replay_checkpoint.rs` that changes the root id fails before the fix and
passes after it.
