---
format: aep.planning-md/3
id: task:read-verbs-open-a-read-only-store
kind: task
status: draft
title: Read verbs answer on a store the caller may not write
relations:
- serves: vision:o5
revision: 1
---
## Context

Found by the adversary pass on wave checks-01 unit N (2026-09-30): on a store whose files the caller
may read but not write, `ekr code-names`, `ekr head` and `ekr ontology` exit 1 on both providers:
`ekr: opening the provider: the store is unavailable: event store is unavailable: Permission denied
(os error 13)`. The store's bytes stay unchanged. Read verbs therefore cannot serve a store mounted
read-only or owned by another user, such as a consumer's promoted store read by a checker.

## Build

- Opening a store for a read verb does not require write access: the file provider and the SQLite
  provider open read-only when the verb only reads, and a write verb on such a store is refused by
  name rather than as an unavailable store.
- Which verbs read only is taken from the verb table, not from a second list.

## Acceptance

- `ekr head`, `ekr ontology`, `ekr snapshot` and `ekr code-names` answer on a read-only store on
  both providers, with the same bytes as on a writable one.
- `ekr propose` on a read-only store is refused with a named refusal, exit 2.
