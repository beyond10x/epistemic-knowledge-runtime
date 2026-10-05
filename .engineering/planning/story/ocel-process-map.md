---
format: aep.planning-md/3
id: story:ocel-process-map
kind: story
status: draft
title: A store's OCEL log can be read as variants and a directly-follows graph
tags:
- consumer:cortex
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-sdk/src/read/ocel.rs
- confidence: inferred
  path: crates/ekr-views/src/lib.rs
- confidence: cited
  path: crates/ekr-views/src/ocel.rs
- confidence: inferred
  path: crates/ekr-views/src/process_map.rs
- confidence: inferred
  path: crates/ekr-views/tests/process_map.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/cli/process_map.rs
- confidence: inferred
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: inferred
  path: crates/ekr/tests/process_map_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: docs/sdk.md
- confidence: inferred
  path: systems/ekr/conformance/views-suite.json
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 4
---
## Outcome

A store's events can be read as a process: its variants and a directly-follows graph, derived from
the store's own OCEL 2.0 log.

## Starting point (0.0.30)

`ekr ocel` exports one revision as an OCEL 2.0 log (`ekr.ocel/1`). Turning that log into process
views (variants, directly-follows counts, handovers between actors) is done today by each consumer
in its own code.

## Acceptance

`ekr process-map` (or a view of the same output) over a store whose OCEL log has 3 cases following
two variants prints both variants with their counts and a directly-follows graph whose edge counts
match the log; the output format is documented like `ekr.ocel/1`.

## Consumer

An organisation-scale consumer that builds these views from its OCEL export today; moving the
generic part here lets it drop that code.

## Scope

Derived 2026-10-05 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-views` (the pure view over `ekr.ocel/1`) and `crates/ekr` (the `ekr process-map` verb) — inferred, the shape of `ekr ocel` (ec6bbf38f, ae2eb4c7a); the story names only the verb and the format
- **Files:** `crates/ekr-views/src/ocel.rs:28` (`OCEL_FORMAT`), `:224` (pure `ocel`, the log the map is derived from) — cited, the story's "derived from the store's own OCEL 2.0 log"
- **Files:** `crates/ekr-views/src/lib.rs:46,63,79-81` (module doc, `mod ocel`, re-exports; a new module and `pub use` go here) — inferred
- **Files:** `crates/ekr/src/cli/mod.rs:51` (mod list), `:354-370` (`Command::Ocel`, the new variant sits beside it), `:585` (read-access arm), `:914-921` (dispatch) — inferred
- **Files:** `crates/ekr/src/cli/session.rs:759` (session read-verb arm) — inferred
- **Files:** `crates/ekr/src/cli/agent.rs:69-75` (verb card) and `crates/ekr/tests/agent_cli.rs:842-850` (card assertions) — inferred
- **Files:** new `crates/ekr-views/src/process_map.rs`, `crates/ekr/src/cli/process_map.rs`, `crates/ekr-views/tests/process_map.rs`, `crates/ekr/tests/process_map_cli.rs` — inferred, none exist yet
- **Symbols:** `ekr_views::export_ocel`, `ocel`, `OcelExported`, `OCEL_FORMAT`, `Command::Ocel` — cited (`ocel.rs:28`, `lib.rs:79-81`, `cli/mod.rs:365`)
- **Documents:** `docs/cli.md:164` (verb table) and a new `### ekr process-map` section next to `:555` — cited, Acceptance says "the output format is documented like `ekr.ocel/1`"; the verb table is held to `ekr --help` by `crates/ekr/tests/docs_cli.rs:13` — cited
- **Also likely:** `systems/ekr/domains/views.yaml:1595` (`ekr.ocel/1`; `ExportOcel` at `:2010`, `:2618`) and the regenerated `systems/ekr/conformance/views-suite.json` — inferred, needed only if the map is a declared view command
- **Also likely:** `crates/ekr-sdk/src/read/ocel.rs`, `docs/sdk.md`, `CHANGELOG.md` — inferred
- **Confidence:** medium — the verb, the input format and the doc requirement are cited; every file past `ocel.rs` comes from the last two OCEL commits, not from the story
- **Would collide with:** any unit that edits the `ekr` CLI `Command` enum and dispatch (`crates/ekr/src/cli/mod.rs`), the agent card (`cli/agent.rs`, `tests/agent_cli.rs`), the `docs/cli.md` verb table, `crates/ekr-views/src/lib.rs` re-exports, or `systems/ekr/domains/views.yaml`
- **Safety fact:** the map only reads the `ekr.ocel/1` document that `ocel()` (`crates/ekr-views/src/ocel.rs:224`) returns, so `ekr ocel`'s output and its pinned SHA-256 (`crates/ekr-views/tests/ocel.rs:37`) stay the same — level 2, unproven
- **Open for the implementor:** whether the map is a declared view in `views.yaml` with conformance (as `ExportOcel` is) or a CLI step after `ekr ocel`; whether "or a view of the same output" means an MCP tool or viewer route (`cli/mcp.rs`, `cli/view.rs`); handovers are in Starting point but not in Acceptance, so out of scope
