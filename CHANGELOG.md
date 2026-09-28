# Changelog

Every change a user of the runtime sees, per release. Unreleased work sits at the top.

## [Unreleased]

## [0.0.13] — 2026-09-28

Agents read the store through MCP; write verbs cost less inside a session.

### Added

- `ekr mcp`: a read-only MCP server over stdio (JSON-RPC 2.0, one message per line), protocol
  `2025-11-25` or `2025-06-18`. Seven tools: `overview`, `search`, `describe_node`, `expand` and
  `timeline` answer the `ekr.views` documents byte for byte; `explain` and `resolve` answer
  exactly what `ekr explain` and `ekr resolve` print. A refusal is an `isError` result with the
  same refusal name `ekr view` or the verb gives; no tool proposes, validates or commits. The
  server opens one store, re-reads the head on every call and sees commits another process makes.
  `ekr session` refuses `mcp`. See `docs/cli.md` § `ekr mcp` for arguments and error codes.

### Changed

- Write verbs: a store handle authorizes a publication preparation once, and the candidate replay
  still precedes every append; the kernel answers a verdict without copying the head graph;
  replay states are shared, not copied. Inside `ekr session` on a 19 MB file store (tmpfs,
  median): `propose` 120–167 → 58–88 ms, `validate` 140–188 → 78–144 ms, `commit`
  185–277 → 123–212 ms.
- The views conformance suite is held to `systems/ekr/conformance/views-baseline.json`: every
  scenario name, each authored scenario's sha256, and a step-count floor per scenario.
- AEP 0.64.0 (from 0.63.1) for the planning store; `.engineering/project.yaml` names the 0.64.0
  tag commit (`58433bd8`) as its protocols.

### Known limits

- A `commit` still costs more than 100 ms in a session: the replay checkpoint on every commit and
  the pointer after every verb (`task:checkpoint-cadence-costs-each-commit`), and the eventlog file
  provider re-hashing the log after a write and syncing 28 times per verb
  (`task:eventlog-rehash-and-fsync-per-write`).
- No tool yet answers what changed since a revision or a time (`story:changes-since-read`).

## [0.0.12] — 2026-09-28

Ingestion throughput: many calls over one opened store, and a store verified and decoded once per
process.

### Added

- `ekr session`: reads one JSON request per line, `{"argv": [...], "stdin": "..."}`, parses `argv`
  with the same definitions as the one-shot verbs, and answers one line
  `{"exit": N, "stdout": <document>, "stderr": "..."}`, where `stdout` is the document the one-shot
  verb prints. It serves `propose`, `validate`, `commit`, `snapshot`, `explain`, `resolve`, `head`,
  `transactions`, `ontology`, `mint`, `hash` and `schema` over one opened store, and a commit is
  seen by the next read, including one made by another process. New refusals (exit 2):
  `session-request-malformed`, `session-verb-unknown`, `session-verb-refused`,
  `session-option-refused` and `session-request-too-large` (a line over 25,231,360 bytes).

### Changed

- A store handle verifies a retained blob once and keeps the revision stream it has read; a later
  read asks the provider only for newer events and re-checks the last one it holds, so a replaced
  or diverged store is still refused. `RetainedHistory::content` hashes an object at most once per
  process.
- The seed envelope is decoded once per process and shared by checkpoint restore, the verified
  read and the commit path; before publishing, the staged envelope bytes are decoded, so a seed
  whose bytes do not decode is refused rather than published.
- On a store seeded with 11 MB of evidence (112 MB on disk, release build), one `ekr resolve`
  takes 1.47 s on file (was 2.58 s) and 0.77 s on SQLite (was 1.72 s); inside `ekr session` the
  same request takes 33.9 ms and 19.8 ms.

### Known limits

- A session request still costs 20–34 ms on that store, against 4.4 ms on a 1.4 MB store
  (`task:session-request-cost-grows-with-evidence`).
- The seed envelope still embeds evidence payloads as JSON integer arrays
  (`story:seed-envelope-v3-references-payloads`).

## [0.0.11] — 2026-09-28

The first runtime code of the views, observation and integration domains, the streamed `ekr view`
viewer, validation profile v3, fixes from an external review, and the planning store on
`aep.project/5` under AEP 0.63.1.

### Added

- `ekr-views`: renders `ekr.graph-projection/1` from a committed revision. Rendering is a pure
  function of the loaded revision, so two renders of one revision are byte-identical, in one
  process and after reopening the store. On a store of 302 revisions (release build) the head
  renders in 1.04 s (file) and 286 ms (SQLite) right after opening, and 239 ms / 177 ms after
  that.
- `ekr-observe`: maps a JSONL file of source records to one observation per line. The content
  hash covers the line's bytes, and the id is derived from the idempotency key (source,
  source-native id, content hash), so the same record always gets the same id. Nothing is
  persisted yet.
- `ekr-integrate`: resolves a typed reference to one node of exactly its type holding one of its
  aliases, to a new-node proposal, or to an ambiguity listing every candidate. It never compares
  names and never picks between candidates. It refuses a reference with no alias, a type with
  subtypes, and a type the ontology does not declare (`reference-type-undeclared`, new in
  `ekr.integrate`).
- `ekr view [--port N]`: serves a read-only viewer for a store on 127.0.0.1 only. `/` is the
  embedded page, `/projection[?revision=N]` the bytes `ekr-views` renders, `/evidence/<id>` the
  retained bytes as `text/plain` or `application/octet-stream`, always with `nosniff`. It answers
  421 unless the `Host` names the server, 413 for any request body, 503 over 64 connections, and a
  400 for a request head not complete 5 s after the connection opens. It never reads a body.
- `ekr resolve <reference.yaml> [--at N]`: resolves a typed-reference document against the
  canonical snapshot and prints the outcome as one JSON document; `ekr schema` and `ekr example`
  gain the `typed-reference` format, and the guide says to resolve before `CreateNode`. The
  reader refuses YAML aliases, tags, non-string scalars and nesting past 64 levels before it
  loads the document.
- `ekr.views` gains five read commands, each with its format and scenarios: `ProjectOverview`
  (`ekr.graph-overview/1`), `ExpandNeighbourhood` (`ekr.graph-slice/1`), `DescribeNode`
  (`ekr.node-detail/1`), `SearchNodes` (`ekr.node-matches/1`) and `ProjectTimeline`
  (`ekr.graph-timeline/1`). `ekr-views` implements them over a per-revision index. The views
  suite runs 41 scenarios on the file and SQLite providers; the kernel suite runs 40.
- `ekr view` serves them: `/overview`, `/expand` (streamed as chunked NDJSON, a page at most 2,000
  nodes and 5,000 edges), `/node/<id>`, `/search` and `/timeline`. The page is the operator's
  prototype viewer (2D and 3D graph, timeline heatmap and swimlanes with one row per subject,
  property history, schema history, command palette) and never reads `/projection`; the earlier
  page is at `/alt`. The page's CSP is sent as a header with `frame-ancestors 'none'`, and every
  script is pinned by SRI. The indexes of the 3 revisions used most recently are kept.
- Validation profile v3 (`ekr.p3-deterministic/1` with `ekr.p2-apply/1`): profile v2 plus a
  refusal of any `CreateNode` or `CreateEdge` whose id an earlier revision held, such as a deleted
  edge's (`identity-previously-held`).
- `CreateNode` carries aliases, so a created node resolves at the next revision;
  `alias-already-exists` and `duplicate-alias` refuse a second node of one type holding one alias.

### Changed

- `ekr_views::load` replays a revision once and reads the schema at every schema-version boundary
  from that one pass (`Runtime::schema_history`), where it replayed from revision 0 once per
  boundary. On a store of 2,288 nodes, 6,314 edges and 5 schema versions (release build, two
  runs each), the first `/overview` of the head went from 14.26 s and 10.78 s to 0.78 s and 0.83 s.
- The planning store moves from `aep.project/4` to AEP's Git-native `aep.project/5`: one Markdown
  file per artifact and one file per evidence record. AEP 0.63.1 (from 0.61.1) in the Taskfile, CI
  and `plan-check`; `.engineering/project.yaml` names the 0.63.0 tag commit (`5bd56624`) as its
  protocols.
- The correctness CI installs ESS, AEP and go-task from checksummed release archives instead of
  building them, builds dependencies at opt-level 3 with no debug info, and runs the conformance
  binary once. The run the change started from took 1,818 s.

### Fixed

- A replay checkpoint whose cached graph-root id, seed payloads or held ids differ from its
  history is refused on restore (`checkpoint-graph-root-identity`, `checkpoint-seed-payloads`,
  `checkpoint-held-identities`), so a corrupted checkpoint can no longer admit a commit that full
  replay then rejects, or drop evidence bytes from a verified read.
- A node and an edge that share one id no longer break the projection: assertion buckets are keyed
  by node id and edge id apart.

### Known limits

- A historical projection carries the store's current head, so rendering revision 0 before and
  after an unrelated commit gives different bytes.
- The views suite's baseline is regenerated from the suite it checks, so a scenario deleted from
  the specification lowers its own floor.

## [0.0.10] — 2026-09-27

The planning store on `aep.project/4`, AEP 0.61.1, ESS 0.36.0 and the `ess/14` source format.

### Changed

- The planning store moves from `aep.project/3` to `aep.project/4`
  (`aep plan store migrate content`, AEP 0.61.1): large values are content blobs under
  `.engineering/blobs`, stored once. The committed store shrinks from 13,818 files and 67.5 MiB
  to 17,407 files and 26.6 MiB. The migration proved 4,296 subjects and 1,278 records equal, and
  `aep plan artifact list`, `validate`, `history` and `explain` give byte-identical output before
  and after for all 197 artifacts.
- AEP 0.61.1 (from 0.60.1) for the planning store; `plan-check` refuses any other `aep`.
  `.engineering/project.yaml` names the 0.61.1 tag commit (`b6213e1f`) as its protocols; the
  governing documents are the same as at 0.60.1 (`a0ad90cc`), so no artifact needs a catch-up.
- ESS 0.36.0 (from 0.35.1) for the specification and the conformance target; `spec-check` and
  `conform-check` refuse any other `ess`. `systems/ekr/system.yaml` moves from `format: ess/13` to
  `ess/14`, the newest source format 0.36.0 reads. The model uses none of the value expressions
  `ess/14` adds and trips none of the rules 0.36.0 tightens, so no declaration changes; both
  suites resynthesize byte-identical and the baseline digest is unchanged.

### Known limits

- `aep plan store inspect` in AEP 0.61.1 still refuses this store's selector as
  `source_unreadable`; `migrate content` is the migration verb that reads a tree store.

## [0.0.9] — 2026-09-27

ESS 0.35.1 and the `ess/13` source format, AEP 0.60.1, and the planning store on its protocols.

### Changed

- ESS 0.35.1 (from 0.33.0) for the specification and the conformance target; `spec-check` and
  `conform-check` refuse any other `ess`. `systems/ekr/system.yaml` moves from `format: ess/7` to
  `ess/13`, the newest source format 0.35.1 reads; the model uses none of the constructs `ess/8`
  to `ess/13` add, so the format line alone changes no compiled byte.
- `ekr.kernel.GraphTransaction.operation_count` and the `operation_count` column of the
  `ekr.kernel.Transactions` and `ekr.kernel.PendingTransactions` views are `Optional<Integer>`.
  ESS 0.35 refuses a required field that an invariant reads and a creating outcome does not set
  (`ESS-COMMAND-018`); `Propose` derives the count from the retained document, which ESS has no
  source to name in `sets:`. The invariant `operation_count >= 1` and every value the runtime
  reports are unchanged.
- The `ekr-kernel` and `ekr-views` suites are resynthesized with 0.35.1 and the baseline takes the
  new digest. The kernel keeps its 40 scenarios; `Validate/outcome/transaction-not-found` and
  `Commit/outcome/transaction-not-found` now send an identity no record carries, which 0.35.1
  reads as the declared answer for a missing transaction. ESS 0.34 and 0.35.0 expected
  `wrong-state` there instead, contrary to the domain, which is why 0.35.0 was not used.
- AEP 0.60.1 (from 0.60.0) for the planning store; `plan-check` refuses any other `aep`.
- The planning store's meta schema moves from the AEP 0.55.0 protocols (`28abe09b`) to the AEP
  0.60.1 protocols (`a0ad90cc`): `.engineering/project.yaml` names the 0.60.1 tag commit. The
  governing documents are the same in both (`aep govern validate`: 60 files, 5 protocols,
  13 lifecycles, 10 profiles, valid), so no artifact needs a catch-up; `aep plan artifact list`
  and `validate` give identical output before and after (197 artifacts, valid).
- The shared source gate workflow is pinned at the Gates 0.1.10 tag commit (`eb5440b0`); its
  `common.yml` is unchanged from the 0.1.8 pin and still runs the verified 0.1.8 binary.

### Known limits

- The planning store stays on `aep.project/3`, the newest format AEP 0.60.1 knows. Its
  `plan store inspect`, `verify` and `migrate dry-run` refuse a `/3` selector as
  `source_unreadable` (`crates/edge/aep-cli/src/store_command.rs:2168` in AEP 0.60.1: "a tree
  authority, which these migration verbs do not read"), `plan store export` reads only an
  `aep.project/2` store, and `init-tree` refuses a project that already has a selector.

## [0.0.8] — 2026-09-27

The specifications of the graph projection, observation and integration domains, and AEP 0.60.0.

### Added

- Three new ESS domains in `systems/ekr/`, specification only (no runtime code yet):
  - `ekr.views` (component `ekr-views`): the graph projection `ekr.graph-projection/1` a viewer
    renders from one committed revision, with its determinism rules, and its own conformance suite
    `systems/ekr/conformance/views-suite.json` (9 scenarios, 6 authored), checked by
    `task conform-check`.
  - `ekr.observe` (component `ekr-observe`): source units, checkpoints, poll health and the
    observation idempotency key; every link still undecided is an `UNMAPPED:` marker naming its
    decision-blocker.
  - `ekr.integrate` (component `ekr-integrate`): typed references, resolution outcomes and the
    merge and split lineage, with `MergeId` and `SplitId` in `ekr-core`.
- The `ekr-kernel` conformance suite and baseline are regenerated for the larger system; the
  kernel's 40 scenarios are unchanged.

### Changed

- AEP 0.60.0 (from 0.59.3) for the planning store; `plan-check` refuses any other `aep`. The store
  is not migrated: `aep.project/3` is the newest format 0.60.0 knows, and its `plan store
  inspect`, `migrate`, `verify` and `rebuild` read only `aep.project/1` and `/2` stores. On both
  versions `aep plan artifact validate` reports the store valid and `aep plan artifact list` gives
  the same 197 artifacts with the same statuses.
- ESS stays at 0.33.0. ESS 0.35.0 refuses the kernel domain (`ESS-COMMAND-018`: `Propose` creates
  a `GraphTransaction` without setting `operation_count`, which an invariant reads), and its
  synthesized `Validate` scenario for an unknown transaction expects `wrong-state` where the domain
  declares `transaction-not-found`.
- The shared source gate runs on Gates 0.1.8 (from 0.1.5), whose scan limit is 512 MiB.

## [0.0.7] — 2026-09-26

Store performance at a few thousand observations (design § 96), and a transaction document format
that holds 10,000 operations (design § 97).

### Added

- `ekr.transaction-document/2`: the fields of `/1`, up to 10,000 operations and 10,000 evidence
  entries in at most 8 MiB (8388608 bytes); `docs/cli.md` § Document limits lists every limit of
  both versions. `ekr example`, `ekr schema` and `ekr guide` now write `/2`; `ekr schema
  ekr.transaction-document/1` still prints the original. A `/1` document is still read, validated
  and replayed under its frozen limits of 256 operations and 262144 bytes, and every retained `/1`
  proposal is unchanged. Write `format: ekr.transaction-document/2` on its own top-level line, as
  the examples do: a document over 262144 bytes whose format is not on such a line is held to the
  `/1` byte cap. On a store of 65 revisions (70 MB, file provider), one propose, validate and
  commit took 1.20 + 1.08 + 1.31 s (3.60 s) for 2,000 operations and 2.40 + 1.45 + 2.22 s (6.08 s)
  for 10,000; median of three runs, each on a fresh copy of the store.

### Changed

- A verb no longer replays the whole history on every open. Every commit leaves one replay
  checkpoint of the head it published; the next verb continues from it, and `ekr head` answers
  from the checkpoint pointer and the head record alone. On a store of 65 revisions and 7,190
  assertions `ekr head` takes 0.34 s (from 5.1 s) and one propose, validate and commit of a
  250-operation document 3.2 s (from 78 s). `--full-replay` replays from the seed, re-deriving
  every retained decision, as every verb did before.
- Within one command, a history is replayed once: the states a command reaches are reused, each
  retained document is parsed once, and a commit reuses its validation's sealed result.
- New proposal records, commit receipts and publication preparations write byte strings as
  base64, and a preparation holds each staged object once: `ekr.proposal-record/2`,
  `ekr.commit-receipt/2`, `ekr.publication-preparation/2`. The same history takes 70 MB where it
  took 361 MB. Records and preparations in the `/1` formats are still read, and re-encode to their
  original bytes; a store written by 0.0.6 opens and answers unchanged.
- A refused document names its own version's bound, for example `transaction document limit:
  operations (at most 10000 operations per document; split the change into several
  transactions)`; a `/1` document over 256 operations is told that `/2` admits 10000.

## [0.0.6] — 2026-09-26

Seed evidence from files, and the visual documentation.

### Added

- `ekr seed --evidence <file>`, repeatable: each file's exact bytes join `evidence_payloads` under
  their content hash, so a seed can say `evidence_payloads: {}` instead of carrying byte lists. The
  kernel checks the completed document as it does a pasted one, and gives the same roots; a payload
  both pasted and passed lands once. A file that cannot be read exits 1 before any store is opened.
  `ekr guide` and `docs/cli.md` § `ekr seed` say how.
- `docs/overview.md` (how EKR works, with diagrams, and what 0.0.6 has versus what is planned),
  `docs/guide.md` (a task-oriented walk through a real store) and `docs/schema-evolution.md`
  (growing the schema under validation profile v2). The documentation manifest publishes them with
  `docs/cli.md`.
- `docs/cli.md`'s refusal table lists `seed-space`: a seed whose graph root is not `Canonical`.

## [0.0.5] — 2026-09-26

The latest beyond10x dependencies.

### Changed

- ESS 0.33.0 (from 0.32.0) for the specification and the conformance target; the synthesized
  suite is byte-identical. `spec-check` and `conform-check` refuse any other `ess`.
- Eventlog 0.5.0 (from 0.4.0).
- `jsonschema` 0.58 (from 0.57), and the lockfile's compatible updates.

## [0.0.4] — 2026-09-26

Schema evolution, store open semantics, and the planning store on `aep.project/3`.

### Added

- Schema evolution. A store seeded under validation profile v2 (`ekr.p2-deterministic/1` +
  `ekr.p2-apply/1`) admits `DefineNodeType`, `DefineEdgeType` and `ModifyProperty` in committed,
  schema-only transactions. Each change produces a new schema version (number + 1, parent = the
  prior version), carried in the transaction's optional `schema_version`. The change is checked
  against canonical state: a property made required while an instance lacks it, a cardinality
  narrowed below held values, a value type or constraint changed under held values or assertion
  objects, and a no-op change are refused with named issues.
- `ModifyProperty` names its owner type: `{owner, property}`.
- `ekr ontology --at <revision>` shows the schema as of a revision, with its version number and
  parent. `ekr example schema-change`; `docs/cli.md` § Evolve the schema; `ekr guide` says how to
  seed a v2 store from the example host.
- `store-not-found` (exit 1): every verb but `seed` opens only an existing store and creates
  nothing at a path that holds none.

### Changed

- A refused `seed` creates nothing at its path: admission, the tenant and the host authority are
  checked before any provider store is created.
- `ekr seed` on an existing store under a host with a different authority is
  `bootstrap-authority-mismatch` (exit 1), as for every other store verb.
- Opening a SQLite store waits out another process's open lock (at most about 10 s), so concurrent
  identical `seed` and `commit` invocations each succeed.

### Unchanged

- A v1 store (every store seeded before this release) keeps replaying byte for byte, including
  retained proposals of the P1 `ModifyProperty` shape, and still refuses the three schema kinds as
  `unsupported-operation`. Moving an existing store to v2 is not yet possible.
- `MergeEntity` is still refused.

## [0.0.3] — 2026-09-26

A platform refresh and the first user documentation.

### Added

- `docs/cli.md`: the `ekr` CLI for users and agents — configuration, every verb, the workflow, the
  `ekr-seed/2` format field by field with the schema (`ontology:`) section, the transaction document,
  the P1 limits, a worked example and the common refusals. A test holds every document block, table
  and refusal on the page to the binary.
- `ekr hash <file|->`: the content hash and the pasteable byte list a seed evidence entry needs; the
  two seed evidence refusals name the expected and the found hash.
- `ekr schema <format>`: the JSON Schema (draft 2020-12) of `ekr-seed/2`,
  `ekr.transaction-document/1` and `ekr.cli-host/1`, generated from the types the readers decode.
  The schemas carry the frozen document limits, integer ranges and the host's fixed texts; each
  description, and `docs/cli.md`, names every case where the schema and the reader still differ.
  The schema is a first check; the reader decides.
- README rewritten for users, with a first run.

### Changed

- ESS 0.32.0 (from 0.29.0) for the specification and the conformance target; `spec-check` and
  `conform-check` refuse any other `ess`.
- Eventlog 0.4.0 (from rev `28e57856`). A command history now loads in a constant number of provider
  calls, so file-provider commands grow linearly: propose at revision 40 went from 2,487 ms to
  463 ms (release build; 221 ms at revision 1).

### Known

- A schema is declared only in the seed: `DefineNodeType`, `DefineEdgeType`, `ModifyProperty` and
  `MergeEntity` are refused as `unsupported-operation`. Schema evolution is the next wave.
- Each command still verifies the whole file-provider log once when it opens the store; reads of a
  past revision cost one provider call per event (`task:selected-revision-loads-read-one-event-per-call`).

## [0.0.2] — 2026-09-25

The P1 exit: the kernel, ontology, graph and store are complete, conforming and reachable through
the `ekr` binary. This section also carries the entries written before 0.0.1, which had no section
of its own.

### Added

- **ESS conformance.** The kernel's executable specification runs through the real runtime on the
  file and SQLite providers: 40 of 40 scenarios (36 generated, 4 authored), gated as
  `task conform-check` (wave p1-14).
- **P1 exit properties.** No transaction naming an absent node, edge, assertion, evidence or graph
  root commits (105 generated strata, both providers); replay from the seed reproduces every root
  of a generated lineage; every canonical reference kind is typed by its target, with a
  compile-fail case per kind (wave p1-14).
- `ekr-store` and the kernel `Runtime`: a read-only read of the events the store published.
- `ekr`: `seed`, `propose`, `validate`, `commit`, `snapshot` and `explain` over both providers, with
  exit codes 0 (declared outcome), 1 (fault) and 2 (declared refusal) (wave p1-13).

### Changed

- The kernel domain declares the store publications its writing commands emit, and
  `ekr.kernel.CurrentRevision` no longer declares an order (wave p1-14).
- `ekr.store.Snapshot` and `ekr.store.SnapshotId` are removed from the store domain; nothing
  implemented them (wave p1-14).

### Known

- On the file provider every read re-hashes the whole event log, so a command's cost grows with
  the square of the number of revisions (propose: 344 ms at revision 1, 2,487 ms at revision 40,
  release build). SQLite grows about linearly. Eventlog 0.4.0 addresses it; adopting it is pending
  (`task:store-reads-rehash-the-whole-log`).

### Added before 0.0.1

- **Invariants 1 and 2 are now true.** An independent review found both claimed and carried by
  nothing. Only the kernel reaches a writer to canonical state: the binary no longer declares the
  store, the kernel holds the only public commit path, and the fold learns validation through a
  trait the kernel implements rather than from an event anyone could append. And canonical state
  references by a typed reference rather than a bare identifier, so an edge into the incubation
  forest does not compile (wave p1-06).
- `ekr-store`: a commit validated against a revision the lineage has moved past is refused on
  replay, and a store opened with no commit authority refuses to say what canonical state is
  rather than folding nothing and reporting success (wave p1-06).
- `ekr-kernel`: the integrity membrane. A `GraphTransaction` is a proposal; seven deterministic
  validators turn it into a `ValidatedTransaction`, which only this crate can construct and which
  is the only thing that commits. A transaction is read as a set rather than a sequence, so no
  verdict depends on the order the operations arrive in. A proposal states a claim and never the
  verdict on it (wave p1-05).
- `ekr-store`: persistence through the eventlog. A revision log with a fold, a replay and two
  providers, content-addressed objects that record the strongest retention class ever asked of
  them, and one named crossing into canonical state that reads every field of a document which has
  something to disagree with. The log is synchronous over an asynchronous port (wave p1-05).
- `ekr-graph`: the value a canonical record may carry admits no float, and the graph types are
  generic over the value they hold, so the canonical encoding exists exactly where canonical state
  does and an incubation-forest candidate has no address at all (wave p1-05).
- `ekr-graph`: the graph model. Nodes, edges, bitemporal assertions with their evidence, graph and
  revision roots, and the six-variant revision event vocabulary the kernel publishes and the
  store persists. `GraphSnapshot` answers one read, `valid_at(Timestamp)` — the current-world
  query at the caller's now, the historical query at any other instant, because the runtime has no
  clock. A `Canonical` reference cannot target a transient type: the bound is sealed and three
  `trybuild` cases hold it (wave p1-04).
- `ekr-core`: `Timestamp`, an `i64` millisecond newtype, and `Encoder::variant`, the variant tag a
  sum type carries under the canonical encoding (wave p1-04).
- `ekr-ontology`: the ontology document and its type checker. Node and edge types with parents,
  properties and cardinality; a `ValueType`/`Value` pair that makes `employer = NodeRef(..)`
  checkable and `employer = "OpenAI"` not; per-type lifecycles and named operations. A document
  loads only when every declaration is coherent, and an inherited property is resolved by the
  specialisation order the document declares rather than by distance or by id (wave p1-03).
- `ekr-core`: stable identity (fifteen newtypes, one text form each), content hashing in two
  address domains that cannot collide, and a deterministic canonical encoding the encoder imposes
  rather than the caller (wave p1-02).
- The six P1 crates — `ekr-core`, `ekr-kernel`, `ekr-ontology`, `ekr-graph`, `ekr-store`, `ekr` — as
  empty workspace members in the acyclic order `docs/roadmap.md` § 3 gives, every P1 dependency
  declared, `rust-version` 1.91, and contract tests holding the manifests, the lockfile, README and
  the doc comments to the story (wave p1-01).
- Repository bootstrap: Cargo workspace with `xtask`, the shared source gate, the AEP planning
  store, and the roadmap and predecessor analysis under `docs/`.
- `systems/ekr/`: the runtime's own executable system specification (`ess/1`), four domains —
  kernel, ontology, graph, store — validated by `task spec-check`.
- Design amendments 81–87 to `docs/epistemic-knowledge-runtime-design.md`, carrying what v1
  (`company-brain`) and v2 (`org-brain`) proved necessary: operator surface, projections and views,
  the outward-write invariant, obligations and horizons, the interpretation session contract, blob
  evidence, and per-type lifecycles.
