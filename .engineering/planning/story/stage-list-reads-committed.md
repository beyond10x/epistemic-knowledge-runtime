---
format: aep.planning-md/3
id: story:stage-list-reads-committed
kind: story
status: implemented
title: A stage listing names every stage whose begin committed
relations:
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T21:34:49Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T21:34:50Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T00:48:10Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`ekr stage list` lists every stage whose begin has committed, on PostgreSQL as on SQLite, whatever
other transaction is open on the server: the listing reads its own writes, as `ekr.cli.Stages`
declares.

## Why

Issue #94. `StageLog::stages` (`crates/ekr-store/src/stage_record.rs`) finds stage ids through the
event store's watermarked published feed, so on PostgreSQL a committed `StageBegun` is hidden while
any older transaction on the server is open. `ekr stage show`, `seal` and `abandon` read the stage's
stream directly and are unaffected.

## Acceptance

- `crates/ekr-kernel/tests/stage_hosted_limits.rs`
  `a_begun_stage_is_listed_while_another_transaction_is_open` passes, where it fails on 0.0.35;
  `a_begun_stage_is_listed_with_no_other_transaction_open` stays green.
- The listing names each stage with the state its own record holds (Begun, Sealed, Published,
  Abandoned), equal to what `ekr stage show` answers for it, on both providers.
- The existing stage listing tests (`crates/ekr-kernel/tests/stage.rs`, `crates/ekr/tests/stage_cli.rs`)
  stay green.
