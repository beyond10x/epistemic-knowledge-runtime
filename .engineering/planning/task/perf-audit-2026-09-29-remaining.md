---
format: aep.planning-md/3
id: task:perf-audit-2026-09-29-remaining
kind: task
status: draft
title: 'Performance audit 2026-09-29: findings not yet scheduled'
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
revision: 2
---
## Context

Findings of the performance audit of 2026-09-29 that are recorded but not scheduled in waves
perf-01 or perf-02. Each carries its measured cost at 1× / 3× / 10× of a consumer's shape.

- **`explain`** and **storage amplification**: scheduled as `story:explain-reads-an-index`
  (implemented in 0.0.25) and `story:preparation-blobs-are-reclaimed`.
- **Views**: `/changes` is uncached (0.30 / 0.69 s warm); timeline ranking is superlinear (cold 29 /
  182 / 1,064 ms); an overview at an old revision loads the full history (cold 1.6 / 3.8 / 12.5 s)
  (`crates/ekr-views/src/changes.rs:556`, `timeline.rs:333`, `crates/ekr-kernel/src/read.rs` `schema_history`).
- **`snapshot`** re-hashes and re-decodes the seed envelope and recomputes the root per call
  (`explain.rs:301–346`) and builds its output as a `Value` tree (`crates/ekr/src/cli/mod.rs:784`):
  session `snapshot` 1.4 → 2.3–3.0 s (1× → 3×); output 87 / 262 / 380 MB.
- **Seeding** hashes evidence as number-array JSON (`crates/ekr-kernel/src/commit.rs:357`): ~1.2 s
  per MB, 54 s for 44 MB; check whether `ekr-seed-envelope/3` (0.0.16) already removes it.
- **Viewer**: every stream batch re-applies every opened node detail, each followed by a full
  redraw (`crates/ekr/src/cli/viewer/index.html:1453`, 1248–1284); read from the code, not measured.
- **Architecture**: structurally shared graph maps (the per-commit copy, per-revision graphs and the
  per-read state copy become O(change)); `Arc`/`Bytes` payloads throughout records; cached parsed
  transaction operations (YAML parses at ~12 MB/s, 40–64% of `propose`).
- SQLite settings (WAL, default durability, 2 MB cache, no mmap, indexes) are not the bottleneck:
  cold-open read syscalls take 0.2–0.37 s against 6.3 s CPU.

## Build

Split into stories when scheduled; each keeps its measurement as acceptance.
