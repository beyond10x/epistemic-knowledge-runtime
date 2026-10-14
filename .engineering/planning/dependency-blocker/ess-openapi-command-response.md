---
format: aep.planning-md/3
id: dependency-blocker:ess-openapi-command-response
kind: dependency-blocker
status: cleared
title: ESS OpenAPI omits a command's response from its success body (beyond10x/ess#423)
refs:
- provider: github
  reference: beyond10x/ess#423
- provider: github
  reference: beyond10x/ess#424
relations:
- blocks: story:read-surface-specified
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T13:04:42Z", actor: "agent:claude-ekr-controller", revision: 3}
---
ESS 0.52.0 `ess generate --kind openapi` leaves a command's `response:` fields out of every success body, also
when the outcome declares `returns: true` (`format: ess/17`). In ESS `main` at 5323e4c7ce,
`crates/specify/ess-compiler/src/resolve.rs:2180` derives `retains_result` from `replays` alone. A 3-file
synthetic reproduction is in beyond10x/ess#423. A synchronous read also projects `202`
(beyond10x/ess#424). That does not block, and is accepted until fixed.

Clears when an ESS release generates the response document into the success body of a `returns: true`
outcome.

## Answer

Cleared 2026-10-14. The fix shipped in ESS 0.53.0 (https://github.com/beyond10x/ess/pull/431) and applies from specification format `ess/22`: below it, an outcome with `returns: true` keeps its 202 body by design. The issue's own reproduction, raised to `format: ess/24` and generated with ess 0.59.0, emits `catalogue.search.FindTitles.Result` with the response fields in the success body. This repository's `systems/ekr/system.yaml` declares `ess/14` and the gate pins ess 0.36.0, so `story:read-surface-specified` first raises the format to `ess/22` or later, which needs the repository's ESS pin moved to a release that reads it.
