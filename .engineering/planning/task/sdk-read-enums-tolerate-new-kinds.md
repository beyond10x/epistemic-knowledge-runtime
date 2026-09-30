---
format: aep.planning-md/3
id: task:sdk-read-enums-tolerate-new-kinds
kind: task
status: active
title: The SDK's read models tolerate a kind a newer ekr adds
relations:
- serves: vision:o5
- decomposes: epic:consumer-sdk
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T00:47:42Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T00:47:42Z", actor: "human:timo", revision: 3}
---
## Context

Found by the adversary pass on wave sdk-03 unit C (2026-09-30): `ekr_sdk::read` models `ChangeKind`
(the `changes` read) as a closed enum, so a change kind a newer `ekr` adds fails the whole read and
every change in the document is lost. `docs/sdk.md` promises that a read ignores what a newer `ekr`
adds, and `CodeNameKind` was given a catch-all for the same reason in unit C
(`CodeNameKind::Other`). `task:changes-since-lists-added-evidence` will add a change kind
(`EvidenceAdded`), which is the first case that reaches this.

## Build

- `ChangeKind` and any other closed enum in the SDK's read models get a catch-all, as
  `CodeNameKind` did; the engine-side exact read still reports the drift.
- `docs/sdk.md` states what a consumer sees for an unknown kind.

## Acceptance

- A changes document with a kind the SDK does not model reads, with that change's kind as the
  catch-all and every other change intact.
- The exact-read drift test still fails on the unknown kind.
