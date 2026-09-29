---
format: aep.planning-md/3
id: story:store-reading-code-names-no-contents
kind: story
status: draft
title: A check reports store contents named in a consumer's code
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
scope:
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: inferred
  path: crates/ekr/src/exit.rs
- confidence: inferred
  path: crates/ekr/src/main.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/views-provenance.json
- confidence: inferred
  path: systems/ekr/conformance/views-suite.json
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 4
---
## Context

A consumer instance keeps a 246-line check that its store-reading code names none of the
store's contents, so code stays generic over any ontology (reported 2026-09-29). The runtime has
the same rule for its own viewer (the viewer's data-free rule, crates/ekr/tests/view_page.rs), but
nothing a consumer can run against its own code.

## Build

A verb that takes a store and a set of source files and reports every literal in the files that
equals a type name, property name, alias or canonical name in the store, with file and line;
exit 1 when any is found. Reads the store only.

## Acceptance

- On a fixture store and fixture sources, every planted name is reported with its file and line,
  and none of the store's ids or the runtime's own vocabulary is reported.
- The verb writes nothing.

## Scope

Derived 2026-09-29 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr/src/cli/` — a new one-shot verb module — inferred, the story asks for "a verb" and every verb body lives here
- **Files:** `crates/ekr/src/cli/mod.rs` — `Command` enum (`mod.rs:102`) and dispatch (`mod.rs:499-577`) gain the verb — cited
- **Files:** `crates/ekr/src/cli/session.rs` — the verb match at `session.rs:231-250` covers every verb, so a new `Command` variant must get an arm, served or refused — cited
- **Files:** `docs/cli.md` — `crates/ekr/tests/docs_cli.rs:12` requires the verb table to equal `ekr --help`, and `AGENTS.md:47` requires it — cited
- **Also likely:** `crates/ekr/src/main.rs`, `crates/ekr/src/exit.rs` — "exit 1 when any is found" has no path today: success prints and exits 0 (`main.rs:32-48`), and exit 1 is `Failure::Fault` only (`exit.rs:56`) — inferred
- **Also likely:** `crates/ekr/src/cli/agent.rs` — the `ekr guide` text lists the verbs (`agent.rs:40-64`) — inferred
- **Also likely:** `crates/ekr-views/src/` — the store's names are best read off `ekr.graph-projection/1`, as the viewer's data-free rule already does (`crates/ekr/tests/view_page.rs:7-10`) — inferred
- **ESS domain:** `systems/ekr/domains/views.yaml` — inferred; the other option is `kernel.yaml`
- **Conformance:** views scenarios and the regenerated `views-suite.json`, `views-provenance.json` (`Taskfile.yml:88-89`) — inferred
- **Tests:** a new `crates/ekr/tests/<verb>_cli.rs` with fixture stores and sources under `crates/ekr/tests/fixtures/` — inferred
- **Confidence:** medium — the three CLI files are forced by the code; the domain, suite and module placement are not named by the story
- **Would collide with:** any unit adding an `ekr` verb (`cli/mod.rs`, `cli/session.rs`, `docs/cli.md`, `cli/agent.rs`); any unit adding `ekr.views` scenarios; any unit changing exit-code mapping
- **Open:** exit 1 means a fault today; reporting findings with exit 1 needs a new exit path or a different code, decided in the ESS draft.
