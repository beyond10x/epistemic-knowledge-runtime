---
format: aep.planning-md/3
id: task:stage-specified
kind: task
status: implemented
title: The stage is specified before it is built
relations:
- serves: vision:o5
- derived_from: story:a-run-is-staged-and-published-whole
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:53:46Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T08:53:47Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-08T03:23:30Z", actor: "agent:claude-ekr-controller", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Build

Model the stage in `systems/ekr/domains/store.yaml`: the stage and its id, begin, publish with an expected head, abandon, their outcomes and refusals, and the amendment of held-bytes rule 2 that admits `forget_tenant` for a stage tenant only. Write design amendment § 107 and an architecture decision record for publishing a staged suffix rather than rewinding a head.

## Acceptance

`ess specify validate --path systems/ekr` passes with ess 0.36.0 (the gate's pin) and with ess 0.55.0; the decision record and § 107 name the cases the later units add.
