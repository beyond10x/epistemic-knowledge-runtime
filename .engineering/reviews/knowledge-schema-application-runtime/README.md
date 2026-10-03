# Schema application runtime checkpoint

This checkpoint implements exact approved **schema-only** application through ordinary
Propose, Validate and Commit, with immutable application/step/attempt elections and atomic
review-stream publication markers. The CLI and typed SDK invoke that same path. Repeated
application, reopening and full replay preserve the committed identities and evidence.

This is unfinished Story F. Source/observation-backed proposals, selected mappings, corrections,
terminal-stale successors and application-aware residual review remain refused or incomplete.
No full conformance, demonstration, `task check`, PR readiness or completion is claimed.
The separate finite recursive fixture synthesis blocker is tracked in ESS issue
[416](https://github.com/beyond10x/ess/issues/416).

## Independent findings

The two independent source reviews are immutable AEP records
`review-result:schema-application-authority-independent-r1` and
`review-result:schema-application-authority-independent-r2`. Their authors did not execute tests.
The coordinator reproduced and fixed all three findings:

- Applying before the approval timestamp retained an invalid election. Chronology now refuses
  before retention; the regression requires unchanged events and a successful cold full replay.
- An election/step transaction without its attempt could escape the application guard.
  Command attachment, replay and new preparation capture now reserve those identities. The
  regression uses the original real transaction document, refuses without publication and
  successfully resumes both genuine interrupted prefixes.
- A new application could adopt an existing ordinary pending Propose preparation. Physical
  election/step admission now refuses that identity; the original preparation must remain
  readable after reopening and resume unchanged.

The first reservation test draft used an empty operation list and was not a valid reproduction;
only the replacement using the genuine elected document establishes red/green evidence.
Raw failing logs contain synthetic signed fixtures and remain outside public source.

## Verification scope

`verification.txt` preserves the actual runner summaries and SHA-256 addresses of private raw
logs. The complete kernel package run predates these review fixes; focused kernel regressions,
the complete graph/store run and CLI/SDK execution follow the fixes. Strict clippy covered all
targets of kernel, store, SDK and CLI. Workspace formatting uses the repository xtask and leaves
generated dependency bytes unchanged.

The copied worker reports describe their frozen units, not the final integrated source.
In particular, codec/projection dead-code lint failures in those historical reports were cleared
when production callers were integrated. Store tests with an injected synthetic authority are
physical contract tests, not kernel conformance. The integrated kernel tests use real authority,
external-human signatures, ordinary transactions and both file and SQLite providers.

The physical audit requires a readable complete tenant feed and remains proportional to that
feed's history. Exact historical captures exclude future application records and payloads;
current captures still refuse malformed or unavailable retained material. A native atomic-group
identifier is unavailable, so matching request identifiers are never treated as group proof.

Source work remains on the existing managed EKR branch for the sole PR64 carrier. No ESS source,
release branch or tag is changed by this checkpoint.
