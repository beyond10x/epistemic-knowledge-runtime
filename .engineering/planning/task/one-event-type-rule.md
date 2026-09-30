---
format: aep.planning-md/3
id: task:one-event-type-rule
kind: task
status: draft
title: One rule decides which node types are events
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

EKR holds two definitions of an event type (found by wave ops-05 unit O, 2026-09-30):

- the timeline's rule in `crates/ekr-views/src/index.rs:599`
  (`timing.event = judged > 0 && 5 * instant >= 3 * judged`), used by the viewer's timeline and now by
  `ekr ocel`;
- the structural rule behind `GET /roles` (`crates/ekr/src/cli/view_roles.rs`, `docs/cli.md` § Roles).

Two consumers asking "which types are events" can get different answers from one store.

## Build

- One rule, in `ekr-views`, specified in `systems/ekr/domains/views.yaml`; the other place reads it.
  Which rule wins is decided with the evidence of both on real stores (counts of types each marks).
- `docs/cli.md` names the one rule in § Roles, § `ekr ocel` and the timeline.

## Acceptance

- `/roles`, the timeline and `ekr ocel` agree on the event types of every views fixture.
