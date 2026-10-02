---
format: aep.planning-md/3
id: task:code-names-matches-whole-words
kind: task
status: implemented
title: ekr code-names matches a name as a whole word anywhere
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:32Z", actor: "agent:codex-ekr-x7b", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-02T16:21:31Z", actor: "agent:codex-ekr-x7b-root", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
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

## Resume decisions (2026-10-02)

Word characters are Unicode alphanumeric characters or underscore. Match the exact case-sensitive
revision name only when each adjacent character is absent or is not a word character. This also
allows punctuation-containing names while preserving existing source locations, ordering and
exemptions. The default literal mode preserves its document bytes.

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
