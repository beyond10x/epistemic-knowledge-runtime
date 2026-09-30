---
format: aep.planning-md/3
id: release-plan:next-waves-2026-09-30
kind: release-plan
status: draft
title: Waves extract-06 and extract-07 after 0.0.24
relations:
- serves: vision:o5
revision: 4
---
## Where this starts

0.0.24 on main (`ec6bbf38`, 2026-09-30). Waves perf-01/02, sdk-01/02/03, checks-01, reads-04 and ops-05
shipped 0.0.17–0.0.24; `release-plan:roadmap-2026-09-28` ends before them. A store audit on
2026-09-30 (29 findings) refined the plan first: `epic:consumer-sdk` and `epic:p6-maintenance-observability`
moved to active, `epic:ingestion-throughput` to implemented, its remaining read and storage work to
the new `epic:read-and-storage-cost`, two done tasks archived, orphan edges added, the type-packs
request filed as `epic:type-packs` with `decision-blocker:property-sensitivity-home`, the constraint
language filed as `decision-blocker:constraint-language`.

## The next two waves (13 items)

Selection rule: ready (dependencies implemented, no open blocker), and serving the two consumer needs
the store shows: a consumer's import through the SDK, and the operator's viewer and org-replay use.

### Wave extract-06 (0.0.25, 8 items)

| item | why now |
|---|---|
| `story:extraction-document-applies-to-a-store` | the format a consumer's extraction writes; retires about 1,700 consumer lines with the verb; gates the verb |
| `story:fact-quality-by-judged-sample` | the consumer's sampled fact-quality check in the engine; gates the SDK's judged-sample call |
| `story:explain-reads-an-index` | `explain` costs 4.6 s and answers 3 MB at a consumer's 1× shape (audit 2026-09-29) |
| `task:one-event-type-rule` | the timeline, `/roles` and `ekr ocel` disagree on which types are events; the replay's event log depends on it |
| `task:projection-carries-per-type-property-definitions` | the viewer refuses a revision whose subtype redeclares a property, which schema evolution now produces |
| `story:commit-cost-flat-with-store-size` | added 2026-09-30: a consumer's 76,830-fact import took 16,540 s; 71.1% of the session is in the SQLite stream read that the evidence count multiplies |
| `task:resolver-queue-drops-shared-alias-answers` | added 2026-09-30: the SDK resolver answers a stale cached node where the store is ambiguous, so a fact lands on the wrong node |
| `task:protocol-error-keeps-the-answer` | added 2026-09-30: a consumer's move onto `ProcessSession` waits on the protocol error keeping the answer |

Collisions: the three stories each add or change a verb, so they meet in the verb registry
(`crates/ekr/src/cli/mod.rs`, `agent.rs`, `tests/agent_cli.rs`, `docs/cli.md`) and the conformance
suites. `systems/ekr/domains/views.yaml` is edited by `task:one-event-type-rule` (the event-type
rule), `task:projection-carries-per-type-property-definitions` (`ekr.graph-projection/1`) and
`story:fact-quality-by-judged-sample` (its command and format): three additions in different
sections. `docs/cli.md` is also edited by `task:one-event-type-rule` (§ Roles at `:770`, § `ekr ocel`
at `:448`), away from the verb table (`:141`–`:166`) the stories change. The coordinator merges these
edits and regenerates the suites once, as in checks-01. The three items added on 2026-09-30 touch
`crates/ekr-store` (the commit-cost story) and `crates/ekr-sdk/src/{resolve,transport,session}.rs`
and `read/one_shot.rs` (the two SDK tasks), which no other extract-06 item edits; both SDK tasks add
to `CHANGELOG.md`.

### Wave extract-07 (0.0.26)

| item | why now |
|---|---|
| `story:extraction-verb-shares-the-sdk-path` | the engine's extraction verb over the SDK's apply path; needs extract-06's document |
| `story:sdk-store-checks` (the judged sample) | the SDK call for extract-06's fact-quality read |
| `story:preparation-blobs-are-reclaimed` | the store at 1× is about 522 MB, half of it preparations (audit) ; after explain-index, since both change `replay.rs`, `explain.rs` and `kernel.yaml` |
| `task:divergence-is-a-typed-store-error` | long-running readers detect divergence by matching a provider's message text |
| `task:seed-document-bounds-alias-expansion` | `ekr seed` has no bound on YAML alias expansion or nesting |

Collisions: `crates/ekr/src/cli/session.rs` is edited by `story:extraction-verb-shares-the-sdk-path`
(`respond`, `admit`) and `task:divergence-is-a-typed-store-error` (the divergence match and reopen,
`:622`); `crates/ekr-store/src/eventlog.rs` by `task:divergence-is-a-typed-store-error` (the typed
error) and `story:preparation-blobs-are-reclaimed` (the reclaim hook, `:1361`–`:1378`);
`crates/ekr/src/conformance.rs` by the extraction verb and the blob reclaim; the verb registry by the
extraction verb and `story:sdk-store-checks`' SDK side only; `crates/ekr-sdk/src/lib.rs` by both, each
adding one `pub mod` line (`extraction`, `checks`) to the list at `:7`–`:16`. Different functions in
each file; the
coordinator merges them and regenerates the suites once. `story:preparation-blobs-are-reclaimed`
depends on `story:explain-reads-an-index` (extract-06), which it follows.

### Coordinator decisions recorded here

- The extraction verb's shared apply routine lives in `ekr-sdk` with the SDK's own mirror of the
  extraction document; the `ekr` verb runs it over an in-process transport
  (`story:extraction-verb-shares-the-sdk-path` § Scope).
- Fact quality follows `story:store-quality-report`'s placement in `ekr-views` and `views.yaml`, not
  a new `ekr-metrics` crate.

## Waiting on the operator

| blocker | stops | evidence found (audit 2026-09-30) |
|---|---|---|
| `decision-blocker:source-unit-granularity` | P2: the source-adapter contract, checkpoints, poll health | design § 55 and ADR 0012 point to option 3 (a stream is checkpointed, an item observed) |
| `decision-blocker:checkpoint-unit-cardinality` | the same | A8 and `story:poll-health-proves-its-window` point to option 3 (one current checkpoint plus an immutable record per poll) |
| `decision-blocker:observation-retention-path` | P2's retained observations; AddEvidence beyond human statements; vision O2's explain to an external observation | design § 69, § 70, § 86 point to option 3; a persisted-format choice |
| `decision-blocker:relation-assertion-edge-correspondence` | degree and expansion of relation assertions; OCEL relationships from them | none; a model choice |
| `decision-blocker:merge-absorbed-node-fate`, `merge-across-node-types`, `split-reverting-merge-identity` | merging duplicates `ekr quality` lists | § 46 suggests keeping the absorbed node with a pointer |
| `decision-blocker:property-sensitivity-home` | `epic:type-packs` | none; an ontology-format choice |
| `decision-blocker:constraint-language` | `story:property-constraints-are-enforced` | none |

With the three P2 blockers answered, the next wave after extract-07 is P2's contract:
`story:source-adapter-contract` and `story:observations-are-retained`, then an attention queue
(vision O5's closing evidence, no artifact yet).
