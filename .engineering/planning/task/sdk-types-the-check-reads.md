---
format: aep.planning-md/3
id: task:sdk-types-the-check-reads
kind: task
status: active
title: The SDK types ekr quality, ekr rejections and ekr code-names
relations:
- serves: vision:o5
- decomposes: epic:consumer-sdk
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T23:07:48Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-29T23:07:49Z", actor: "human:timo", revision: 3}
---
## Context

0.0.21 added three store checks as reads, each a one-shot verb and an `ekr session` verb:
`ekr quality` (`ekr.store-quality/1`), `ekr rejections` (`ekr.rejections/1`) and `ekr code-names`
(`ekr.code-names/1`). `story:sdk-store-checks` asks for typed SDK calls for them and for the judged
fact-quality sample, whose verb (`story:fact-quality-by-judged-sample`) does not exist yet. This task
is the part of that story whose verbs exist; the story keeps the judged-sample part.

## Build

- `ekr_sdk::read` gains typed calls and serde models for the three documents: quality at the head or
  a revision, rejections over an optional basis range, code-names over a list of source paths at the
  head or a revision. They run through a session and as one-shot reads, as the kernel reads already
  do (`Reader` and `one_shot`).
- The models ignore fields a newer `ekr` adds, as the other read models do; a drift test against the
  views and kernel conformance fixtures or real `ekr` output fails when a format gains a field the
  SDK does not model.

## Acceptance

- Each SDK result equals the verb's document: the typed value written back equals the verb's JSON,
  through a session and one-shot, on both providers.
- A field added to one of the three formats without an SDK update fails an engine test.
- `code-names` through the SDK reports a planted store name with its file and line, and the
  `runtime_word` flag.
