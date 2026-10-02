---
format: aep.planning-md/3
id: task:code-names-matches-whole-words
kind: task
status: active
title: ekr code-names matches a name as a whole word anywhere
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:32Z", actor: "agent:codex-ekr-x7b", revision: 4}
---
## What is wrong

A consumer (2026-10-02): `ekr code-names` checks quoted string literals only, so a type or edge name
written bare in a comment or as an identifier (`ZORBED_BY`) is not reported, while a backticked
property name is. The consumer's data-free rule needs whole-word matches anywhere in a line.

## Decision (coordinator, 2026-10-02)

A second match mode, `--words`, reports a revision name wherever it appears as a whole word in a
line (identifiers, comments, backticks), beside the default literal mode, whose output stays as it is.

## Acceptance

- `ekr code-names --words` reports `ZORBED_BY` in a comment and as an identifier, and does not
  report a name inside a longer word.
- The default mode's output is byte-identical for the existing fixtures; the mode is in
  `ekr.code-names/1`'s spec and the SDK's typed call.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr-views/src/code_names.rs`
- `crates/ekr/src/cli/mod.rs`
- `crates/ekr/src/cli/code_names.rs`
- `crates/ekr-sdk/src/read/checks.rs`
- `systems/ekr/domains/views.yaml`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.
Typed task scope is unavailable: AEP 0.64.0 restricts the scope field to stories.
