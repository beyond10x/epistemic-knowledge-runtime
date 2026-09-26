# Changelog

Every change a user of the runtime sees, per release. Unreleased work sits at the top.

## [Unreleased]

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
