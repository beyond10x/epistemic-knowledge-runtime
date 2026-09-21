# Predecessors — what v1 and v2 hold that the v3 design must not lose

**Status:** survey, 2026-09-21. Every count below was read from a command or a file on that date;
the source is named beside it. Where a claim is inferred rather than read, it says so.
**Purpose:** answer two questions before the first crate exists. (1) Which capabilities of
`company-brain` (v1) and `org-brain` (v2) does `docs/epistemic-knowledge-runtime-design.md` not cover,
or cover weaker? (2) Would their stored data and schemas conflict with the design if this runtime
imports or coexists with them?
**Result:** 15 capabilities to carry (§ 2), 6 v1 conflicts (§ 5), 13 v2 conflicts (§ 6), all
resolvable by one import policy (§ 9). Nothing found that invalidates the design; several things it
must additionally say.

Design references are `§ n` (section) or `design:L` (line) in `docs/epistemic-knowledge-runtime-design.md`.

---

## 1. The lineage

| | v1 | v2 instance | v2 engine |
|---|---|---|---|
| checkout | the `company-brain` tree | the `org-brain` instance tree | the `org-brain` engine tree (`beyond10x/org-brain`, 0.5.0) |
| language | Python (`pyproject.toml`), 130+ scripts in `bin/` | Rust, 9,969 LOC in `src/` | Rust, ≈80k LOC across 7 crates (`brain`, `brain-core`, `brain-ledger`, `brain-rules`, `brain-views`, `brain-check`, `brain-round`) |
| kernel | none; JSON Schema `knowledge/state/ontology.schema.json` + 13 entity YAMLs | entity-runtime 0.17.7 (`org-brain-successor/Cargo.toml:24-28`) | same |
| store | 537 curated `.md`, 2,296 raw `.jsonl`, 121 MB under `knowledge/` | `store/` in `entity.file-store/2`: 5,304 subject files, 3.6 GB; `raw/` 26 GB / 619,487 files | none (engine is ontology-agnostic) |
| ontology | 20 `$defs` in the JSON Schema | 36 `definitions/*.yaml` bound to 15 engine roles | `roles/*.yaml` |
| loop | `/refresh` round: scripts + one bounded LLM pass (`skills/workday/SKILL.md:20`) | acquire → normalise → pre-extract → extract → mark → gate → attention → render, as an AEP driver map | `drivers/sync-round.yaml`, `brain continuous run` |
| status | retired into v2 on 2026-09-03 (`org-brain/import-report.json`) | live; interpretation stopped 2026-09-07 (§ 8) | live |

The v2 instance's `Cargo.toml:1-10` states the split: "the engine `brain` is a separate crate and is
not vendored here … This repository is data and configuration." `atlas/ROADMAP.md:113` confirms
`org-brain-successor` is the only engine checkout and `beyond10x/org-brain` `main` is its history.

## 2. Capabilities to carry

Each row is a capability at least one predecessor has that the design does not specify or
specifies weaker. "Action" names the design amendment or roadmap phase that carries it.

| id | capability | v1 | v2 | design today | action |
|---|---|---|---|---|---|
| A1 | Attention page with numbered open questions a named human answers | `/workday answer "5c 2a"`, `knowledge/state/workday-view.html` | `views/attention.index.json` (`brain.attention-index/1`), `brain answer` | § 18 "human reviewers" (design:964-971); § 44 "escalate to human review" (design:1988) — no queue, no render, no answer protocol | design amendment "Operator surface"; P4 |
| A2 | Rendered output: briefs, digests, HTML view, Slack post-back, typed view spec | `bin/build-workday-view.py`, `bin/slack-post.py`, `knowledge/{briefings,digests}/` (53 + 75 files) | `views/*.j2`, `ui/views.json` + `ui/schemas/ui-views.schema.json`, `brain render` | none; § 47 query scopes (design:2058-2077) is the nearest | design amendment "Projections and views"; P4 |
| A3 | Action boundary: the system reads the world; outward writes need standing approval | `bin/task-queue.py` propose → approve → execute; `knowledge/drafts/` "never sent automatically" | `communication.send` requires an approval; `approvals/health-notifications.md`; sync profile denies `communication.publish` | § 31 frontier plans crawls only (design:1461-1505); no action model | new invariant 6.11; P4 |
| A4 | Obligations with an external clock; horizons past which a fact reads `?` | `Due:` / `Recurs:` lines, `bin/deadlines.py`, `bin/check-deadlines.py` | `verified.horizon_days`, reply-owed, "past it the fact reads `?`, never `✗`" (engine design § 7.6) | § 14 bitemporal ranges only (design:782-805) | design amendment "Obligations and horizons"; P4 |
| A5 | Bounded model protocol: immutable request bytes with digests, one disposition per signal including no-ops with a `basis`, receipts, three-visit bound | — | `brain.extraction-request/4`, `brain.extraction/2`, `receipt/4` (engine design, amendment 2026-09-05) | § 18 agents are abstract; § 19 transactions carry no session contract | design amendment "Interpretation session contract"; P3 |
| A6 | Credential redaction before model input; PII/secret gates; GDPR erasure register; privacy projection | `bin/gdpr-erase.py`, `check-pii.py`, `check-secrets.py`, `knowledge/compliance/` | credential policy 3 (engine design amendment), `src/dashboard/privacy.rs` | § 58–60 state the principle (design:2326-2391) | P2 requirement; erasure through eventlog redaction in P6 |
| A7 | Cost accounting: micro-USD, daily boundaries, reservations, pilot limit, actual-cost reconciliation | — | `brain-round`; `brain.toml:21-53` | § 32 `estimated_cost: f32` (design:1534) | P5 requirement with a stop condition |
| A8 | Coverage denominators and poll health: `checked_through`, `attempt | complete | partial | failed`; a successful poll proves only its window | partial (`knowledge/state/cursor.json`, `bin/check-channels.py`) | engine design § 8; `src/coverage_audit.rs`; `raw/.audit/coverage-latest.json` | § 55 checkpoints (design:2266-2287) | P2 requirement |
| A9 | Self-improvement ledgers: lessons, gates, recurrence | `knowledge/meta/{lessons,gates}.jsonl`, `bin/improve.py` | `lesson` (443 records), `gate` (35) | § 61 agent metrics (design:2418-2423) | P6; lessons are ordinary canonical nodes |
| A10 | Identity: source id as identity, typed person references with a name-loss gate, prefix-collision rule | `bin/entity_ids.py:5-25` documents the longest-prefix collision rule | `src/person_refs/` (1,532 LOC); "Identity beats fuzz … no string-matched resolver exists" (engine design § 7.4) | § 45 concept only (design:1996-2026) | P3 resolver: explicit merge transactions, no string matching |
| A11 | Scheduled and continuous operation with independent per-source collectors | `config/systemd/company-brain-sweep.{timer,service}` | `systemd/` (continuous, dashboard, round, coverage-audit, health-notify, connectors); collectors run while the model waits | § 33 `loop { }` (design:1599-1627) | P5: both modes; loop steps ordered by an AEP driver map |
| A12 | Binary attachments as evidence | 12 PDF, 18 PNG, 3 `.m4a` under `knowledge/` | — | `ObservationContent` has no blob variant (design:827-835) | add `Blob(ContentHash)`; P2 |
| A13 | Per-type lifecycles and named operations with transitions and emitted events | 13 entity YAMLs under `knowledge/state/entities/` | every `definitions/*.yaml`: `lifecycle.{initial,states}`, `operations.<name>.{arguments,transitions,set,emits}` | § 11 types and properties (design:591-613); § 19 generic operations (design:995-1010) | add `lifecycle` and `operations` to `NodeType`; ontology-constraint validator enforces; P1 |
| A14 | MCP read surface for agents with the doctrine that record text is untrusted evidence | — | `src/mcp.rs:66-83`: 8 tools, 418 LOC, no SDK, no async runtime | § 70 internal trait only (design:2770-2796) | P4 `ekr-mcp`, lifted |
| A15 | Human work backlog ranking (person, direction, next step, freshness) | partial (`bin/painpoints.py`, `bin/busfactor.py`) | `src/backlog.rs` (413 LOC) | § 32 ranks crawl work, not people's work (design:1520-1550) | P4 projection over obligations, distinct from the frontier |

## 3. Where the design is stronger than both predecessors

Not carried from them, because v3 already exceeds them: entity merge and split with lineage
(§ 45–46); contradiction as a `Disputed` state (§ 44) — v2 refuses conflicting claims at write time and
keeps no record of the loser; bitemporality (§ 14); retention, consolidation and GC (§ 35–42); schema
evolution from incubation evidence (§ 25–26).

## 4. Dropped

| item | where | why |
|---|---|---|
| story backlog and CHANGELOG for building v1 | `company-brain/docs/stories/` (≈200 files) | development process; this repository has its own AEP store |
| telephony and platform probes | `bin/acd-health.py`, `acd-queue-rows.py`, `ghost-calls.py`, `call-timeline.py`, `can-login.py` | instance concern, not runtime |
| adoption screen over a foreign system | `org-brain/src/screen.rs` (355 LOC) | organisation-specific projection |

## 5. v1 data and schema against the design

| conflict | v1 | design | import policy |
|---|---|---|---|
| "observation" means a mutation | `knowledge/state/observations.jsonl` lines are ops: `create` 11,393, `update` 245, `close` 185, `answer` 16, `mark` 7 (11,846 lines) | § 15: an observation is an immutable record of external bytes and "does not say the semantic claims are true" (design:808-845) | v1's real observation layer is `knowledge/raw/<channel>/<YYYY-MM>.jsonl` → v3 Observations. `observations.jsonl` → transient root `v1-workday` with a local schema; never canonical directly (else § 74.2 canonical contamination) |
| identity is a directory slug, which is also the human name | `knowledge/<category>/<slug>/…`; `index.json` keyed by path | § 6.4 "Human-readable names are not identities" (design:374-380); `NodeId(u128)` (design:549) | mapping table slug → `NodeId`; slug kept as an alias property |
| untyped values | `payload.title` is prose; `claim`, `resolver`, `validation` declared `type: json` in `knowledge/state/entities/agent-task.yaml` | § 11.3 "Values should not default to arbitrary JSON" (design:615-659) | stays transient until a mapping types it |
| no validation state; provenance is `manual` | `source: {kind: "manual", ref: "…"}`, payload hash `h` | § 17 `ValidationState` (design:898-930); § 6.5 "an agent said so is not sufficient" (design:381-386) | imported records carry `origin: inherited` and are never `Accepted` until a live source re-observes them (v2 engine design § 7.9) |
| point-in-time only | `ts` (write), `event_ts` (occurrence); no interval | § 14 valid time and transaction time as ranges (design:782-805) | `valid_from = event_ts`, `valid_to` open, flagged inferred |
| storage-class taxonomy | `knowledge/meta/file-classes.json`: `fact | stream | view | generated | asset` | § 37 `Canonical | Provenance | Incubating | Cache | Ephemeral` (design:1740-1748) | fact → canonical candidate; stream → Provenance; view, generated → Cache; asset → Blob evidence |

Consistent rather than conflicting: 2,296 raw `.jsonl` against 537 curated `.md` is roughly the
ratio § 41 predicts.

## 6. v2 data and schema against the design

Store format `entity.file-store/2`; one JSON file per subject under `store/subjects/<hex(kind)>/<hex(id)>.json`
with envelope `{format, origin, instance:{entity, version, id, lifecycle_state, revision, fields}, records:[…]}`.
Record counts (files under `store/subjects/`, 5,304 total): workload 1,305, ticket 554, lesson 443,
request 400, commitment 362, flag 314, database 270, operational-warning 242, node 213, source-cursor 207,
gitlab-project 128, lookup 124, namespace 115, repository 89, component 62, slack-channel 57, task 49,
communication 37, gate 35, goal 29, incident/customer/blocker/aurora-cluster 28 each, decision 26,
person 22, environment 21, availability 20, jira-project 17, team 13, vendor 12, workstream/source/
confluence-space/cluster 5 each, aws-account 3.

| # | v2 | design | severity | import policy |
|---|---|---|---|---|
| 1 | ids are human-readable and domain-meaningful: `<first.last>` for a person, `dec-0020`, `DEV-520` (Jira native), sha256 hex for a workload | § 6.4 names are attributes (design:374-380) | hard | mint `NodeId`s; the v2 id becomes `canonical_name` plus an alias; `views/attention.index.json` item refs and `annex/*` filenames are re-pointed |
| 2 | field `provenance` is an enum `observed | asserted | derived | inherited` (`definitions/person.yaml:20`) | `provenance: Vec<EvidenceId>` on edges (design:708); § 6.5 | hard name clash, different semantics | v2 `provenance` imports as a property `origin`; `inherited` means no evidence and is never `Accepted` |
| 3 | `revision` is per subject, +1 per operation | `revision: u64` is the global root revision (design:1637-1647) | hard | v2 revision → property `source_revision`; no global order is reconstructed |
| 4 | `lifecycle_state` describes the world (`active | departed | unknown`, `open | answered`, `mirrored`) | `KnowledgeState` describes knowing (design:1400-1410); `GraphRoot.lifecycle` (design:2752) | hard, same slot name, orthogonal axis | keep both: v2 lifecycle becomes the per-type lifecycle of A13; `KnowledgeState` stays separate |
| 5 | one store tier; unreliability marked in-band with `unverified[]`, `absent[]`, `verified{horizon_days}` | § 6.1 one-way membrane (design:330-350); § 76.2 "do not let UI labels or confidence scores substitute for architectural isolation" (design:2996-2999) | design | everything imports into transient root `v2-store` whose local schema is the 36 definitions; integration per kind through mappings |
| 6 | `source-cursor` × 207 stored as ordinary entities | § 55 "checkpoint state should be durable but not confused with canonical knowledge" (design:2286) | must relocate | → checkpoint store in `ekr-observe` |
| 7 | infrastructure mirrored as records: workload 1,305, database 270, node 213, namespace 115 (1,903 = 36 % of all records) | § 76.4 "canonical state should capture useful semantics, not mirror every source format" (design:3004-3006) | policy | transient root `infrastructure` with its own stable schema (§ 53); no canonical promotion by default |
| 8 | kinds with no design counterpart: `flag` 314, `operational-warning` 242, `lookup` 124, `gate` 35 | ontology is open (§ 11) but gives no pattern for advisory records | soft | stay transient until a schema proposal (§ 25) earns them a type |
| 9 | whole-entity records; no subject–predicate–object assertions; field-level provenance only through `unverified[]` / `absent[]` | § 13 the assertion is the primitive (design:727-778) | structural | do not shred 5,304 subjects × N fields up front; integration plans derive assertions per mapping |
| 10 | no valid time; only `observed_at` / `recorded_at` | § 14 | gap | `valid_from = recorded_at`, flagged inferred |
| 11 | `source: [{kind, ref}]` are inline strings; some refs point into v1 files (`knowledge/state/observations.jsonl`); no content hashes | § 16 evidence is content-addressed and independently referenceable (design:848-890) | gap | evidence minted from `raw/` where the page still exists; otherwise `HumanStatement`-class evidence with `origin: inherited` |
| 12 | conflicts are refused at write time ("refuses older changes and conflicting claims at the same timestamp", `org-brain/README.md`); the losing claim is not stored | § 44 `Disputed` is a state to investigate (design:1965-1992) | semantics reversal | nothing to import; the dispute set starts empty |
| 13 | per-subject `records[]` history is ≈99 % of the 3.6 GB (inferred: 3.6 GB ÷ 5,304 files ≈ 680 KB per file) | § 34 revision lineage is global (design:1632-1671) | size | history → Provenance storage class as content-addressed blobs; not replayed into v3 revisions |

Design concepts with no v2 data at all (gaps, not conflicts): agents and `TrustProfile` (design:2085-2090);
`SchemaVersionId` registry (design:2757-2761 — v2 has only per-entity `version: 1`); transient graph
roots; `RootUtility` (design:1852-1861); retraction and supersession lineage.

## 7. What the v1 → v2 import already lost

From `org-brain/import-report.json` (`recorded_at: 2026-09-03T22:21:52Z`): 3,037 rows written across
27 kinds (1,945 created, 773 moved, 427 synthesised, 477 amended, 22 restated, 44 refused).

| lost | count | note |
|---|---|---|
| whole v1 types skipped | `finding` 5,545, `entry` 3,493, `deployment` 789, `statement` 639, `state_change` 68, `correction` 64, `causal_link` 20, `round` 7 — ≈10,600 records | no v2 home |
| synthesised to satisfy references | 427: `commitment` 181, `incident` 138, `person` 18, … | these records have no source |
| refused | 44, concentrated in `task` 24 and `flag` 9 | lifecycle or vocabulary mismatch (e.g. `operation 'amend_progress' is not defined`, `'moot' is not valid from lifecycle state 'decided'`) |
| field-level drops | 2 (`person.status=n/a`, `repository.row_shared_with`) | |

Consequence for P7: the skipped kinds are imported from v1 directly, not through v2; synthesised v2
records carry `origin: inherited` with no evidence and stay transient.

## 8. Known v2 defects carried as requirements

| defect | source | requirement it becomes |
|---|---|---|
| interpretation stopped on 2026-09-07; "historical coverage and interpretation remain incomplete" | `org-brain/README.md:12-17`, `annex/incident/2026-09-07-brain-knowledge-gap.md` | P5: a stalled interpreter is a visible status, never a silent gap; collectors continue |
| thread replies missed despite successful polls | same incident | P2 (A8): a completed poll establishes only its own window; replies are their own units |
| `unlimited_budget = true` set by hand for repair | `org-brain/brain.toml:38-39` | P5 (A7): the stop condition cannot be removed without an approval record |
| four of eighteen v1 gates "wallpaper", four "cannot fire on the thing they name"; 10 of 12 lessons recurred, 7 within 24 h | `company-brain/README.md:14-24`, `docs/reviews/2026-08-20-defect-register-and-plan.md` | P6 (A9): recurrence is a metric; a gate that never fires is reported |

## 9. Import policy, in five rules

1. Raw source pages (v1 `knowledge/raw/`, v2 `raw/`) become Observations, content-addressed, in P2.
2. Curated or interpreted records (v1 `.md`, v1 `observations.jsonl`, v2 `store/`) enter transient
   roots — `v1-workday`, `v2-store`, `infrastructure` — whose local schemas are the predecessors' own
   definitions. They never enter the canonical core directly.
3. Every imported record carries `origin: inherited` and no `Accepted` state until a live source
   re-observes it (v2 engine design § 7.9, kept).
4. Identities are minted; predecessor ids survive as `canonical_name` or aliases. Predecessor
   `provenance`, `revision` and `lifecycle_state` are renamed on import (§ 6 rows 2–4).
5. Checkpoints (`source-cursor`) and per-subject history are not knowledge: checkpoints go to the
   checkpoint store, history to Provenance-class blobs.

## 10. Code worth lifting (not depending on)

| what | where | into |
|---|---|---|
| adapter declarations for Slack, Jira, Confluence, GitLab, GitHub | `org-brain-successor/adapters/*.yaml` | `ekr-adapters` |
| request/receipt protocol shape | engine design amendment 2026-09-05 (`brain.extraction-request/4`, `receipt/4`, no-op `basis`) | `ekr-interpret` |
| view templates | `org-brain-successor/views/{attention,brief}.md.j2` | `ekr-views` |
| MCP facade | `org-brain/src/mcp.rs` (418 LOC, no SDK) | `ekr-mcp` |
| cost accounting | `org-brain-successor/crates/brain-round` | `ekr-runtime` |
| adversary cases named after the false pass they catch | `org-brain/tests/adversary/case-*.sh` (11) | this repository's test suite |
| privacy projection and credential policy | `org-brain/src/dashboard/privacy.rs`; engine credential policy 3 | `ekr-observe` |

## 11. How this was measured

Two read-only surveys on 2026-09-21, one per predecessor, each capped at inventory depth (directory
listings, `find | wc -l`, sampled records, the repositories' own `AGENTS.md`, `README.md`,
`CHANGELOG.md` and design documents). The v2 engine's design contract
(`org-brain-successor/docs/design/org-brain-v0.1.md`, 527 lines) and `entity-runtime/README.md` were
read in full. No function bodies were audited; capability rows name the file, not its correctness.
Unknown after this survey: whether entity-runtime's later releases (0.18.1 is current) add anything
resembling a canonical/transient membrane or bitemporality — its README and guide index do not
mention either.
