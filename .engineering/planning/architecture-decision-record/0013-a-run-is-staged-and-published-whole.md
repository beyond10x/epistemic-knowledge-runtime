---
format: aep.planning-md/3
id: architecture-decision-record:0013-a-run-is-staged-and-published-whole
kind: architecture-decision-record
status: accepted
title: ADR 0013 — A run is staged and its suffix published whole; a head is never rewound
summary: A batch of one-shot verbs writes to a stage tenant and is published into the store in one append group, or the stage is forgotten; no operation moves a store's head backwards
relations:
- decides: story:a-run-is-staged-and-published-whole
revision: 4
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-08T03:23:31Z", actor: "agent:claude-ekr-controller", revision: 4}
---

## Status

Proposed on 2026-10-07 with design § 107, the stage model in `systems/ekr/domains/store.yaml` and
its commands in `systems/ekr/domains/cli.yaml` (`task:stage-specified`). The owner's decisions on
that unit's open questions are recorded below. Units C, P and L of wave 2026-10-07c execute it:
the PostgreSQL capture and copy (`task:postgres-source-copy`), the suffix publication
(`task:stage-suffix-publication`) and the CLI (`task:stage-cli`). Design § 107.12 and § 107.13
record what those units decided where § 107 was silent or wrong.

## Context

A consumer runs a batch of separate one-shot `ekr` processes — `ontology`, `apply-extraction`,
`snapshot`, `mint`, `propose`, `validate`, `commit` — and gates the batch afterwards on
`ekr quality`. A batch that fails its gate must leave `ekr head` where it was; a batch that passes
must land whole. On SQLite the consumer restores a file snapshot. On PostgreSQL nothing returns a
store's head to an earlier revision.

Three facts constrain the answer:
- Committed revisions are immutable (design § 6.8, invariant 5 of `AGENTS.md`); physical
  reclamation happens only through the maintenance path.
- The pinned Eventlog PostgreSQL provider has no event-range deletion. Its withdrawals are
  `redact`, `delete_blob` and `forget_tenant`.
- Design § 105.3 already names the missing capability: atomic incremental suffix publication into
  a served store, with a captured base, an expected-head condition, all-or-nothing suffix
  validation and publication, and exact retry behaviour.

## Decision

A run is staged, and its suffix is published whole or dropped whole. The head of a store is never
rewound.

- **A stage is its own Eventlog tenant in the same store.** It references the store it was begun
  from: a minimal `ekr.store.Store`, identified by its tenant, with its provider kind and nothing
  that moves with its head. The tenant name is derived deterministically from the store's tenant
  and the stage id. It carries a marker reserved to stages, so it can never be a store's tenant.
- **Begin mints the stage id** (the `Id::mint()` family) and returns it; the caller never supplies
  one. Begin then makes the preserving copy of the store at its head (§§ 100.3, 105.2), not a byte
  copy. PostgreSQL becomes an admitted copy source for a stage, read under one provider capture as
  SQLite is read under one image. The File provider refuses stages by name, because its
  `forget_tenant` rewrites its one journal.
- **The stage's record** is a private stream per stage in the store's own tenant. It holds only
  small events (begun, sealed, published, abandoned) and no blob, and stays after the stage's
  tenant is forgotten, as the stage's history.
- **Every store verb joins a stage** named by `--stage <id>` or `EKR_STAGE=<id>`, and then reads
  and writes the stage's tenant. A joined verb is refused by name once the stage is not Begun.
- **`publish --expect-head <revision>` first seals the stage**, then captures its suffix. Each
  joined write reads the record again after it lands. A write reported successful therefore landed
  before the seal and is in the suffix.
- **The publication requires the store's head to equal both the expected head and the stage's
  base.** A moved store is refused, never rebased. The suffix is derived again for the store's
  lineage and checked through the kernel against the store at that head. It is appended, with
  `StagePublished` carrying the published revision range by id, in one Eventlog append group in the
  store's tenant. The election is a publication preparation of a new kind, keyed by Publish and the
  stage id, whose native request is the whole group, in a new format,
  `ekr.publication-preparation/4`. A retry adopts its own publication. Then the stage's tenant is
  forgotten.
- **`abandon` records the stage Abandoned**, then forgets its tenant with the provider's
  `forget_tenant`. Held-bytes rule 2 admits `forget_tenant` for a stage's tenant only; the store's
  own tenant is never forgotten.
- **A stage's revisions are provisional.** They are not the store's committed revisions, and become
  committed only when published into the store. The stage owns them, and forgetting the stage
  removes them. Forgetting a stage's tenant reclaims no committed revision and no Canonical object
  of the store, so invariant 5 is untouched.

## Alternatives

**Rewind the head, guarded by an expected head** (`rewind --to R --expect-head H`). Refused. It
deletes committed revisions R+1..H, which invariant 5 forbids outside the maintenance path, and
the PostgreSQL provider cannot delete an event range. The expected-head guard decides only that
nobody else committed meanwhile; it does not make the deletion admissible. A reader, session or
replay checkpoint that already observed H would then hold revisions the store no longer has. That
is the "live handle answering from bytes a fresh handle refuses" that held-bytes rules 1 and 2
exist to prevent.

**A restore point**: copy the store aside before the run and copy it back on failure. This
generalises what consumers do with SQLite files today. It is refused. Restoring replaces a store's
history in place: on SQLite every live handle refuses it as `store-replaced` (rule 1), and on
PostgreSQL it needs `forget_tenant` of the store's own tenant followed by a second copy, with an
empty or partial store visible in between. Any commit another writer made during the run is
silently lost, with no named refusal. A restore is a rewind with more steps.

**A stage inside one session process**: hold the run in `ekr session` and commit it at the end.
Refused. The consumer's verbs are separate one-shot processes, so an in-process stage cannot span
them without restructuring the consumer. A crash loses the whole run, with nothing durable to
resume or inspect. The end of the session would still need an all-or-nothing publication of many
revisions, which is the suffix publication decided here, so it adds a constraint and removes
nothing.

## Consequences

- **Cost of begin.** Each begin costs a full preserving copy of the store into the stage's tenant:
  time and space proportional to the store. Unmeasured.
- **Cost of a cold open.** Each cold open of a stage takes the checked path for a migrated head,
  not the legacy fast head. Its cost per one-shot verb is unmeasured.
- **A moved store.** A publication is refused when the store's head moved after the base. A store
  with concurrent writers forces the run to be made again in a new stage; no rebase is offered. A
  head that moves after the seal leaves the stage Sealing, unpublishable, to be abandoned.
- **A lost begin answer.** Because begin mints the id, a begin whose answer is lost leaves a stage
  the caller cannot name until it reads the store's stage records (`ekr.store.Stages`).
- **The store's tenant.** It keeps every stage's small record events, outside the revision stream.
  `ekr head`, snapshots and checkpoints do not move for them.
- **Published revisions.** They carry the stage's event and revision identities, actors, times and
  knowledge, evidence, ontology and authority roots. Their record addresses are the store's, as
  for a migration (§ 100.4).
- **Held-bytes rule 2.** It gains its first `forget_tenant`. The source guard
  `no_source_withdraws_retained_bytes_without_an_event_on_the_object_stream` admits the one call in
  the change that adds it. A handle joined to a stage reads the stage record before every read and
  write, because rule 1 cannot see a forgotten tenant.
- **Host configuration.** It refuses a tenant carrying the stage marker, so no store's tenant can
  be a derived stage tenant.
- **Retained formats.** `ekr.publication-preparation/4` is a new retained format. Earlier binaries
  refuse it by format, and `/1`–`/3` are unchanged.
- **The gate.** This repository's binding cases hold every `store.yaml` declaration, and every id
  newtype, to a Rust carrier. The specification lands together with unit P's carriers.
- **The CLI component handles the four commands.** The owner chose it over `ekr-kernel`. ESS
  admits only the component owning a command's domain as its handler (`ESS-COMPONENT-004`), so the
  commands are in their own domain, `ekr.cli` (`ekr.cli.BeginStage`, `SealStage`, `PublishStage`,
  `AbandonStage`), owned by the component `ekr`, the `crates/ekr` binary. The stage and its events,
  refusals and view stay in `ekr.store`.

## Evidence

The cases design § 107.10 names, as run on wave 2026-10-07c's integration branch with a
PostgreSQL server required (`EKR_REQUIRE_POSTGRES=1`):

- unit C: `crates/ekr-kernel/tests/hosted_postgres.rs` (18 cases) and
  `crates/ekr-kernel/tests/adversary_w20261007c_c_capture.rs` (5);
- unit P: `crates/ekr-kernel/tests/stage.rs` (36 cases, SQLite and PostgreSQL) and
  `crates/ekr-kernel/tests/adversary_w20261007c_p.rs` (9);
- unit L: `crates/ekr/tests/stage_cli.rs` (9 cases, SQLite) and `crates/ekr/tests/postgres_cli.rs`
  (13, PostgreSQL), which hold the story's acceptance: a run whose gate fails is abandoned and
  `ekr head` is unchanged; a run that passes is published, `ekr head` is the stage's last revision
  re-derived for the store, and replay verifies; a publish at a moved head is refused by name and
  changes nothing; every verb works with the stage named only by `EKR_STAGE`; after abandon or
  publish the stage's tenant holds nothing.

The continuous-integration gate does not run the PostgreSQL cases: its workflow provides no
PostgreSQL server. The three adversary reviews are `.engineering/reviews/w20261007c-s-adversary.md`,
`w20261007c-c-adversary.md` and `w20261007c-p-adversary.md`.
