---
format: aep.planning-md/3
id: story:explain-scoped-to-cited-evidence
kind: story
status: implemented
title: explain answers only the cited evidence within a whole-answer cap, as ekr.explanation/3
summary: 'Scoped explanation with omitted_evidence and a 48 KiB cap (ekr-mcp-fixes-20261015 #7).'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:names-inline-in-read-answers
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr/src/cli/explain.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/tests/explain_by_reference.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T13:21:17Z", actor: "agent:ekr-controller-planner", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T13:21:17Z", actor: "agent:ekr-controller-planner", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-16T16:00:40Z", actor: "agent:claude-ekr-controller", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Context

A consumer's measured patch (ekr-mcp-fixes-20261015, item 7), measured on 0.0.34:

- `ekr.explanation/2` lists every Evidence link of the transaction that added the assertion: 247
  links and 125 KB for an assertion citing one;
- with `documents`, 338 KB in 5.2 s;
- with `documents` and `limit=6000`, 2.7 MB, because `limit` caps each record and not the answer.

The patch made this an MCP-only answer (`explain::run_scoped`). The rule that every MCP answer is byte
for byte what the verb answers stays, so the scoped answer is a new explanation format version that
`ekr explain` serves too, and `ekr.explanation/2` stays served.

## Build (spec first)

1. **`systems/ekr/domains/kernel.yaml` first** (where `ekr.explanation/2` is declared), green under
   `ess specify validate --path systems/ekr` and the conformance suites before any code. It extends
   `ekr.explanation/3`, which `story:names-inline-in-read-answers` declares with its `names` map;
   both land in this wave's one release, so no released /3 lacks these fields:
   - an input choosing the evidence scope, `cited` or `all`. `cited` answers only the Evidence links
     an Assertion or Attachment link of the chain cites, and counts the rest as `omitted_evidence`;
     `all` answers every link, and `omitted_evidence` is 0;
   - with `documents`: no proposal `record`, and a base64 `payload` dropped where its `text` is
     answered;
   - a whole-answer cap of 48 KiB: while the document is over it, the longest evidence text is
     halved at a character boundary and marked `truncated`. The cap and the order of halving are
     written out so two answers to one request are byte-identical;
   - the per-surface default: the MCP `explain` answers /3 with `cited` unless asked otherwise;
     `ekr explain` answers /2 unless asked for /3. The spec states that the MCP answer equals
     `ekr explain`'s for the same request with format and scope named explicitly.
2. `crates/ekr/src/cli/explain.rs` (and `crates/ekr-kernel/src/explain.rs` where the format constant
   and chain types live): the scope, the cap, the selector. `crates/ekr/src/cli/mcp.rs`: the default
   and the tool's input schema.
3. `docs/cli.md`, `CHANGELOG.md`.

## Acceptance

Each case names "ekr-mcp-fixes-20261015 #7".

- `crates/ekr/tests/explain_by_reference.rs` (or a new file),
  `a_scoped_explanation_answers_only_the_cited_evidence`: an assertion committed by a transaction
  with 40 evidence entries that cites one. `ekr explain --format ekr.explanation/3` with scope
  `cited` answers exactly one Evidence link and `omitted_evidence: 39`; with scope `all`, 40 links
  and `omitted_evidence: 0`.
- `a_scoped_explanation_with_documents_stays_under_its_cap`: the same with `documents` and evidence
  texts totalling over 48 KiB answers at most 48 KiB, with the halved texts marked `truncated` and no
  proposal `record`.
- `ekr explain` with no selector prints `ekr.explanation/2`, byte for byte what it printed before
  (`crates/ekr/tests/explain_bounds_documents.rs` keeps passing unchanged).
- `crates/ekr/tests/mcp.rs`, moved, not deleted: `explain` in
  `each_tool_answers_the_document_its_read_returns_on_the_same_store_state` and in
  `a_refusal_is_a_tool_error_with_its_name_and_the_server_keeps_serving`, compared byte for byte with
  `ekr explain` asked for /3 and `cited` explicitly.
- If the kernel's explain conformance scenarios cover the format, each admitted one passes under /2
  and /3.

## Scope

- `systems/ekr/domains/kernel.yaml`
- `crates/ekr/src/cli/explain.rs`, `crates/ekr-kernel/src/explain.rs` (inferred)
- `crates/ekr/src/cli/mcp.rs`
- `crates/ekr/tests/mcp.rs`, `crates/ekr/tests/explain_by_reference.rs` or a new test file
- `docs/cli.md`, `CHANGELOG.md`

## Overlaps and order

Shares `kernel.yaml` and `cli/explain.rs` with `story:names-inline-in-read-answers`, after which it
lands; `mcp.rs`, `crates/ekr/tests/mcp.rs`, `docs/cli.md` and `CHANGELOG.md` with every story of the
wave.

## Out of scope

Changing `ekr.explanation/2`; the size of a proposal record outside `documents`.
