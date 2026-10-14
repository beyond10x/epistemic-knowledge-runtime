---
format: aep.planning-md/3
id: decision-blocker:migration-marker-provenance
kind: decision-blocker
status: cleared
title: Migration marker identity needs a compatible specification
relations:
- blocks: task:migration-marker-cannot-be-evidence
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T07:29:19Z", actor: "agent:claude-ekr-controller", revision: 3}
---
Current migrate::finished recognizes fixed marker content addresses; arbitrary evidence can
contain those bytes and may be raised to Canonical. A storage-class-only predicate does not prove
migration provenance. kernel.yaml and the migration design specify the current markers. Clear
with a compatible specification for protected migration state and old-marker behavior, together
with executable same-bytes evidence and genuinely incomplete migration acceptance.

## Answer

Decided 2026-10-14: for `/2` and `/3` stores, evidence whose bytes equal a reserved migration marker is refused at validation by a named reserved-payload refusal. Legacy marker recognition is unchanged; `/4` copies stay as design § 105.2 says (a claim-bound `ekr.migration-finished/2` receipt). This is option A; `task:migration-marker-cannot-be-evidence`'s acceptance becomes "refused by name".
