---
format: aep.planning-md/3
id: dependency-blocker:ess-openapi-command-response
kind: dependency-blocker
status: open
title: ESS OpenAPI omits a command's response from its success body (beyond10x/ess#423)
refs:
- provider: github
  reference: beyond10x/ess#423
- provider: github
  reference: beyond10x/ess#424
relations:
- blocks: story:read-surface-specified
revision: 1
---
ESS 0.52.0 `ess generate --kind openapi` leaves a command's `response:` fields out of every success body, also
when the outcome declares `returns: true` (`format: ess/17`). In ESS `main` at 5323e4c7ce,
`crates/specify/ess-compiler/src/resolve.rs:2180` derives `retains_result` from `replays` alone. A 3-file
synthetic reproduction is in beyond10x/ess#423. A synchronous read also projects `202`
(beyond10x/ess#424). That does not block, and is accepted until fixed.

Clears when an ESS release generates the response document into the success body of a `returns: true`
outcome.
