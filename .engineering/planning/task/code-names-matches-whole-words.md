---
format: aep.planning-md/3
id: task:code-names-matches-whole-words
kind: task
status: draft
title: ekr code-names matches a name as a whole word anywhere
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 1
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
