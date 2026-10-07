unit: S (task:stage-specified), commit f3fc6c4a72 on unit/stage-specified, tree ekr-w20261007c-s
verdict: NEEDS-CHANGE (3 blocking)
cases: executed n/a→n/a (no case added: the brief forbids repository edits), red 0 added; existing targets measured: domain_projection 8 run / 3 red, identity_serde 52 run / 4 red
origin: introduced 28 / pre-existing 0 / undecided 0
wrote-outside-worktree: none (scratch under .engineering/drafts/adversary-s-scratch/, ignored; cargo built into the tree's own target/)
needs-coordinator: B2 and B3 each need a decision (how a /4 slot is resolved; which revision-stream version the group expects)

## 1. git --no-pager diff --stat

(empty: no tracked file changed; `git status --short` empty)

## 2. Cases added

None. The brief forbids editing repository files, and this unit has no Rust carrier to drive.
Programs run instead: `ess specify validate` (0.36.0, 0.55.0, text and `--format json`),
`ess verify conform synthesize` (0.55.0, whole system and `--component ekr`), two ESS probes on a
copy of `systems/ekr` in scratch, and the two cargo targets below.

## 3. Runs (after reading, on the unit's tree)

- `ess specify validate --path systems/ekr`: valid on 0.36.0 and 0.55.0; 22 warnings on both base and
  head (no new warning). `--format json` on 0.55.0: `completeness.unanswered` 11 → 13; both new
  items are the stage commands' undeclared not-found order (cli.yaml:32-34 acknowledges them).
- 0.36.0 re-synthesis of the three committed suites: byte-identical (`cmp` silent).
- `cargo test --locked -p ekr-store --test domain_projection`: EXIT=101, 5 passed, 3 failed
  (`every_type_and_entity_the_domain_declares_names_a_rust_carrier`,
  `every_event_the_crate_writes_is_declared_by_the_domain`,
  `every_event_the_crate_writes_carries_the_fields_the_domain_declares`), as § 107.11 says. The
  implementor's log `.engineering/drafts/tests-2.log` never ran this target: cargo stopped at
  `adversary_p1_14_store_bindings`.
- `cargo test --locked -p ekr-core --test identity_serde`: EXIT=101, 48 passed, 4 failed (see N6).

## 4. Findings

Severity: blocking / fix-in-unit-P-or-L (C where named) / note. All origin `introduced`.

| # | severity | location | what is wrong | smallest correction |
|---|---|---|---|---|
| B1 | blocking | cli.yaml:13-15, cli.yaml:131-133; design.md:5567-5568, 5574, 5612-5614, 5688-5689; story:55-56 | `ekr stage publish` runs SealStage then PublishStage, and SealStage answers `wrong-state` for a Published stage (the synthesized `ekr.store.Stage/state/Published/refuses/ekr.cli.SealStage` confirms). So retrying `ekr stage publish` after the append landed is refused `stage-already-published` and never reaches `retained-publication`. The story's "retried after an uncertain outcome it adopts its own earlier publication" and § 107.8's "publish … forgets the tenant" can't be reached from the CLI. After a crash between the append and the forgetting, nothing can empty the tenant. | State in cli.yaml:13-15 and § 107.4 that `ekr stage publish` reads the record and runs SealStage only on a Begun stage; on Sealing or Published it runs PublishStage alone. Add unit L case `a_publish_retried_from_the_cli_after_its_append_returns_the_original_result_and_empties_the_tenant`. |
| B2 | blocking | design.md:5607-5610, 5630-5632, 5640-5642, 5688; cli.yaml:79-82, 143-148, 237-246; crates/ekr-kernel/src/migrate.rs:208-224; crates/ekr-store/src/inventory.rs:202-203 | Nothing says how a `/4` publication preparation is resolved. (a) A definitive conflict after election (§ 107.8 row 6) leaves the decision elected and never published. Abandoning that Sealing stage (abandoned-sealed's own summary invites it) does the same. BeginStage then refuses `UnresolvedPreparation` ("the store holds a decision elected and never published", cli.yaml:80; migrate.rs checks `inventory.prepared` against published event ids). The only resolver is publish, which refuses, so the store can never begin a stage again. (b) inventory reads `record.decision.event.event_id`, and a `/4` decision holds a list. So after the first publication, every begin and `ekr migrate` refuses `inventory-preparation-unreadable` unless inventory learns `/4`, which § 107.5 never asks. | In § 107.5, say (1) inventory reads `/4`; (2) a `/4` slot counts as resolved when its stage record holds StagePublished, or Abandoned, or its append definitively conflicted; (3) abandon of a Sealing stage resolves that slot first. Add unit P case `a_store_begins_a_stage_after_a_publication_refused_at_its_append` (and one after an abandon of an elected Sealing stage). |
| B3 | blocking | cli.yaml:138-141; design.md:5591-5592, 5622, 5631-5632; store.yaml:603-617; crates/ekr-store/src/eventlog.rs:28-29, 1859-1860, 1896-1899 | "Head" (last committed revision number) and "revision-stream version" are treated as the same thing. Every proposal, validation, rejection and stale record goes on the one `ekr.revision/canonical` stream without moving `ekr head`. cli.yaml conditions the group on "the revision stream at the expected head", § 107.4 on "the base's version", § 107.5 on "the expected revision-stream version", and none says where that version comes from. With another writer's `ekr propose` in the store during the run, one reading refuses `StageHeadMoved` with expected == base == current, contradicting its own summary, and kills the stage. The other appends the suffix after foreign occurrences. Separately, § 107.5 answers every definitive conflict `StageHeadMoved` "and the stage stays Sealing". That includes a record-stream conflict from a concurrent abandon (the stage is Abandoned) and an object-stream conflict. | Choose one rule and write it in cli.yaml, § 107.4 and § 107.5. Either (a) the expectation is the stream version read at capture, with head == base, or (b) any occurrence since begin refuses with a distinct refusal carrying stream versions. Map a record-stream conflict to `StageStateConflict` with the record's state. Add unit P case `a_proposal_made_in_the_store_during_the_run_…` asserting the chosen outcome. |
| F1 | fix-in-unit-P | design.md:5553-5555 vs 5714-5715; cli.yaml:181, 236; story:70 | A joined write that lands in an already-forgotten tenant leaves bytes behind. Object puts use `Expected::NoStream` (eventlog.rs:1749-1751) and forget_tenant leaves no tombstone (eventlog-sqlite lib.rs:2134-2150). § 107.3 says "the retry of publish or abandon" removes them, but after a successful publish or abandon nobody retries. So "the stage's tenant holds nothing" can fail. | A joined write whose re-read finds Published or Abandoned forgets the tenant named by StageBegun before refusing, from the same rule-2 source. Add a case with a hook between the pre-check and the write. |
| F2 | fix-in-unit-P-or-L | design.md:5551-5552; store.yaml:588-601, 491-503 | A write refused after the seal may still be in the publication, and the spec says to "read it from the published range". That range is the first and last revision id. Proposals and validations are not revisions, and `StageStateConflict` carries only stage_id and state, so the caller can't locate its write. A multi-commit verb (`apply-extraction`) can be partly published while it reports refusal. | The post-write refusal carries the event ids the write landed, under a distinct name (for example `stage-write-landed-after-seal`). The docs then say: once Published, read the store for those ids. |
| F3 | fix-in-unit-P | design.md:5586-5588 vs store.yaml:553-556 | "The objects the stage stored after its copy go with the suffix" excludes only the migration report and the receipt. That brings in the stage-lineage record objects the publication derives again: Canonical (never withdrawn), unnamed in the store, and naming the stage's `/4` seed envelope, which the store doesn't hold. This contradicts "nothing in the store's lineage names a record of the stage's". | Exclude every stage record the publication derives again. |
| F4 | fix-in-unit-C | store.yaml:220-232; design.md:5475-5481, 5443-5444; crates/ekr/src/host.rs:46; eventlog-core MAX_FIELD_LEN=512 | The spec says the derived stage tenant is valid "for every store tenant a host configuration admits". The host configuration admits any string the provider accepts (up to 512 bytes). Store tenant + marker + a 36-character id goes past 512 for long tenants. Refusing a tenant that carries the marker is also a new refusal for an existing store, against "no refusal a store gives today". | Derive the name as marker + a fixed-length digest of the store tenant + stage id. State the marker refusal as an exception in the § 107 header. |
| F5 | fix-in-unit-C | design.md:5734-5739, 5731-5732; crates/ekr-kernel/tests/hosted_postgres.rs:17-27 | `a_stage_tenant_is_never_a_store_tenant` "runs on every provider" but is placed in hosted_postgres.rs. Every case there returns early without `EKR_TEST_POSTGRES_CONFIG`, and no workflow under .github/ provides PostgreSQL. In the gate it passes without running. § 107.10's "each runs on SQLite and on PostgreSQL" is also false for C's PostgreSQL-only cases and the File-only case. | Move it to `crates/ekr-kernel/tests/stage.rs`. Reword § 107.10:5731-5732. |
| F6 | fix-in-unit-P | design.md:5747-5748 | `a_write_reported_successful_is_in_the_publication` and `a_write_joined_to_a_sealed_stage_is_refused_by_name`, written in sequence, both pass when the post-write re-read is missing. Nothing then forces the one mechanism behind "never lost". | Name the interleaving: a hook between the write's pre-check and its append, with the seal in between. |
| F7 | fix-in-unit-P | design.md:5761, 5660-5663; store.yaml:550-551 vs cli.yaml:27-30 | `a_handle_joined_to_a_published_or_abandoned_stage_refuses` passes with a check at open time only. Rule 2 says a joined handle checks "before every read and write", while cli.yaml says "first". | The case opens the handle while the stage is Begun, publishes, then reads through it. Make cli.yaml:27-30 say "before every read and write". |
| F8 | fix-in-unit-P | design.md:5758, 5651-5653 | `publish_and_abandon_of_one_stage_cannot_both_succeed` written in sequence passes without the record-version condition. | Abandon reads Sealing, publish appends, then abandon's conditional append is refused. Assert the refusal name too (see B3). |
| F9 | fix-in-unit-P | store.yaml:562-564; design.md:5677, 5759 | At forget time the source refuses only "the store's own" tenant. A StageBegun naming another store's tenant (PostgreSQL holds many stores at one location) or an unmarked name is forgotten. `the_store_tenant_is_never_forgotten` passes with a correct derivation and no check at forget time. | Refuse unless the recorded name carries the stage marker and equals the derivation from this store's tenant and the stage id. The case forges a StageBegun naming the store's tenant and one naming another store's tenant. |
| F10 | fix-in-unit-P | design.md:5760 | `a_reader_of_the_store_never_sees_part_of_a_suffix` passes against one append per occurrence unless the group is broken partway through. | Inject a fault inside the group and assert the store shows none of the suffix. |
| F11 | fix-in-unit-P | design.md:5746, 5749, 5688 | No case covers § 107.8 row 6's other branch (elected, then a definitive conflict), which is where B2 lives. | Add `a_publish_elected_then_refused_at_its_append_leaves_the_store_able_to_stage`. |
| F12 | fix-in-unit-L | store.yaml:592-594; cli.yaml:208-210; design.md:5614-5615 | `StageStateConflict` for a Begun stage on publish has no refusal name. The table names only Sealing, Published and Abandoned. | Add `stage-not-sealed` for Begun. |
| F13 | fix-in-unit-L | design.md:5539-5541, 5557-5559; story:35, 69 | The consumer exports `EKR_STAGE` for the run, so `ekr stage publish $EKR_STAGE` normally runs joined, and what stage verbs do when joined is left open. `mint` is in the story's verb list but not in § 107.3's. | Stage verbs always open the store's tenant and ignore `EKR_STAGE` (a conflicting `--stage` is a usage error). Say that `mint` ignores the stage. Include `mint` in `every_store_verb_joins_the_stage_named_by_ekr_stage`. |
| F14 | fix-in-unit-L | store.yaml:666-686; cli.yaml:20-23; design.md:5728-5729 | `ess verify conform synthesize --component ekr` (0.55.0) selects 4 scenarios. Every lifecycle scenario is "outside, other_component, needs ekr.store.Stages", so no component suite can ever hold the stage. Probe: moving the view into cli.yaml as `ekr.cli.Stages` validates on 0.36.0 and 0.55.0 and selects 33. | Declare the Stages view in cli.yaml. |
| N1 | note | cli.yaml:115-130, 188-203 | The ESS model allows `head-moved`, `stage-incomplete`, `unresolved-preparation`, `suffix-refused` and `already-published` on an Abandoned stage: 0.55.0 synthesis arranges each from Abandoned. `when_subject_state` beside `external` is refused (ESS-COMMAND-004, both versions, probed). `already-published` is reachable only by injection, because `retained-publication` (a state branch) is answered first. So the prose "stays Sealing" is undeclared. | Add a comment like cli.yaml:32-34: state refusals come first; undeclared and unchecked at ess/14. |
| N2 | note | cli.yaml:32-33 vs 115/119, 192/204 | The comment says `stage-not-found` comes before every refusal, but it is declared after `head-moved`, and ESS takes external branches in declaration order. | Declare `stage-not-found` as the first external outcome. |
| N3 | note | cli.yaml:247-249; design.md:5756 | `retained-abandonment` replays `abandoned` only. A stage abandoned from Sealing (`abandoned-sealed`) has no matching original in the model. | Run `an_abandon_interrupted_after_its_record_is_finished_by_its_retry` from Begun and from Sealing. |
| N4 | note | kernel.yaml:316-330 | The comment cites migrate_store.rs for "every replay of a /4 store without a receipt refuses migrate-incomplete"; migrate_store.rs never interrupts a copy. The case that executes it is `crates/ekr/tests/hosted_http.rs:498`, on File. "Legacy markers are read for /2 and /3 seeds only" has no case (no test names `ekr.migration-started/1`). COORDINATOR § 1. | Cite hosted_http.rs:498. Mark the legacy-marker sentence `unexecuted`. |
| N5 | note | store.yaml:305-330, 544-559, 649-651; design.md:5581-5582 | The stage comments in store.yaml state behaviour with no `unexecuted` marker (§ 107 has one in its header; store.yaml has none). store.yaml:649-651 states a fact about eventlog-file's code with no marker (§ 107.2 has one). design.md:5581-5582 names migrate.rs, the source, rather than its case (migrate_store.rs:181-188). | Add `unexecuted` / name the case. |
| N6 | note | design.md:5787-5788 | § 107.11 names one identity_serde case; 4 fail: also `an_unknown_domain_filename_with_a_missing_identity_fails_the_actual_scan`, `adversary_input08::semantic_yaml_spelling_does_not_hide_a_new_identity` and `adversary_input08::renamed_domain_files_preserve_the_identity_inventory` (measured). | List all four. |
| N7 | note | crates/ekr/tests/adversary_docs_contract.rs:229-231, 247-262 | Its premise is that the binary "implements no single domain", but component `ekr` now owns `ekr.cli`. It still passes, and it would refuse unit L naming cli.yaml in main.rs. | Tell unit L. Amend the premise when L touches it. |
| N8 | note | design.md:4197-4199 vs 5646-5649 | § 94.3 says "Do not delete data to clear an uncertain result". Abandon forgets a stage tenant even when it holds an unresolved joined-write preparation, and § 107 doesn't amend § 94.3. | Add one sentence in § 107.6 exempting stage tenants. |
| N9 | note | ADR 0013:98-128 | Missing consequence: eventlog-postgres `forget_tenant` takes the provider-wide exclusive publication gate (eventlog-postgres lib.rs:788, 1882-1897) while deleting a full copy, so every tenant's writes at that location wait. Unmeasured. | Add a Consequences bullet. |
| N10 | note | design.md:5719-5726 | Invariant 1 (story_contract.rs:565): only ekr-kernel may declare ekr-store. domain_projection.rs:185-197 needs the four events written from ekr-store eventlog.rs or preparation.rs. So "`ekr` handles the four" is ESS ownership only; the code goes in ekr-kernel and ekr-store. | Say so in § 107.10 so unit L doesn't add an ekr-store dependency. |
| N11 | note | commit message; design.md:5778-5791 | The integration branch is red from S until P, with 8 failing cases (3 + 1 + 4). Unit C comes before P in the story's order, so C's gate starts red for reasons that aren't C's. | Record it in C's brief, or merge P's carriers before C. |

## 5. Attacked and could not break

- States and transitions: store.yaml:385-398, § 107.1:5492 and the synthesized transitions agree.
- Event emitted per command: cli.yaml, components.yaml:95-114 and § 107.10 agree (the precedent of ekr-kernel publishing ekr.store events holds).
- Double append on retry: the record-version condition plus the `(Publish, stage id)` slot prevent it, given B1's CLI fix.
- Publish at a moved head: Seal refuses before writing; publish refuses after the seal; neither appends.
- Stage revisions counted as committed: § 107.1:5504-5511, rule 2 and the ADR are consistent, apart from F3.
- `unknown_instance:` needs ess/15 and 0.36.0 does not know it (probed), so cli.yaml:33's claim holds.
- Cited cases exist and assert what is claimed: postgres_runtime_reopens_seed_and_committed_history, interrupted_postgres_copy_is_unreadable_after_reopen, a_claimed_copy_can_be_captured_and_copied_again, migrate_refuses_an_elected_unpublished_decision_…, migrate_cli.rs sqlite runs. `migrate-source-not-supported` is truly unexecuted.
- § 107.7's counts hold: eventlog.rs 1 (delete_blob), eventlog_reads.rs 5 under `#[cfg(test)]` (eventlog.rs:50-52).
- TenantId rules: eventlog-core validate_field matches the quoted rule (512 bytes, ASCII graphic or space).
- No event-id uniqueness across tenants in eventlog-sqlite or eventlog-postgres, so reusing identities in the same database is admitted.

## 6. Paths written outside the worktree

none. Inside the tree, all ignored: `.engineering/drafts/adversary-s.md`,
`.engineering/drafts/adversary-s-scratch/` (base spec copy, ESS outputs, probe copy, two cargo logs);
cargo wrote into the tree's own `target/`.

## 7. Findings block

```findings
- file: systems/ekr/domains/cli.yaml
  line: 13
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ekr stage publish runs SealStage first, which refuses a Published stage, so a CLI retry never adopts its publication nor empties the tenant
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5630
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a /4 preparation is never resolved after a definitive conflict or an abandon, and inventory cannot read /4, so later begins refuse permanently
- file: systems/ekr/domains/cli.yaml
  line: 138
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: head and revision-stream version are conflated, so a foreign proposal or any definitive conflict is answered StageHeadMoved with an unchanged head
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5553
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a late joined write into a forgotten tenant leaves bytes that only a retry removes, and no retry follows a successful publish or abandon
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5551
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a refused write may be published, and the refusal carries nothing the caller can match against the published revision range
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5586
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: carrying every object the stage stored brings its re-derived stage-lineage records into the store as unnamed Canonical objects
- file: systems/ekr/domains/store.yaml
  line: 226
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: store tenant plus marker plus id exceeds Eventlog's 512-byte limit for long admitted tenants, and the marker refusal is a new refusal for existing stores
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5738
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the every-provider tenant case is placed in hosted_postgres.rs, whose cases return early without PostgreSQL, which CI never provides
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5747
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the two joined-write cases pass sequentially without the post-write re-read that alone guarantees no reported write is lost
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5761
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the joined-handle case passes with an open-time check, and cli.yaml says first where rule 2 says before every read and write
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5758
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: publish-versus-abandon written sequentially passes without the record-version condition
- file: systems/ekr/domains/store.yaml
  line: 562
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the forget-time check refuses only this store's tenant, not another store's or an unmarked name, and its case cannot fail on that
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5760
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the reader-atomicity case passes against per-occurrence appends unless a fault is injected inside the group
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5688
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: no case covers an elected publication refused at its append, the path where the unresolved-preparation lockout lives
- file: systems/ekr/domains/store.yaml
  line: 592
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: StageStateConflict for a Begun stage on publish has no refusal name
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5557
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: stage verbs under an exported EKR_STAGE are left open though the consumer flow always has it set, and mint is missing from the joined verbs
- file: systems/ekr/domains/store.yaml
  line: 666
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the Stages view owned by ekr-store puts every lifecycle scenario outside the ekr component suite; declared in ekr.cli it selects 33
- file: systems/ekr/domains/cli.yaml
  line: 115
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: externally decided refusals carry no state guard, so the model admits head-moved and suffix-refused on an Abandoned stage
- file: systems/ekr/domains/cli.yaml
  line: 32
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: stage-not-found is said to come first but is declared after head-moved
- file: systems/ekr/domains/cli.yaml
  line: 247
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: retained-abandonment replays only the Begun-origin abandonment
- file: systems/ekr/domains/kernel.yaml
  line: 326
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the receipt claim cites a case that never interrupts a copy, and the legacy-marker claim has no case and no unexecuted marker
- file: systems/ekr/domains/store.yaml
  line: 544
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: store.yaml's stage comments and its File provider claim carry no unexecuted marker
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5787
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: four identity_serde cases fail, and section 107.11 names one
- file: crates/ekr/tests/adversary_docs_contract.rs
  line: 229
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the test's premise that the binary owns no domain is now false
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5646
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: abandon deletes an unresolved preparation, which section 94.3 forbids, with no amendment
- file: .engineering/planning/architecture-decision-record/0013-a-run-is-staged-and-published-whole.md
  line: 98
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the consequences omit that PostgreSQL forget_tenant holds the provider-wide exclusive publication gate
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5719
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the ekr component handles the commands in ESS only; invariant 1 puts their code in ekr-kernel and ekr-store
- file: docs/epistemic-knowledge-runtime-design.md
  line: 5778
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the integration branch stays red from S until P, so unit C's gate starts red
```

## 8. Where the blocking rows were fixed

- B1: `systems/ekr/domains/cli.yaml` (domain comment; SealStage and PublishStage comments) and design § 107.4 and § 107.8: `ekr stage publish` reads the record and runs SealStage only on a Begun stage; unit L case `a_publish_retried_from_the_cli_after_its_append_returns_the_original_result_and_empties_the_tenant` (§ 107.10).
- B2: design § 107.5 ("When a `/4` slot is resolved": published, abandoned, definitively conflicted; "The inventory reads `/4`") and § 107.6; cli.yaml BeginStage `unresolved-preparation` and the AbandonStage comment; unit P cases `a_store_begins_a_stage_after_a_publication_refused_at_its_append`, `a_store_begins_a_stage_after_an_elected_sealing_stage_is_abandoned`, `a_store_with_a_published_stage_begins_another_and_migrates`.
- B3: option (a). The head is the last committed revision (`systems/ekr/domains/store.yaml`, comment on `ekr.store.StageHeadMoved`; design § 107.4). The group expects the revision-stream version read at the capture, and the suffix follows any non-committing foreign occurrences, which the text now says (cli.yaml PublishStage comment; § 107.4). Each append conflict has its own refusal and stage state: record `StageStateConflict` (`abandoned-meanwhile`, Abandoned), head `StageHeadMoved` (Sealing), stream `ekr.store.StageStreamMoved` (new, Sealing, retry publishes), object `ekr.store.StageObjectMoved` (new, Sealing, retry publishes) (store.yaml errors; cli.yaml PublishStage outcomes; § 107.4 table; § 107.5; § 107.8). Unit P cases `a_proposal_made_in_the_store_during_the_run_does_not_refuse_the_publication_and_precedes_the_suffix`, `a_proposal_landing_between_capture_and_append_is_refused_stage_stream_moved_and_the_retry_publishes`, `an_object_stored_in_the_store_after_the_capture_is_refused_stage_object_moved_and_the_retry_publishes`, `an_abandonment_landing_between_capture_and_append_is_answered_stage_already_abandoned`.
- F4 and F5, specification text only: store.yaml `ekr.store.StageId` comment and design § 107.1 (marker, fixed-length digest of the store's tenant, stage id; within `MAX_FIELD_LEN` 512); the § 107 header names the marker refusal as its one exception; § 107.10 places `a_stage_tenant_is_never_a_store_tenant` in `crates/ekr-kernel/tests/stage.rs` and states per case where it runs.
