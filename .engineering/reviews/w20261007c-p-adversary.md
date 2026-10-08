unit: P (task:stage-suffix-publication), commit 811cddb2af on unit/stage-suffix-publication, tree ekr-w20261007c-p plus one untracked test file
verdict: NEEDS-CHANGE (1 blocking)
cases: executed 30→41 (stage 30, from the implementor's stage-pg-2.log; after: stage 30 + adversary_w20261007c_p 11), red 8
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: none (cargo built into the tree's own target/; scratch and logs under .engineering/drafts/adversary-p-scratch/, ignored)
needs-coordinator: probe `two_publishes_of_one_stage_append_it_once_and_answer_alike` failed 2 of 5 full-target runs before diagnostics were added: a revision-stream count of 4 read from the provider feed where 7 was expected, while both publishes answered Published. Provider not recorded, and 0 of 11 runs failed after the diagnostics. Cause not established

## 1. git --no-pager diff --stat

(empty: no tracked file changed.) `git status --short`: `?? crates/ekr-kernel/tests/adversary_w20261007c_p.rs`, a test file. No implementation file was touched.

## 2. Cases added

File `crates/ekr-kernel/tests/adversary_w20261007c_p.rs`: 11 cases. Each runs on SQLite and on PostgreSQL (`each`). With `EKR_ADVERSARY_POSTGRES_FIRST=1`, PostgreSQL runs first. Every case was run alone, first on SQLite-first and then on PostgreSQL-first, before the suite. Logs: `adversary-p-scratch/red-*.log` and `pg-red-*.log`.

| case | asserts | now |
|---|---|---|
| `an_abandonment_that_read_begun_abandons_a_stage_sealed_meanwhile` (:356) | abandon reads Begun, a seal lands, and abandon then abandons the Sealing stage (§ 107.5) | red, both providers |
| `a_seal_that_read_begun_is_refused_stage_already_abandoned_when_an_abandonment_lands` (:388) | seal reads Begun, an abandonment lands, and seal answers `StageStateConflict{Abandoned}` (§ 107.4) | red, both |
| `an_abandonment_that_read_begun_is_refused_stage_already_published_when_a_publication_lands` (:420) | abandon reads Begun, seal+publish lands, and abandon answers `StageStateConflict{Published}` (§ 107.6) | red, both |
| `a_begin_refused_for_an_unresolved_stage_publication_names_the_stage` (:455) | begin's `unresolved-preparation` names the Sealing stage (§ 107.5) | red, both |
| `a_stage_published_before_its_forgetting_then_abandoned_leaves_its_tenant_empty` (:485) | after a crash between append and forget and then an abandon, the stage tenant holds 0 events (story acceptance) | red, both |
| `what_lands_after_a_seal_is_never_an_occurrence` (:521) | § 107.3's sentence, two threads joined by channels: an election lands between the seal's capture and its record, and the publication lands after | red, both |
| `a_validation_another_writer_made_in_the_store_during_the_run_does_not_refuse_the_publication` (:604) | § 107.4's sentence: another writer's validation of a pending store proposal that the run validated and committed | red, both |
| `a_validation_landing_between_capture_and_append_is_answered_as_a_capture_answers_it` (:627) | the same validation at PublishElected; the retry publishes or is refused `stage-suffix-refused` | red, both |
| `publications_in_sequence_from_a_seed_replay_and_migrate` | probe | green |
| `another_store_cannot_reach_a_stage_and_a_stage_has_no_stage` | probe | green |
| `two_publishes_of_one_stage_append_it_once_and_answer_alike` | probe | green (see needs-coordinator) |

Red output, captured when each case first ran alone (SQLite part; PostgreSQL gives identical text, see `pg-red-*.log`):

```
an abandonment of a stage sealed meanwhile is admitted (design § 107.5), and the stage is now Sealing: the store is unavailable: idempotency key ekr.stage.01a11913-90be-7362-9535-64788e72ac31.1 was already used for a different request
---
assertion `left == right` failed: seal refuses an Abandoned stage by name (design § 107.4)
  left: Backend("idempotency key ekr.stage.01a11913-b73f-7236-baaf-05ef0cd4e0d0.1 was already used for a different request")
 right: StageStateConflict { stage_id: StageId(2165696412296660667177303398663119056), state: Abandoned }
---
assertion `left == right` failed: at most one succeeds, and the other is refused by name (design § 107.6)
  left: Backend("idempotency key ekr.stage.01a11913-b99f-731f-8c4c-c73bffbb0931.1 was already used for a different request")
 right: StageStateConflict { stage_id: StageId(2165696413031691860252064299007805745), state: Published }
---
the refusal names the Sealing stage whose slot is unresolved, 01a11913-bc20-76eb-b6c7-54362da58e3f, so the caller knows which stage to publish or abandon (design § 107.5): unresolved-preparation: occurrence 01a11913-bc6f-77e9-9137-30cd935a2ba3 was elected and never published; resolve it with the command that elected it
---
assertion `left == right` failed: after abandon or publish, the stage's tenant holds nothing (story acceptance)
  left: 23
 right: 0
---
an occurrence landed after the seal ([EventId(2165696423797168409890334124524386017)]); the publication then published it (1 occurrences, transaction Some(Proposed)) while its writer was refused stage-write-landed
---
a validation another writer made does not refuse the publication (design § 107.4): stage-suffix-refused: stage 01a11913-df33-762b-85e9-698dca39702b: retained-transaction-state-conflict: a stored document could not be read: retained-transaction-state-conflict
---
expected stage-suffix-refused: a stored document could not be read: retained-transaction-state-conflict
```

## 3. Suite run (after the cases existed)

`EKR_TEST_POSTGRES_CONFIG=…/unit-p/app.json EKR_TEST_POSTGRES_OWNER=…/unit-p/owner.json EKR_REQUIRE_POSTGRES=1 cargo test --locked --no-fail-fast -p ekr-kernel --test stage --test adversary_w20261007c_p -- --test-threads=1`. EXIT=101.

```
Running tests/adversary_w20261007c_p.rs
test result: FAILED. 2 passed; 9 failed; … finished in 15.41s   (the 9th was the intermittent probe)
Running tests/stage.rs
test result: ok. 30 passed; 0 failed; … finished in 68.91s
```

The latest full-target run (`feed-6.log`) gives `test result: FAILED. 3 passed; 8 failed; … finished in 12.73s`.

## 4. Findings

| # | severity | location | failure | smallest fix | verdict / origin | what reaches it |
|---|---|---|---|---|---|---|
| B1 | blocking | crates/ekr-store/src/stage_record.rs:336 | The stage record's idempotency key is `ekr.stage.{stage}.{version}`, identical for StageSealed and StageAbandoned at one version. When seal and abandon race from Begun, the second command meets the first's group receipt, and Eventlog answers `IdempotencyMismatch` before checking the stream. That becomes `StoreError::Backend`, displayed "the store is unavailable: …". The resulting state is right. The refusal is not. Abandon of a stage sealed meanwhile is refused, against § 107.5 "always admitted", seal of an abandoned stage is not `stage-already-abandoned`, and abandon racing a whole publish is not `stage-already-published`. The F8 case seals first, so it never reaches this | Put the event name in the key (`ekr.stage.{stage}.{version}.{name}`), or map `IdempotencyMismatch` on a record append to `Conflict` | NEEDS-CHANGE / introduced | `ekr stage abandon` run while `ekr stage publish` is sealing a Begun stage (§ 107.6's own race) |
| F1 | fix-in-unit-L | crates/ekr-store/src/lib.rs:256; crates/ekr-kernel/src/stage.rs:214 | `UnresolvedPreparation(EventId)` names an occurrence that is in no store, not the stage. § 107.5 says "The refusal names that stage" so it can be published or abandoned. `migrate-unresolved-preparation` has the same gap | Carry the stage id for a `/4` slot (inventory knows it), and have the message say publish or abandon that stage | NEEDS-CHANGE / introduced | `ekr stage begin` or `ekr migrate` after a publish elected and never appended |
| F2 | fix-in-unit-L | crates/ekr-kernel/src/stage.rs:320 | The successor branch returns the store authority's verification refusal raw, as `a stored document could not be read: retained-transaction-state-conflict`. A capture of the same store state gives `stage-suffix-refused` (5a). So design.md:5828 "stays Sealing until a retried publish publishes" is false here, and PublishStage's outcomes do not name this refusal | Map a refusal of the successor's election to `StageSuffixRefused`, or rerun `stage_suffix` for the successor | NEEDS-CHANGE / introduced | another writer validates a store transaction the run validated, between the publication's capture and its append |
| N1 | note | docs/epistemic-knowledge-runtime-design.md:5584 | "what lands after a seal is an election, or a checkpoint pointer, and never an occurrence" is false. The seal's unresolved check runs at its capture, before `StageSealed`. An election in that window, whose publication lands after the seal, is refused `stage-write-landed` with a non-empty `event_ids`, and the publication then publishes it. The cited case `a_write_joined_to_a_sealed_stage_is_refused_by_name` builds only the election-after-seal interleaving, so it cannot fail on this sentence (COORDINATOR § 1/§ 2) | Strike the sentence; say that a refused joined write's occurrences may be published and are found by id (StageWriteLanded already says this) | CONFIRMED / introduced | a joined write concurrent with `ekr stage publish` |
| N2 | note | docs/epistemic-knowledge-runtime-design.md:5668 | "Occurrences … that commit nothing (a proposal, a validation, …) do not refuse the publication" over-claims. Another writer's validation of a pending store transaction that the run also validated and committed refuses the publication `stage-suffix-refused` (`retained-transaction-state-conflict`). The refusal is correct; the sentence is not | Add "unless it decides a transaction the run also decides" | CONFIRMED / introduced | a validator agent working the store while a run commits the same pending proposal |
| N3 | note | crates/ekr-kernel/src/stage.rs:552 | A Published stage whose forgetting was interrupted keeps its tenant (23 events) if the caller recovers with abandon. Abandon is refused `stage-already-published` (as specified) and forgets nothing. Only a publish with the base as expected head empties the tenant. Story acceptance: "After abandon or publish, the stage's tenant holds nothing" | Abandon of a Published stage still refuses, after `forget_stage_tenant` | CONFIRMED / introduced | a crash after the group lands, with the run's error path calling `ekr stage abandon` |
| N4 | note | systems/ekr/domains/store.yaml:292 | "an earlier binary refuses `/4` by format": the base reader (`git show 811cddb2af~1:crates/ekr-store/src/inventory.rs`, :303-317) never reads `format`. It refuses `inventory-preparation-unreadable` because `decision.event.event_id` is absent. It still refuses; only the stated reason is wrong. No case | Say "refuses it as unreadable" | CONFIRMED / introduced | an older `ekr` running begin or migrate on a store after any stage publication |

## 5. Attacked and could not break

- Two publishes of one stage (one at PublishElected runs the other whole): the group lands once (capture count 7), and both return the same Published result. The suite never ran the deduplicating resume before this probe.
- Another store at the same location: seal, publish, seal-and-publish, abandon and join of its stage id answer `stage-not-found`, and the stage tenant and the neighbour store are unchanged.
- A stage of a stage: begin, join, seal, publish, abandon and stages on a joined runtime answer `stage-command-on-stage`, and openers refuse marker tenants (unit C's case).
- Sequential publications from a seed-only store: a pending store evidence proposal, committed by the run, has its payload raised to Provenance. An empty run publishes 0 occurrences and leaves the stream unchanged. A third run follows. Full replay roots equal, snapshot holds, content is the store's, Stages lists 2/0/1 revisions, and SQLite migrate roots equal.
- `forget_tenant` for the store's own tenant: the one product call (stage_record.rs:300) is reached from `forget_stage_tenant` and `landed`. Both check the tenant against the derivation from the store's tenant, and a store tenant cannot carry the marker. No path found.
- Forged or replayed records: reachable only through a raw provider. `StageLog::record_stage_*` take a caller-built `StageRecord` (a fabricated Begun at the current version could append Abandoned after Published and make `stages()` unreadable for the whole store), but only ekr-kernel holds ekr-store (invariant 1). INFEASIBLE, not raised.
- A foreign commit during the run, or between seal and append: `stage-head-moved`, nothing appended (existing cases).
- `/1`–`/3` preparations: inventory returns the same single id. The `/1`–`/3` `read_preparation` slots never collide with a `/4` slot stream.

## 6. Paths written outside the worktree

none. Inside the tree, ignored: `.engineering/drafts/adversary-p.md` and `.engineering/drafts/adversary-p-scratch/` (build, red, pg-red, probe, suite, rerun and feed logs). cargo wrote only into the tree's own `target/`.

## 7. Findings block

```findings
- file: crates/ekr-store/src/stage_record.rs
  line: 336
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: seal and abandon share one idempotency key per record version, so racing from Begun answers an unnamed Backend idempotency mismatch instead of the conditional-append conflict
- file: crates/ekr-store/src/lib.rs
  line: 256
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: unresolved-preparation names an occurrence in no store rather than the Sealing stage that design 107.5 says the refusal names
- file: crates/ekr-kernel/src/stage.rs
  line: 320
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a successor attempt the store authority refuses is returned as a raw document error, where a capture of the same store answers stage-suffix-refused
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5584
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an election between the seal's capture and its record lets an occurrence land after the seal and be published while its writer is refused
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5668
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: another writer's validation of a transaction the run also validated refuses the publication, contrary to the sentence listing validations as harmless
- file: crates/ekr-kernel/src/stage.rs
  line: 552
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: abandon of a Published stage whose forgetting was interrupted refuses and leaves the stage tenant holding its events
- file: systems/ekr/domains/store.yaml
  line: 292
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an earlier binary refuses a /4 record as inventory-preparation-unreadable, not by format
```

## Outcome

- Blocking: each stage-record move has its own idempotency key, `ekr.stage.{stage}.{version}.{name}`;
  a seal/abandon race ends in its named refusal, and an abandonment of a stage sealed meanwhile is
  admitted.
- F1: begin, seal and `ekr migrate` name the stage of an unresolved `/4` slot.
- F2: a publication whose election the store refuses answers `stage-suffix-refused`.
- N3: abandon of a Published stage forgets what is left of its tenant before refusing.
- N1 and N2 were sentences of § 107, not code: § 107.3 now says an occurrence may land after the
  seal (its writer is refused `stage-write-landed` and the publication carries it), and § 107.4
  says another writer's validation of a transaction the run also decides refuses the publication.
  The two adversary cases that asserted the struck sentences were removed; `stage.rs` holds the
  corrected behaviour.
- N4: an older binary's inventory refuses a `/4` record as `inventory-preparation-unreadable`;
  `store.yaml` and § 107.5 say so.
- The two-publish probe counted the store from the PostgreSQL change feed, which withholds events
  committed after the oldest transaction still open in the database. It was not a double or
  partial publication; `two_publishes_of_one_stage_append_its_suffix_once_and_answer_alike` runs
  the second publish at a named hook inside the first, on both providers.
