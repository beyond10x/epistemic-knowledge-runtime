---
format: aep.planning-md/3
id: story:a-run-is-staged-and-published-whole
kind: story
status: active
title: A run is staged, and published whole or dropped whole
relations:
- serves: vision:o5
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/migrate.rs
- confidence: inferred
  path: crates/ekr-store/src
- confidence: inferred
  path: crates/ekr/src/cli
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: systems/ekr/domains/store.yaml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T08:53:46Z", actor: "agent:claude-ekr-controller", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-07T08:53:46Z", actor: "agent:claude-ekr-controller", revision: 9}
---
## Outcome

A consumer that runs one-shot `ekr` verbs can make a run all-or-nothing: the run's commits go to a
stage, the run's reads see them, and the stage is published into the store whole, only if the
head has not moved, or dropped whole. A failed run leaves `ekr head` where it was.

## Why

A consumer runs batches of `ekr ontology`, `ekr apply-extraction`, `ekr snapshot`, `ekr mint`,
`propose`, `validate` and `commit` as separate processes, then gates the run on `ekr quality`.
On SQLite it undoes a failed run by restoring a file snapshot; on PostgreSQL nothing returns a
store's head to an earlier revision. A rewind of the head would delete committed revisions, which
invariant 5 allows only through the maintenance path, and Eventlog's PostgreSQL provider has no
event-range deletion.

Design § 105.3 already names the missing piece: atomic incremental suffix publication into a
served store, with a captured base, an expected-head condition, all-or-nothing suffix
validation and publication, and exact retry behaviour. This story builds it, with a stage as the
captured base.

## Shape

- A stage is its own Eventlog tenant in the same store, begun from the store at its head by a
  preserving copy (design § 100.3, § 105.2). PostgreSQL becomes a supported copy source for this.
- Every store verb joins a stage named by `EKR_STAGE=<id>` or `--stage <id>`; it then reads and
  writes the stage's tenant, so ontology, snapshot, quality and every write see the stage's
  commits.
- `ekr stage publish <id> --expect-head <revision>` validates the stage's suffix against the
  store at that head and publishes it in one append group, or refuses by name when the head has
  moved; retried after an uncertain outcome it adopts its own earlier publication.
- `ekr stage abandon <id>`, and a published stage, remove the stage's tenant with the provider's
  `forget_tenant`. The store's rule that no runtime operation forgets retained bytes in place
  (`systems/ekr/domains/store.yaml`, held bytes, rule 2) is amended for a stage tenant only; the
  store's own tenant is never forgotten.

## Acceptance

- On a PostgreSQL store, a run made in a stage whose gate fails is abandoned, and `ekr head`
  afterwards equals `ekr head` before the run; the same on SQLite.
- A run that passes is published: `ekr head` is the stage's last revision re-derived for the
  store, and replay of the store verifies.
- A publish whose expected head is not the store's head is refused by name and changes nothing.
- Every verb above works with the stage named only by `EKR_STAGE` in the environment.
- After abandon or publish, the stage's tenant holds nothing.
- Each named case runs on SQLite and on PostgreSQL where the repository's PostgreSQL tests run.

## Units

`task:stage-specified` (the specification, design amendment and decision record),
`task:postgres-source-copy`, `task:stage-suffix-publication`, `task:stage-cli`, in that order.
