---
format: aep.planning-md/3
id: task:lineage-keys-members-by-kind
kind: task
status: implemented
title: The lineage keys schema members by kind and id
relations:
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T08:12:45Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T08:12:45Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-30T12:13:41Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Context

Wave reads-04 unit L's adversary pass: the lineage's member diff (`crates/ekr-views/src/index.rs:743`)
keys ids by their text alone, so a version that adds a property or node type whose UUID another id
kind already uses lists nothing in `added`, `removed`, `widened` or `modified`, and the viewer calls
a changed version empty. `Ontology::load` does not refuse a UUID reused across id kinds; nothing
found produces one.

## Build

Key the lineage's members by (kind, id).

## Acceptance

- The adversary's ignored case `a_version_adding_an_id_another_kind_already_uses_is_not_listed_as_empty`
  (`crates/ekr-views/tests/adversary_lineage.rs`) passes.
- Byte identity for stores without such ids holds.
