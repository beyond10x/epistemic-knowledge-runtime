---
format: aep.planning-md/3
id: decision-blocker:migration-marker-provenance
kind: decision-blocker
status: open
title: Migration marker identity needs a compatible specification
relations:
- blocks: task:migration-marker-cannot-be-evidence
revision: 1
---
Current migrate::finished recognizes fixed marker content addresses; arbitrary evidence can
contain those bytes and may be raised to Canonical. A storage-class-only predicate does not prove
migration provenance. kernel.yaml and the migration design specify the current markers. Clear
with a compatible specification for protected migration state and old-marker behavior, together
with executable same-bytes evidence and genuinely incomplete migration acceptance.
