---
format: aep.planning-md/3
id: story:changes-since-by-time-within-deadline
kind: story
status: active
title: A changes query by recorded time costs what the same query by revision costs
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/commands.rs
- confidence: inferred
  path: crates/ekr-kernel/src/read.rs
- confidence: inferred
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-views/src/changes.rs
- confidence: cited
  path: crates/ekr-views/src/index.rs
- confidence: cited
  path: crates/ekr-views/src/lib.rs
- confidence: cited
  path: crates/ekr-views/tests/changes_by_time_cost.rs
- confidence: cited
  path: crates/ekr-views/tests/support/fixtures.rs
- confidence: inferred
  path: crates/ekr/src/cli/http.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T07:25:55Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T07:25:55Z", actor: "agent:claude-ekr-controller", revision: 3}
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

Derived 2026-10-21 by `story-scoper` against the tree of wave 2026-10-21b U1 (profile v4), read-only; nothing was run. Each line is **cited** (read in the tree) or **inferred**.

- **Primary surface:** `crates/ekr-views` (`changes_as`, `IndexCache::index`) and the counting test. cited
- **Files:**
  - `crates/ekr-views/src/changes.rs:660` `changes_as`: first revision `:669-696` (revision arithmetic `:670-689`, head read only when `after > at` `:672-674`; transaction time is an in-memory scan of `loaded.revisions` `:690-694`), chosen set `:697-701`, one `runtime.transactions()` call `:709-716`, seed `runtime.replay(SEED)` `:723-727`. cited
  - `crates/ekr-views/src/changes.rs:644-649` `Index::changes` delegates to `changes_as`. cited
  - `crates/ekr-views/src/index.rs:1091-1121` `IndexCache::index`: `runtime.head()` on every call `:1096-1098`, `Lineage::read` when the cache is non-empty `:1107-1110`, `Index::load` on a miss `:1114`. cited
  - `crates/ekr-views/src/lib.rs:307-320` `Lineage::read` calls `runtime.transactions()` `:309`. cited
  - `crates/ekr-views/tests/changes_by_time_cost.rs:38-68` `measure` (`Index::load` `:40` runs before the counters `:41`; counted call `:45-47`), `:93-150` test body, `:152-160` File and Sqlite only. cited
  - `crates/ekr-views/tests/support/fixtures.rs:87-105` `open`/`reopen`: File and Sqlite arms only. cited
  - `crates/ekr-kernel/src/commands.rs:206-237` `read_state`/`replayed_state`, `transactions`. cited
  - `crates/ekr-kernel/src/runtime.rs:172` `schema_history`, `:201` `transactions`, `:522` `head`, `:542` `replay`; `crates/ekr-kernel/src/read.rs:162` `schema_history` (complete `store.history()`). cited
  - `crates/ekr-store/src/eventlog.rs:984` `read_on`, `:1027` `occurrences`, `:2038` `history`, `:2047` `replay_history`, `:2180` `head`, `:2196` `replay`. cited
  - `crates/ekr/src/cli/mcp.rs:701-719` `changes_since`; `crates/ekr/src/cli/view.rs:1728-1765` HTTP `/changes`; `crates/ekr/src/cli/http.rs:20-21` constants, `:359-361` `alive`, `:506` deadline, `:518` `request deadline exceeded`. cited
- **Read counters that exist:** `ekr_store::read_work` and `stream_reads` (re-exported `crates/ekr-kernel/src/lib.rs:150`, `crates/ekr-store/src/lib.rs:97`), `Runtime::seed_replays` `runtime.rs:563`, `ekr_kernel::graphs_applied` `apply.rs:77`, `Index::transaction_documents_parsed` `changes.rs:625`. Production `/metrics` counts no store reads. cited
- **Also likely:** `crates/ekr/src/cli/http.rs` only if the cause is queue wait; `systems/ekr/domains/views.yaml:126-141` and `:1954-1990` only if a cost or semantics change is made. inferred
- **Confidence:** medium. Call paths cited; the cause is unmeasured.
- **Would collide with:** wave 2026-10-21b U1 and U2 (`crates/ekr-kernel/src/replay.rs`, `authority.rs`; sequential waves); the unmerged branch `unit/20261017b-u1` of issue #96 (`changes.rs`, `index.rs`, `http.rs`, `views.yaml`, `docs/cli.md`, `CHANGELOG.md`). cited
- **Safety fact:** the two `since` kinds reach identical store reads for the same chosen set (`changes.rs:697-716`), read and not run.

**Not established, decided by the coordinator on 2026-10-21:**

1. Which request the deadline measured. The two kinds share one read path for one chosen set, so window and limit are the inputs the report did not hold equal (`changes.rs:697-731`). Decision: the diagnosis measures per-request work through `IndexCache::index` on warm and cold caches as well as `Index::changes`, at the reported window and limit.
2. The acceptance may already pass at the `Index::changes` seam. Decision: the counted probes cover the path the MCP and HTTP readers take (`mcp.rs:717`, `view.rs:1758-1760`), not only `Index::changes`.
3. Provider coverage. `fixtures.rs:90-102` has no PostgreSQL arm; the report was on PostgreSQL. Decision: "both providers" in the acceptance means File and Sqlite, as every other counted test of this crate does; a PostgreSQL arm is recorded as not established.
4. A `since_recorded` before the seed has no `since_revision` equal; it stays out of the acceptance, as the test already prints it as a diagnostic (`changes_by_time_cost.rs:130-144`).
5. Queue cancellation (`http.rs:359-361`, `:506-518`) stays with issue #96's story.
6. The Context's citations `http.rs:448`, `:438`, `:15-16` are now `:518`, `:506`, `:20-21`.
