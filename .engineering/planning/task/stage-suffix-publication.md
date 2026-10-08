---
format: aep.planning-md/3
id: task:stage-suffix-publication
kind: task
status: implemented
title: A stage's suffix is published whole into the store, or refused
relations:
- serves: vision:o5
- derived_from: story:a-run-is-staged-and-published-whole
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:53:47Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T08:53:47Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T03:23:30Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Build

Publish the stage's revisions after its base into the store: re-derived for the store's lineage, validated through the kernel against the store at the expected head, and appended in one group; retry adopts its own earlier publication; then forget the stage's tenant.

## Acceptance

A publish at the expected head makes the stage's last revision the store's head and replay verifies; a publish at a moved head is refused by name and changes nothing; a retried publish appends nothing twice; after it the stage's tenant holds nothing.
