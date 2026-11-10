---
format: aep.planning-md/3
id: story:changes-since-by-time-within-deadline
kind: story
status: implemented
title: A changes query by recorded time costs what the same query by revision costs
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
- decomposes: epic:zero-open-issues-2026-11
scope:
- confidence: cited
  path: crates/ekr-views/src/changes.rs
- confidence: cited
  path: crates/ekr-views/tests/changes_by_time_cost.rs
- confidence: cited
  path: crates/ekr-views/tests/support/fixtures.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T07:25:55Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T07:25:55Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-10T02:23:28Z", actor: "agent:claude-integrate-20261110", revision: 9, decided_on: {"recorded":{"test_result":1}}}
---
## Context

On 2026-10-04, ekr 0.0.30 `mcp-http` on the PostgreSQL provider answered `changes_since` with
`since_recorded` one week back and `limit=2000` with `request deadline exceeded`. That refusal is
`crates/ekr/src/cli/http.rs:448`, and the deadline is `READ_TIMEOUT` plus `WAIT_TIMEOUT`, 5 s + 30 s
(`http.rs:15-16`, `:438`). A `since_revision` request with `limit=500` on the same store answered. No cause is
established: the time query, the page size, the provider and the evidence entries in the window are all
candidates.

## Build

- Diagnose first: a red-capable loop that times `since_recorded` against `since_revision` over the same window
  on a synthetic store of comparable revision count and evidence volume, on both providers. Then probes that
  vary one input each.
- Fix the measured cause.

## Acceptance

- The diagnosis is recorded as evidence with its commands and numbers.
- A `since_recorded` request selecting the same revisions as a `since_revision` request costs no more reads
  than it, counted rather than timed, on both providers.

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-views` changes read (`Index::changes_as`) — cited (wave-2026-10-14a row T names `crates/ekr-views/src/changes.rs`)
- **Files:** `<tree>/crates/ekr-views/src/changes.rs:669-696` — cited; the `first` revision is chosen here per `SinceKind`
- **Files:** `<tree>/crates/ekr-views/tests/changes_by_time_cost.rs` (commit a49980997), `<tree>/crates/ekr-views/tests/support/fixtures.rs:31-35` — cited; the counted cost test already exists and names this story
- **Symbols:** `SinceKind::TransactionTime`, `ChangesRequest`, `LoadedRevisionEntry.committed_at` (`crates/ekr-views/src/lib.rs:249-253`) — cited
- **How the two differ:** `since_recorded` (`changes.rs:690-694`) scans the in-memory `loaded.revisions` for the first entry with `committed_at.millis() > since`, with no store read and `u64::MAX` when none matches; `since_revision` (`changes.rs:670-689`) takes `since + 1` and calls `runtime.head()` only when `since` is past the index's revision. Both then share one path from `changes.rs:697` — cited
- **Consequence:** resolution itself adds no provider read for the time query; it can only differ by choosing the seed (revision 0), which no `since_revision` can choose — inferred
- **Providers runnable locally:** "both providers" in this crate's tests is File and SQLite (`fixtures.rs:31-35`); no PostgreSQL in `crates/ekr-views` — cited
- **PostgreSQL:** kernel and CLI tests reach it only through `EKR_TEST_POSTGRES_CONFIG` and `EKR_TEST_POSTGRES_OWNER`, and skip when unset (`crates/ekr-kernel/tests/adversary_w20261007c_p.rs:26-41`, `crates/ekr/tests/postgres_cli.rs`) — cited
- **Acceptance without a hosted database:** the counted clause can be met on File and SQLite, if "both providers" is read as this repository's tests use it; the observed PostgreSQL timeout would stay unreproduced — inferred
- **Also likely:** `crates/ekr-store` read path, only if the diagnosis points there (wave row T) — inferred
- **Also likely:** `crates/ekr/src/cli/http.rs:20-21,518` — cited as the deadline site only, not an edit site; the story's `:448`, `:15-16`, `:438` have drifted
- **Documents:** evidence record of the diagnosis under `.engineering/`; `views.yaml` cost clause via another unit (wave row T) — inferred
- **Confidence:** medium — the tree shows the resolution site and an existing cost test, but the cause of the PostgreSQL timeout is unestablished and may sit outside `ekr-views`
- **Would collide with:** any unit touching `crates/ekr-views/src/changes.rs` (notably `story:changes-since-small-default-and-cached`, which references this story) or `tests/support/fixtures.rs` — cited
- **Safety fact:** `since_recorded` resolves from `loaded.revisions` already held by the index, so changing the resolution alters no stored bytes or wire field (`meta.since_recorded` is echoed at `changes.rs:777-788`) — step 2, unproven

## Diagnosis

Measured 2026-11-09 by the coordinator of `epic:zero-open-issues-2026-11` on the integration
branch at `d48cf0320`, no code changed:

`cargo test --locked -p ekr-views --test changes_by_time_cost -- --nocapture --test-threads 1`
printed `test result: ok. 2 passed; 0 failed` (File and SQLite). Per provider, for windows of 1, 10
and 40 revisions, `since_recorded` and `since_revision` choosing the same revisions printed the same
counts: `tenant-log reads 2`, `revision-stream reads 1`, `object-stream reads 1`,
`replays from the seed 0`, `changes built` equal to the window. The one probe that differs is a
`since_recorded` before the seed (revisions 0..=40), which no `since_revision` can choose:
`replays from the seed 1`, `revision-stream reads 2`, `tenant-log reads 3`, `changes built 43`.

So the counted acceptance holds on both providers this crate tests. Inferred, not measured: the
2026-10-04 PostgreSQL timeout is still unreproduced (no PostgreSQL configuration here; the
PostgreSQL tests skip with `EKR_TEST_POSTGRES_CONFIG` unset), and the probe above names one
candidate for it — a one-week window on a store seeded within that week chooses the seed and pays
a replay from the seed, which a `since_revision` request never does.
