# Changelog

Every change a user of the runtime sees, per release. Unreleased work sits at the top.

## [Unreleased]

### Fixed

- **`ekr migrate` migrates a store that took evidence after its seed.** It refused any store
  holding a committed `!AddEvidence` with `ekr: a stored document could not be read:
  required-object-missing`, exit 1, on both providers, because it published each commit without
  the evidence payloads the commit had brought, and replaying that commit reads them. Each commit
  is now published with those payloads, with the `stored_at` the source holds them at; a payload
  the source held below Provenance before its commit is stored at that class first and raised by
  the commit, as in the source. These payloads are no longer listed in the report's
  `carried_objects`. Every added evidence entry and its bytes are in the migrated store, and
  `ekr explain` of an assertion citing one answers as it did in the source.
- **`ekr seed` bounds its document's size, nesting and alias expansion.** It read a seed document
  of any size, and YAML aliases could make one decode to far more than it held: forty doubling
  aliases are 2^40 values. A seed document is now held to `ekr_kernel::SEED_LIMITS` before it is
  decoded, each refused as `ekr.kernel.InvalidSeed`, exit 2, before any store is created:
  `seed-too-large` past 16777216 bytes, of which `ekr seed` reads no more than the cap and one
  byte; `seed-too-deep` past 64 nested containers; `seed-alias-expansion` past 33554432 values,
  keys and containers or 16777216 bytes of text, each alias counted as everything it repeats
  (`docs/cli.md`, `ekr seed`). Every earlier seed refusal keeps its code. The YAML loader stops
  at the first container past the depth, so a deep nesting is refused at once.
- **A transaction document nested past its depth is refused before it is loaded.** The loader's
  scan is quadratic in flow nesting, and `ekr propose` checked depth only after loading the whole
  document. The loader now stops at the first container past the format's depth of 32; the
  refusal is unchanged, `transaction document limit: container_depth`.

## [0.0.25] — 2026-10-01

A long import costs less as the store grows; the SDK resolver no longer answers a stale node; the
extraction document, fact quality from a judged sample, explain by reference, one event-type rule
and per-type property definitions in the projection.

### Added

- **The extraction document, `ekr.extraction-document/1`** (`docs/cli.md`, Extraction
  documents): the format an extracting agent writes what it read from its sources in. It names
  the types it needs (node and edge types, with properties) instead of identifying them, lists the
  named things it found (a node type's name and aliases), its facts (`!Property` or `!Relation`)
  and the evidence items they cite, each the entry and payload an `!AddEvidence` carries.
  `ekr example ekr.extraction-document/1` (alias `extraction`) prints one for the example seed and
  `ekr schema ekr.extraction-document/1` its JSON Schema. Value types are written as a seed
  writes them, a `NodeRef` naming its types in `parameters: {allowed_types: [...]}`.
  `ekr_integrate::read_extraction` reads it against a store's ontology and refuses each problem by
  its code (`docs/cli.md` lists all sixteen): a document over 8388608 bytes, nested deeper than 32
  (the YAML loader stops there, so a deep nesting is refused at once), holding a YAML alias or
  malformed; a name declared twice; a store type redeclared with other parents, abstractness or
  cardinality; a type neither the document nor the store declares; an empty Enum or NodeRef; a
  reference with no identifying alias; an undeclared property; a value its property's type does
  not hold; a relation its edge type does not connect; a fact citing no evidence or an id the
  document does not carry; two evidence items under one id; and a payload that does not hash to
  its entry. The vendored YAML loader gains an opt-in depth bound for this. Specified, with the
  report applying a document will print, in `systems/ekr/domains/integrate.yaml`. No verb applies
  a document yet.

- **Fact quality is reported from a judged, reproducible sample** (`docs/cli.md`, `ekr sample`
  and `ekr fact-quality`). `ekr sample --seed S --size N [--type <type id>] [--revision N]` prints
  the `ekr.fact-sample/1` document: the revision's Active assertions ranked by the SHA-256 of
  `ekr.fact-sample/1:<seed>:<assertion id>`, the `N` lowest, each with its subject, the names a
  judge reads it by and the bytes of every evidence entry it cites. One seed, size, type and
  revision draw the same facts on either provider and in every run; a larger size extends a
  smaller one. `ekr fact-quality <file | ->` reads the caller's verdicts, an
  `ekr.fact-judgements/1` document, opens no store, and prints the `ekr.fact-quality/1` document:
  passed, failed, the pass rate and its Wilson score interval at `--confidence` basis points
  (9500 by default), computed with only exactly rounded IEEE 754 operations so every host prints
  the same numbers. It prints the library's bytes as they are, on one line, and `ekr session`
  embeds them unparsed; the workspace's `serde_json` reads every JSON number with
  `float_roundtrip`, so no parse rounds one to a neighbouring binary64. A size outside 1–1000 or a confidence outside 1–9999 is refused as
  `ekr.views.LimitExceeded`, an assertion judged twice as `ekr.views.JudgedTwice`. The runtime
  judges nothing. Both verbs are `ekr session` verbs. Specified as `ekr.views.DrawFactSample` and
  `ekr.views.ReportFactQuality` in `systems/ekr/domains/views.yaml`, with `ekr_views::draw_sample`
  and `ekr_views::report_fact_quality` in the library.

### Changed

- **`ekr explain` answers by reference, in `ekr.explanation/2`** (`docs/cli.md`, `ekr explain`).
  The document gains `format`; a `Proposal` link names its proposal record by `record_hash` and
  carries only the document's operations about the assertion; a `Commit` link, and a `Lifecycle`
  link's new `commit`, name the receipt and the proposal and validation records by hash instead of
  embedding the receipt; an `Evidence` link no longer carries `payload` and `text`. The chain —
  which links, in which order, naming which records — is unchanged. `ekr explain --documents`
  (MCP `explain` with `documents: true`, SDK `Reader::explain_documents`) adds the whole records:
  the proposal record as `record`, each commit receipt as `receipt`, and each evidence link's
  `payload` and `text`. The kernel looks the chain up from what the verified graph records of
  each assertion (the instant of the commit that added it, the revision of the one that retracted
  or superseded it) and parses only those commits' documents, where it parsed every committed
  document per call. At a consumer's 1× shape (4,127 nodes, 67k assertions, 57 commits; SQLite)
  an answer is 23–64 KB through the session and one-shot (3.2–7.9 MB before) and 57–118 KB
  through MCP (6.3–15.8 MB before). **The 0.5 s per-call bound is not met.** A call costs about
  0.52–0.56 s CPU through the session and 0.44–0.65 s through MCP (28–90 s wall under load
  before), and about 4 s one-shot. Most of the session and MCP cost is capturing the head for
  the read, and most of the one-shot cost is opening the store, where every retained blob's
  SHA-256 is checked again; neither is explain's own work, and the open is a task of its own.
  Explain's own work, with the store open, is reported by the 1× bench in
  `crates/ekr/tests/explain_by_reference.rs`. Explain trusts the verified graph and reads only the
  documents on the assertion's own chain; a record of a capture on no chain is not read
  (`kernel.yaml`, "What explain trusts"). Two records claiming one revision are refused as
  `origin-ambiguous`. Specified as `ekr.kernel.ExplanationResult` in
  `systems/ekr/domains/kernel.yaml`. SDK: `Explanation.format`, and `ExplainedEvidence.payload` is
  now an `Option`.

- **One rule decides which node types are events** (`docs/cli.md`, Roles and `ekr ocel`).
  `GET /roles` of `ekr view` now marks `event` or `observation` exactly the types the overview,
  the timeline and `ekr ocel` treat as events, and `observation` exactly the overview's
  observation type, the one the viewer lays out as the observation. A node type now counts as an
  event type only by the timeline's rule (`ekr.views.TypeTiming`): when at least 60% of its judged
  nodes carry the timeline's time at one moment — a timestamp-like value, or dated facts within
  one hour. Before, `/roles` used its own structural rule — a type with any valid-time assertion
  that pointed at another type — so one store could get different event types from the two.
  Visible change: a store whose types qualified only under the structural rule, for example one
  whose nodes carry one dated fact each, now has no event type, and its types lose their `event`
  and `observation` roles in `/roles`; `subject` still reads the edge types' source and target
  types. Roles move both ways across revisions: a revision that adds a dated fact can make a type
  an event type or stop it being one. `/roles` is now derived in `ekr-views`
  (`ekr_views::Index::view_roles_document`, with `Index::event_types` the one rule), specified in
  `systems/ekr/domains/views.yaml`.

### Fixed

- **Queuing a node drops cached answers that share its aliases** (`ekr-sdk`, `Resolver`). When
  `resolve` or `resolve_with` queues a node for a `ProposeNew` answer, it now drops every other
  cached key of the same type that shares an alias with that node, the rule `observe` already
  applied to a committed `CreateNode`. Before, a key cached earlier (`["Ada", "X"]` resolved to the
  node holding `X`) went on answering that node after a node holding `Ada` was queued, where
  `ekr resolve` against the flushed store answers `Ambiguous`, so a consumer could assert a fact
  about the wrong node. Keys of other types, and keys sharing no alias with the queued node, stay
  cached.
- **An `ekr-sdk` protocol error says what the process printed** (`docs/sdk.md`, "Failure, the
  latch and cancellation"). When `ekr session` or a one-shot `ekr` answered with something that is
  not JSON, `TransportError::Protocol` named the verb, the parse error and the stderr tail, but
  not the answer, so the message read "(expected ident at line 1 column 2); stderr tail: (empty)"
  and nothing more. It now carries `answer`: the start of what the process printed, raw and
  without its final line end, at most 400 of the bytes printed (`ANSWER_BYTES`), cut back to a
  character boundary and ended with ` [cut]` (`ANSWER_CUT`) when it was longer. The message shows
  it escaped as a string literal, and so does the `Latched` error a session failed this way
  returns for every later call. Code that builds a `Protocol` value, rather than matching one with
  `..`, names the new field.

- **A subtype that redeclares a property renders, and each type shows its own definition**
  (`ekr view`, `ekr.graph-projection/1` and the overview's `ontology`). A revision in which two
  types declare one property id with another name or value kind — a child type redeclaring an
  inherited property, which a `ModifyProperty` can produce — was refused as inconsistent by the
  projection, the overview and so the viewer. `ontology.properties` now lists one entry per
  distinct definition of an id; where an id has more than one, each entry carries `owners`, every
  type that has that definition, whether it declares it or inherits it, so a reader finds any
  type's definition by lookup; the entries of one id ascend by their first owner. The viewer names
  a property, and its value kind, as the type of the node, edge or chip shown has it, and shows
  the id where no such type is known; the schema history names a version's added property as that
  version named it. A revision whose types define each property alike renders the same bytes as
  before, with no `owners`.

- **A held evidence payload is not read again while its stream has not moved**
  (`story:commit-cost-flat-with-store-size`). A store handle served a verified object from its
  memo only when its class was `Canonical`; every other class had its stream read again on every
  history load, because another handle may raise it. Evidence payloads are `Provenance`, so each
  `propose`, `validate` and `commit` read one object stream per evidence payload in the store,
  and a transaction cost more the more evidence the store held. A raise appends an event to the
  object's own stream, so a handle now looks through the tenant log from the position it last
  looked through — one provider read when nothing was appended — and reads again only the held
  objects whose streams have an event there. A class raised by another handle is still seen on
  the next load. Refusals are unchanged except one: a SQLite handle that already holds an evidence
  payload whose stream event is then redacted in place (eventlog `redact`, which nothing in the
  runtime calls yet) serves the payload until it next reads that stream, where it refused before
  (`task:held-bytes-notice-deleted-blobs`); a fresh handle, and a file-provider handle, refuse
  as before. `ekr_store::stream_reads` counts object-stream and log reads
  (`crates/ekr-store/tests/eventlog_object_memo.rs`), and `crates/ekr-sdk/tests/commit_scaling.rs`
  is an ignored release harness that times each transaction of a 10,000- and an 80,000-fact delta
  over one base store.

## [0.0.24] — 2026-09-30

A store exports as an OCEL 2.0 event log; the viewer's compact mode works from the keyboard and in
narrow windows; a validate command builds one candidate view.

### Added

- **`ekr ocel` exports a store as an OCEL 2.0 event log** (`docs/cli.md`, `ekr ocel`): the
  `ekr.ocel/1` document of the head or of `--revision N`, a one-shot verb and an `ekr session`
  verb, whose `ocel` member is an object-centric event log in the OCEL 2.0 JSON format. The event
  types are the viewer's, by the valid-time rule its overview and timeline use, or the node types
  `--events <type name>...` names; an unknown name is refused as `ekr.views.EventTypeNotFound`.
  Each node of an event type is an event at its timeline time, and one with no time is left out;
  every other node type is an object type; edges are relationships qualified by their type id, and
  properties are attributes. Types, attributes and qualifiers are named by id, and the document's
  `names` gives each id its name; an attribute typed `time` holds a time, and a `Timestamp` outside
  the years 0000–9999 is left out. Two reads of one request print the same bytes. Specified as
  `ekr.views.ExportOcel` in `systems/ekr/domains/views.yaml`, with `ekr_views::export_ocel` in the
  library.

### Changed

- **A narrow window opens the viewer compact** (`docs/cli.md`, `ekr view`). Below 970 px — the
  two sidebars, 290 and 360 px, and 320 px of graph — the page opens with both sidebars collapsed,
  where the graph had no width and the right sidebar ran off-screen. An address with `compact`
  holds as before; `compact=0` shows both sidebars and is written while they are shown in a
  narrow window, so a reload keeps the reader's choice. Only opening the page applies the narrow
  rule: back and forward to an address without `compact` show both sidebars. Wider windows open
  as before.
- **A detail chosen behind the collapsed right sidebar marks its strip** (`ekr view`). The
  sidebar stays collapsed; its strip shows a dot, and its title and accessible name name the node
  (or edge, or path) now in the panel. A click on the strip restores the sidebar showing it and
  clears the mark. The same detail drawn again (back to the node shown, the entities toggle) is
  not new and marks nothing.
- **Type chips work from the keyboard** (`ekr view`). Each chip is a focusable button with
  `aria-pressed` saying whether its type is shown: Enter or Space hides or shows the type, and
  Shift+Enter or Shift+Space shows only that type, or every type again, as click and shift+click
  do; a held key toggles once, and the focus stays on the chip when the chips are drawn again.
  The Compact button carries `aria-pressed`, and the collapse
  tabs and strips are named in words ("Collapse the details", "Show the controls") rather than by
  their arrow.
- **A validate command against a revision the session holds builds one candidate view.** The
  replay that admits a validation or a rejection, and each retry of it after an unrelated append,
  reads the verdict the kernel reached when it decided the command, keyed by the document, the
  basis revision's graph and root, the profile and the validator, instead of running the
  validation pipeline again. The index of canonical state's assertions about edges is kept with
  each revision's graph, so a session builds it once per revision it holds rather than once per
  validation. `validate --against` a revision whose graph the session has released still rebuilds
  that graph by replaying the history to it, which revalidates every retained validation on the
  way: a candidate view per retained validation up to that revision (nine in all for one
  command against revision 7 of an eight-commit session). Verdicts, refusals, their messages and
  their order, and roots are unchanged.

### Fixed

- **The schema lineage lists a version that reuses another kind's id** (`ekr view`, the
  overview's `schema.versions`). A version that declared a property under an id the parent used
  for a type, or a type under a property's id, listed nothing it added, and the viewer called the
  version empty. A lineage member is now its kind and id together, ordered by id, then kind; a
  store without such ids reads the same bytes as before.

## [0.0.23] — 2026-09-30

The viewer collapses its sidebars and solos a type; the schema lineage shows every schema change;
the changes list added evidence; read verbs answer on a store the caller may not write.

### Added

- **The viewer's sidebars collapse to their edges** (`docs/cli.md`, `ekr view`). The Compact
  button, or the key C, collapses both sidebars to a 20-pixel strip at their edge, and the graph
  takes the freed width in 2D and 3D with its camera and chosen node kept; pressed again, it shows
  both. A tab at each edge of the graph collapses that sidebar alone, and a click on a strip
  restores its own sidebar. The state is part of the address (`compact=1`, `compact=left` or
  `compact=right`), so a reload, a shared link and the browser's back button keep it; the page
  opens with both sidebars shown.
- **A shift+click on a type chip shows only that type** (`ekr view`). A click still hides or
  shows one type; a shift+click on the type that is already the only one shown shows every type
  again. The chip's title names both gestures.
- **The changes list evidence added after the seed** (`docs/cli.md`, `ekr view`, "changes
  since"). `ekr.graph-changes/1` gains the change kind `EvidenceAdded`: every evidence entry an
  `!AddEvidence` brought, once, as a change of the revision that committed it, with its id, its
  `locator` (the statement's source identity) and the `content_hash` of its payload, ordered
  after the revision's assertion changes. `GET /changes`, the MCP tool `changes_since` and the
  session verb `changes` answer it; `ekr.views.ChangesListed` counts it as `evidence_added`. The
  seed's own evidence is no change, a valid-time since never chooses one, and a range holding no
  `AddEvidence` answers the same bytes as before.
- **The SDK's reads tolerate a kind a newer `ekr` adds** (`docs/sdk.md`, Typed reads). Every
  closed set of kinds in `ekr_sdk::read` — `ChangeKind`, `MatchTier`, `MatchField`,
  `TransactionState`, `ViewValue`, `OntologyValueType` and `ExplanationLink`, as `CodeNameKind`
  already did — ends in `Other`: an unknown kind reads as `Other` and the rest of the document
  reads as before, where it failed the whole read. `Ontology`, `Snapshot` and `Explanation` now
  hold the read side's own `OntologyCardinality`, `SnapshotSubject` and `SnapshotPredicate`, each
  with `Other`, instead of the document builders' `Cardinality`, `Subject` and `Predicate`. `ChangeKind::EvidenceAdded` is modelled, with
  `GraphChange::locator` and `GraphChange::content_hash`.

### Changed

- **The schema lineage lists widened edge ends and modified properties.** In
  `ekr.graph-overview/1`, each version of `schema.versions` also carries `widened` (each edge-type
  end that gained node types against its parent: `edge_type`, `side`, `node_types`) and `modified`
  (each property declaration that differs from its owner's in the parent: `owner`, `property`,
  `name`, and what `changed` — `Name`, `ValueType`, `Cardinality`, `Required`, `Constraints`, or
  `Declared` for a property an owner newly declares). Both are omitted when empty, so a store
  without a `WidenEdgeType` or `ModifyProperty` of an existing property answers the same bytes as
  before. `added` and `removed` are unchanged. The `ekr view` schema history shows both, and calls
  a version empty only when its ontology is its parent's; a version made only by `WidenEdgeType`
  no longer reads "adds no types or properties". `ekr_sdk::read::SchemaVersionChange` gains
  `widened` (`WidenedEnd`) and `modified` (`ModifiedProperty`).

### Fixed

- **The read verbs answer on a store this process may not write** (`docs/cli.md`,
  Configuration). `ekr head`, `ontology`, `snapshot`, `code-names` and every other verb that only
  reads — and `ekr session`, `view`, `mcp` and `migrate`'s source — exited 1 with `the store is
  unavailable: … Permission denied` on a store mounted read-only or owned by another user, on both
  providers. Such a store now opens read-only: a file store is read through a private copy taken
  under a shared lock, a SQLite database through a read-only connection into memory, and nothing
  is written at the store's path, not even a `-wal` or `-shm` beside a SQLite database. The
  answers are the bytes a writable store gives. A session, `ekr view` and `ekr mcp` read the store
  again once its files change, so a commit another process makes is what the next request reads;
  `ekr view` and `ekr mcp` remove the private copy when sent SIGTERM, SIGINT or SIGHUP, and a copy
  a killed process left is removed by the next read-only open.
- **A verb that writes such a store is refused by name.** `ekr seed`, `propose`, `validate` and
  `commit`, and those requests in a session, are the refusal `store-read-only` (exit 2) before
  anything is opened, instead of a fault. Which verbs write is the verb table's `store` column.

## [0.0.22] — 2026-09-30

The SDK types the store checks and carries evidence with the assertions that cite it; a commit
applies the head graph once and a validation builds one candidate view.

### Added

- **The SDK types the store checks** (`docs/sdk.md`, Typed reads). `ekr_sdk::read::Reader` and
  `OneShotReader` gain `quality(revision)`, `rejections(from, to)` and `code_names(files, at)`,
  which return `StoreQuality` (`ekr.store-quality/1`), `Rejections` (`ekr.rejections/1`) and
  `CodeNames` (`ekr.code-names/1`) through a session or as one-shot `ekr` processes. Each value
  writes back exactly the document `ekr` printed and ignores a field a newer `ekr` adds.
- **`ekr-sdk` commits a consumer's evidence with the assertions that cite it** (`docs/sdk.md`,
  "Batches" and "Evidence items"). An `EvidenceItem` is a source identity, an observed-at time
  and the exact bytes. `EvidenceSet::cite` hashes the bytes as `ekr hash` does and mints the
  evidence id as `ekr mint evidence` does, with no request; the same item cited again through
  one set gets the same id. `EvidenceSet::from_store` rebuilds a set from the entries a store
  holds, so a consumer that restarts adds no second entry for an item.
  `Batcher::commit_with_evidence` puts each entry's `!AddEvidence` into the group of the first
  assertion citing it in every transaction it proposes, until one commits it; later groups cite
  the existing id. Bisection rebuilds each half the same way, so an assertion is never submitted
  without the evidence it introduces, an entry that only rejected groups cite is never
  committed, and a batch an entry moved into is split rather than proposed past the operation
  limit. `Batcher::commit` and its report are unchanged.

### Changed

- **A commit clones and applies the head graph once.** The replay that admits a commit takes the
  graph the kernel applied when it decided the commit, keyed by the same inputs as the root it
  already reused, instead of cloning the head graph and applying the transaction again. Roots and
  receipts are unchanged.
- **A validation builds its candidate view once.** The node and edge index of canonical state with
  the operation set applied is built once per validation and read by every validator, where five
  validators built one each; canonical state's assertions about edges are indexed by edge once.
  Refusals, their messages and their order are unchanged.
- **The bench targets compile in the gate.** `command_bench` builds again, and `task test`
  compiles `cargo test -p ekr-kernel --features bench --no-run`.

## [0.0.21] — 2026-09-30

A store's quality, its rejections and the store names a consumer's code quotes are reads; the SDK's
`close()` reports why a session failed at its end.

### Added

- **`ekr code-names <file>...` reports the store's names a consumer's code quotes**
  (`docs/cli.md`, `ekr code-names`). Every literal in the given source files — text between two
  `"`, `'` or backtick quotes on one line — that equals a node or edge type's name, a property's
  name, or a node's canonical name or alias is listed in one `ekr.code-names/1` document with its
  file, line, column and every name it equals. A name that is also the runtime's own vocabulary
  (the field names, enum variants and union tags of its specification) is reported flagged
  `runtime_word` and counted in `meta.runtime_word_findings`; the text of a store id is never
  reported. The verb reads only, exits 0 whatever it finds and puts the count in `meta.findings`;
  `ekr session` serves it, and `ekr-views` exposes it as `find_code_names`.
- **`ekr quality` reports a store's quality beyond its size** (`docs/cli.md`, `ekr quality`):
  the `ekr.store-quality/1` document of the head or of `--revision N`, a one-shot verb and an
  `ekr session` verb. It counts the active assertions with evidence and with evidence an
  `AddEvidence` added after the seed, the property declarations that declare a constraint, and
  every name two or more nodes of one type share, listed with the type and the nodes. Each
  figure comes with its count and, where it has a whole, a share in basis points. It reads one
  revision only, so two reads of that revision print the same bytes. Refused transactions and
  open ambiguities are not part of it. Specified as `ekr.views.ReportStoreQuality` in
  `systems/ekr/domains/views.yaml`, with `ekr_views::report_quality` in the library.
- **`ekr rejections` reads the validation findings of rejected transactions** (`docs/cli.md`,
  `ekr rejections`). It prints one `ekr.rejections/1` document: each rejected transaction with
  the issues its rejection recorded (validator, code, message), keyed on the revision it was
  validated against, for a range `--from N --to M` of those revisions. Committed transactions
  carry no findings and never appear. Two reads of one range print the same bytes, before and
  after a later commit. `ekr session` serves it too. The document and its view
  `ekr.kernel.Rejections` are specified in `systems/ekr/domains/kernel.yaml`, and a kernel
  conformance scenario holds it on both providers.

### Changed

- **Breaking: `ProcessSession::close()` returns `Result<(), TransportError>`**, no longer
  `Result<ExitStatus, TransportError>` (`docs/sdk.md`, "Failure, the latch and cancellation").
  A child that exits with a status other than 0, or is killed when it has not exited within the
  timeout, is the new `TransportError::CloseFailed { status, killed, stderr_tail }`, carrying the
  last 4096 bytes the child wrote to stderr, such as why it could not write its replay
  checkpoint. A cancelled session closes as `Cancelled` and a failed one as `Latched`. A caller
  that wrote `session.close()?.success()` writes `session.close()?`.

## [0.0.20] — 2026-09-29

The SDK resolves through a cache, commits in bisected batches and types the views reads, which
`ekr session` now serves; it retries starting a binary that is busy being written.

### Added

- **`ekr-sdk` resolves through a cache and commits in bisected batches** (`docs/sdk.md`,
  "Resolve before you create" and "Batches").
  - `Resolver` caches `ekr resolve` answers by exact type id and sorted aliases, so a fixture
    that names 31 distinct references in 480 resolves sends 31 resolve requests. A `ProposeNew`
    answer mints an id and queues a `CreateNode` carrying the aliases. `flush` commits the queue,
    and a resolve that shares an alias with a queued node flushes it first. `invalidate` works
    per alias, and `observe` records the consumer's own commits. `Ambiguous` is returned as a
    value.
  - Each flush reads `ekr head`. A commit the SDK did not make drops the cache, and the queued
    references are resolved again, so a node another process committed meanwhile replaces the
    queued one (`Flushed::replaced`) instead of a duplicate or an `alias-already-exists`
    rejection.
  - `Batcher` packs atomic dependency groups into batches under the 10,000-operation and 8 MiB
    caps, retries a `Stale` commit under a newly minted transaction id, and bisects a rejected
    batch down to the single group. Its `BatchReport` lists every committed transaction with its
    id, revision, batch and groups, and every rejected operation with its issues, batch and
    group. With one invalid operation planted among 2,000, that operation is the only rejection
    and the other 1,799 groups commit.
  - A refusal of the transaction as a whole, such as a proposer that is not the host operator,
    stops the run after one proposal instead of bisecting it. `BatchReport::refused` names that
    refusal once, for every group not committed.
  - A `commit` sent without an answered outcome, such as a lost reply, is named by
    `BatchError::outcome_unknown` with its id, batch and groups. `docs/sdk.md` says how to settle
    it from `ekr transactions`. The resolver keeps such nodes queued, and recognises its own
    landed commit on the next flush.
- **`ekr session` serves the `ekr.views` reads** (`docs/cli.md`, § `ekr session`, "The
  `ekr.views` reads"). Six session verbs, `overview`, `search`, `describe`, `expand`, `timeline`
  and `changes`, answer the document that `ekr view` serves on the matching route for the same
  query and revision. They refuse what that route refuses, with the same name and message, as
  `"exit": 2`. The session loads each revision's index once, keeps the three used most recently,
  and drops them when it follows a replaced store. A reader no longer has to fetch a whole
  snapshot to count or find things. `ekr` has no one-shot verb of these names.
- **Typed reads in `ekr-sdk`** (`docs/sdk.md`, "Typed reads"). `read::Reader` works over any
  transport and returns `Overview`, `NodeMatches`, `NodeDetail`, `Timeline` and `Changes`. Its
  `ExpandPages` iterator reads a neighbourhood page by page, pinned to the first page's
  revision. It also returns `Head`, `Snapshot`, `Ontology`, `Transactions` and `Explanation`,
  which `OneShotReader` reads with no session open, one `ekr` process per read. A read that
  answers no value is a `ReadError` naming the verb and, for a refusal, its code. Every value
  writes back exactly the document it was read from. `crates/ekr-sdk/tests/read.rs` checks this
  against every document the `ekr-views` conformance fixtures render, so a field added to a views
  format without an SDK update fails there.

### Fixed

- **`ekr-sdk` retries starting a binary that is busy being written.** Linux refuses to execute a
  file that any process holds open for writing (`ETXTBSY`), and in a multithreaded consumer a
  child forked by another thread holds a freshly written binary's descriptor until it execs. The
  `--version` and `operations` probes, `ProcessSession::start`, one-shot requests and
  `Viewer::spawn` now retry that refusal for up to 630 ms; any other start error is still
  returned at once, and the refusal after the last retry is the same `BinaryError::Run`,
  `TransportError::Io` or `ViewerError::Spawn` as before. The wait counts against the probe
  timeout and a one-shot request's timeout (`TimedOut`), and a cancel ends a one-shot's wait as
  `Cancelled`.
- **`docs/sdk.md` covers the document builders** ("Documents"), which the 0.0.19 `README.md`
  said it did: `TransactionBuilder`, `OntologySpec::seed` and `SeedBuilder`, local minting and
  hashing, the ten transaction limits and `DocumentError::Limit`, and `OntologySpec` with
  `Ontology::ensure`, including what it emits, what it refuses by name and profile v1.
  `crates/ekr-sdk/tests/docs_examples.rs` holds each Rust block of the section to code it
  compiles and runs, three of them on a real `ekr session`, and every API name the section uses
  to code the compiler checks.

## [0.0.19] — 2026-09-29

A Rust SDK drives a store through one child `ekr session`; nodes gain aliases; `ekr mcp` serves
the head, and long-running readers follow a store replaced at their path; the 3D view draws a
frame in 2 calls instead of 25,386.

### Added

- **The `ekr-sdk` crate: a consumer's client for an EKR store** (`docs/sdk.md`). It runs one child
  `ekr session --create` and exchanges one JSON line per request, so a consumer links neither the
  kernel nor the store; a test in `crates/ekr/tests/story_contract.rs` refuses a kernel, store or
  graph dependency, and the API is blocking with no async runtime.
  - `EkrBinary::open(path)` takes an explicit path only (a bare name is refused, `PATH` is never
    searched), runs `--version` in an empty environment, refuses a binary below 0.0.14 as
    `TooOld`, and probes `operations`; probes time out after 5 s.
  - `ProcessSession` passes the child exactly `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND` (or an
    exact list), times out a request after 300 s, kills the child on any failure and refuses every
    later call. A request over the 25,231,360-byte line cap runs inside the session through a
    private temporary file (0600) that is deleted after the reply.
  - `Reply::answer()` types a reply as an outcome (`Validated`, `Rejected`, `Committed`, `Stale`),
    a refusal with its code, a usage message or a fault. `CancelHandle` is safe to set from a
    signal handler and ends the child within 20 ms. `Viewer::spawn` starts `ekr view` and reads
    its URL. `RecordingTransport` and `ReplayTransport` (`ekr-sdk.recording/1`) test a consumer
    without an `ekr` binary.
  - Builders for `ekr.transaction-document/2` (all 15 operation kinds), `ekr-seed/2` and typed
    references; `build()` derives the evidence list, refuses an empty transaction and a
    schema/data mix, and checks the kernel's ten transaction limits under the kernel's names.
    Identities are minted and payloads hashed locally with `ekr-core`'s functions.
  - `OntologySpec` names types, properties and edge ends; `Ontology::ensure` compares it with a
    store's `ekr ontology` and emits only the missing `DefineNodeType`, `DefineEdgeType`,
    `ModifyProperty` and `WidenEdgeType`, and refuses by name what no schema operation can change.
    A drift test reads every builder's output with the kernel's reader and `ekr schema`.
- **`AddAlias`: a transaction gives a node that exists one more alias.** `!AddAlias {node,
  alias}`, operation 14, is data rather than a schema change and is applied under every
  validation profile. The alias is appended to the node's `aliases`, so `ekr resolve` answers the
  node for it from the next revision on; nothing removes an alias. Refused by name: an alias the
  node or another node of its type already holds (`alias-already-exists`), one alias given twice
  for a type in one transaction (`duplicate-alias`), the empty alias (`empty-alias`, new) and a
  node that does not exist (`unresolved-node`). A consumer's import keys nodes by an alias and
  could not give that key to a node created earlier without it. `ekr operations`, `ekr guide` and
  `docs/cli.md` describe it; four new kernel conformance scenarios, 53 in all, pass on both
  providers.
- **`ekr mcp` has a `head` tool.** It takes no arguments and answers
  `{"format":"ekr.view-head/1","head":N}` as `GET /head` does; an argument is error -32602.

### Changed

- **`ekr mcp`, `ekr view` and `ekr session` follow a store replaced at their path.** Before each
  request that reads the store they compare its identity at the configured path (device and inode
  of the file store root or the SQLite file) with the one they opened; on a change they reopen at
  the path and answer from the new store. A read through a file-store handle that fails as
  diverged reopens once and retries, which covers a replacement under the same inode. Refused by
  name: `store-replaced` when the store at the path does not open (session exit 1, MCP tool error,
  view 503) and `store-replaced-proposals-open` (exit 2, naming the transactions) while a session
  still has proposals open. Without a replacement a request costs one identity check.
- **The 3D view of `ekr view` draws in batches and rests when idle.** Nodes are one instanced mesh
  per glyph, edges one line-segments buffer, arrows and particles one instanced mesh each;
  3d-force-graph lays the graph out and builds no object per node or edge. The render loop pauses
  once the layout has stopped and no control, drag, camera flight or particle moves, and the next
  interaction wakes it. A generated store of 4,127 nodes with 3,490 nodes and 12,083 edges drawn,
  headless Brave on a GPU: 25,386 → 2 draw calls per frame while the camera moves, 133.4 → 16.7 ms
  median frame, 0 calls while idle. Hover, click, double-click, node drag and, up to 5,000 edges,
  edge hover and click work as before; arrows keep the same 5,000-edge threshold.
- **The hops slider streams the neighbourhood it names.** Setting it to N (1 or 2, the most
  `/expand` answers) on a neighbourhood focus streams `/expand` with `depth=N` from the focus, once
  per focus and depth once its read has succeeded, and the focus is walked again when the stream
  ends; an address carrying `hops` does the same. With no focus, the crumb bar says how to get one.

### Removed

- **`ekr view` no longer serves the earlier viewer page at `GET /alt`.** The streamed page at `/`
  is the only page the binary embeds; `/alt` answers 404 like any other unknown path, and the
  binary is 65,297 bytes of HTML lighter. `/projection` and `/roles` are still served.

## [0.0.18] — 2026-09-29

A commit hashes once; checkpoints fall due by size; a session holds one graph; edge types widen.
At a consumer's size (4,127 nodes, 57 commits, SQLite) a 1,561-operation batch takes 0.94 s (0.0.17:
1.64 s; 0.0.15: 3.5 s), commit 1,186 → 362 ms.

### Added

- **`WidenEdgeType`: a schema change adds node types to an edge type's ends.** Under validation
  profiles v2 and v3, `!WidenEdgeType {edge_type, source_types, target_types}` writes both ends
  whole, each keeping every type it has; it is a schema-only transaction naming its
  `schema_version`, and commits as the next schema version with the prior one as its parent. Edges
  the type already holds stay valid, and the next transaction may create edges to the added types.
  Refused by name: `unknown-edge-type`, `unknown-endpoint-type`, `edge-endpoint-removed` (an end
  never shrinks) and `schema-change-without-effect`; two widenings of one edge type in one
  transaction are `conflicting-write`; profile v1 refuses it as `unsupported-operation`. A
  consumer's delta ingest had skipped 1,768 of 2,312 facts for want of it. `ekr operations`,
  `ekr example schema-change`, `ekr guide`, `docs/cli.md` and `docs/schema-evolution.md` describe
  it; design § 101; five new kernel conformance scenarios, 49 in all, pass on both providers.

### Changed

- **A checkpoint is due by commits and document bytes, not by operations** (design § 99.5). A commit
  writes a replay checkpoint when it is the fifth past the retained one
  (`REPLAY_CHECKPOINT_COMMITS` = 5) or when the transaction documents committed since hold 16 MiB
  (`REPLAY_CHECKPOINT_BYTES`, which replaces `REPLAY_CHECKPOINT_OPERATIONS`). A consumer batch of
  1,561 operations no longer writes a checkpoint every time: 57 batches in one session wrote 13
  checkpoints instead of 58.
- **`ekr session` leaves a checkpoint of its head when its input ends**, if that head is past the
  last checkpoint (`Runtime::retain_checkpoint_at_rest`), so the next verb replays nothing the
  session committed. A cold `ekr transactions` on a 57-batch store under profile v3: 5.07 → 2.33 s
  of CPU (medians).
- **New commit receipts name the identities they created.** `ekr.commit-receipt/3` is `/2` with
  `created`, the node and edge ids of the transaction's `CreateNode` and `CreateEdge` operations
  (`CommitReceiptV1::created`, `CreatedIdentitiesV1`); `/1` and `/2` receipts are read as before and
  keep their bytes. Under validation profile v3 an open that admits a checkpoint reads them from the
  receipt instead of parsing every committed proposal. A `/3` receipt whose list is not what its
  transaction creates is refused on replay as `commit-created-identities`. A session that comes to
  rest does not write its checkpoint when the store already holds a newer one.
- **A commit hashes the graph once, as a stream.** The verify step reuses the root the kernel
  computed for the decision instead of hashing the whole graph again, and `ContentHash::of` hashes
  as it encodes, holding at most a 64 KiB window. Roots are byte-identical to 0.0.17's. A hashing
  encoder refuses `Encoder::as_bytes` and `Encoder::finish` by name.
- **A session keeps the head graph, not one graph per revision.** An older revision's graph is
  rebuilt by a verified replay when a read or `validate --against` asks for it, and released when
  that read returns. At 3× a consumer's size, a session's memory after 20 commits is 1.24× its
  memory after one (was 2.39×; peak 4.3 → 2.2 GB); `validate --against` an early revision on a
  20-batch store 8.5 → 2.8 s.

## [0.0.17] — 2026-09-29

Reads share state instead of copying it; requests copy no retained bytes; validation scans nothing
whole. Measured at a consumer's size (4,127 nodes, 57 commits, SQLite), CPU median inside
`ekr session`: `resolve` 145.2 → 3.05 ms (file 90.7 → 1.31 ms); a 1,561-operation batch 3.5 → 1.6 s.

### Changed

- **Read verbs share the verified state instead of copying it.** `VerifiedRead` holds the kernel's
  own graph, transaction records and seed input (`graph: Arc<CanonicalGraph>`,
  `transactions: Arc<BTreeMap<..>>`, `seed_input: Arc<SeedDocument>`); `Runtime::transactions`
  returns `Arc<BTreeMap<..>>`; `SchemaHistory::graph` and `ekr_views::LoadedRevision::graph` are
  `Arc<CanonicalGraph>`. A read of an unchanged head copies none of them. Every answer is
  byte-identical.
- **`resolve` is a lookup.** `ekr_graph::AliasIndex` holds a graph's nodes by type and alias; the
  kernel builds it once per head (`VerifiedRead::aliases`) and `ekr resolve` and the `resolve` MCP
  tool answer through `ekr_integrate::resolve_indexed`, which answers exactly what
  `ekr_integrate::resolve` answers.
- **A request copies no retained bytes.** `RetainedObject.bytes` is `Arc<Vec<u8>>`, shared from the
  bytes a handle verified; the seed input is assembled once per runtime and hands out the store's
  own allocations (`SeedDocument.evidence_payloads` values are `Arc<Vec<u8>>`), and every read still
  re-checks each held payload. Per-request cost no longer grows with seed evidence: `resolve` on a
  store with 44 MB of evidence 96.4 → 2.0 ms (`/2`, file), 107.2 → 4.9 ms (`/2`, SQLite).
- **Validation without whole-graph scans.** Edge cardinality counts and the alias check use indexes
  built once per validation; refusals, codes, messages and their order are unchanged (a
  differential over 3,000 generated transactions against 0.0.16). An edge-heavy batch at 10× a
  consumer's size validates in 88 ms in-process instead of 3.18 s.

### Known limits

- A session `validate` at 10× still pays about 0.7–1 s per call outside the validators, and
  `explain` (about 3 s at 1×) and `snapshot` (about 0.7 s) still rebuild what they answer
  (`task:perf-audit-2026-09-29-remaining`).
- A store handle keeps answering from verified bytes after their blob is deleted on disk
  (`task:held-bytes-notice-deleted-blobs`).

## [0.0.16] — 2026-09-29

Evidence after the seed; seed payloads out of the envelope; `ekr migrate`.

### Added

- **Evidence after the seed.** `AddEvidence` (operation 12) in `ekr.transaction-document/1` and
  `/2` carries one `ekr.graph.Evidence` and its payload bytes; an assertion in the same or a later
  transaction may cite it, and the commit retains the payload as a Provenance object. Refusals: a
  payload that does not hash to its entry, a reused evidence id (`identity-already-exists`,
  `duplicate-identity`), an assertion citing evidence no revision holds (`unresolved-evidence`),
  evidence other than a human statement (`evidence-unsupported-source`), and an `extracted_by` that
  is not the submitter. A payload is at most 16,384 bytes in `/2` and 4,096 in `/1` (the
  `sequence_elements` limit); a larger statement goes in the seed with `ekr seed --evidence`. The
  kernel conformance suite holds 44 scenarios (4 new).
- `ekr migrate --to <path>` (design § 100.3): writes a copy of the store at a new path whose seed is
  `ekr-seed-envelope/3`, re-publishing every decision with its original identities and times,
  carrying every other object and converting a legacy inline `ObjectStored` object into metadata
  and a blob; the new store is replayed in full against the old one, which is left untouched, and
  the `ekr.store-migration/1` report maps each replaced record. A destination that is, is inside or
  contains the source is refused (`migrate-destination-is-source`,
  `migrate-destination-inside-source`, `migrate-destination-contains-source`); a store whose
  migration did not finish refuses every read as `migrate-incomplete`.

### Changed

- **New seeds write `ekr-seed-envelope/3`** (design § 100.1): the envelope names each evidence
  payload by its content hash instead of carrying it as a JSON number array, and the seed's
  publication preparation (`ekr.publication-preparation/3`) names it too, binding its bytes once
  under the payload's own address. A seed with 10 MiB of evidence retained a 37,446,229-byte
  envelope and a 63,916,454-byte preparation; it now retains a 5,542-byte envelope and a
  14,481-byte preparation, and each payload once. A `/2` store opens,
  replays and commits as before. A `/3` envelope naming a payload the store does not hold is
  refused `seed-evidence-payload-absent: <hash>`.
- The `ekr.views` reads work out which evidence is retained in one pass over the verified history
  instead of one history read per evidence entry: rendering a store with 400 added evidence
  entries took 13.66 s on SQLite and takes 57.2 ms. The documents are unchanged.

### Known limits

- `AddEvidence` accepts human statements only; evidence from observations waits on
  `decision-blocker:observation-retention-path`.
- `GET /changes` and `changes_since` do not list added evidence yet
  (`task:changes-since-lists-added-evidence`).
- Still open from 0.0.15: no MCP `head` tool, and long-running readers keep a store replaced at
  their path.

## [0.0.15] — 2026-09-29

Agents and pages ask what changed since a revision or a time.

### Added

- `ekr.views` ChangesSince and its format `ekr.graph-changes/1`: every node and edge created and
  every assertion added, superseded or retracted after a revision (`since_revision`), a valid time
  (`since_valid`, assertion changes only) or a transaction time (`since_recorded`), up to `at` or
  the head. Changes are ordered by revision, change kind and id; each carries its revision,
  `recorded_at` and evidence ids, and a created node or edge carries the evidence of the assertions
  its revision added about it. Paged by `limit` (1–2,000, 500 by default) and a cursor; the document
  carries the revision it read and no head.
- `ekr view` serves it as `GET /changes`; `ekr mcp` as the tool `changes_since`, with the same
  argument names. A request naming none or two of the three since inputs is `invalid-query`
  (`-32602` in MCP); a `since_revision` below 0 is `since-malformed`; a since past the head is
  `revision-not-found` naming the head.

### Changed

- The views conformance suite holds 51 scenarios (10 new), passing on the file and SQLite
  providers; the kernel suite was resynthesized for its specification digest only.

### Known limits

- `ekr mcp` has no `head` tool (`task:mcp-serves-the-head`), and a running `ekr mcp`, `ekr view` or
  `ekr session` keeps answering from a store file replaced at its path until restarted
  (`task:readers-reopen-a-replaced-store`).

## [0.0.14] — 2026-09-29

Past revisions keep their view bytes; a session can seed; replay checkpoints are written when due.

### Added

- `ekr view` serves `GET /head`, `{"format":"ekr.view-head/1","head":N}`; the viewer pages read the
  head from it.
- `ekr session` starts when the configured store does not exist: it answers `mint`, `hash` and
  `schema` as the one-shot verbs do, and a store verb answers `store-not-found` inside the
  response while the session keeps serving. `ekr session --create` also serves `seed`, then holds
  the store it created. A small store is built in one process where one-shot calls took nine.

### Changed

- **Breaking for readers of `meta.head`:** the six `ekr.views` formats and their events no longer
  carry `head`, so a render of a past revision is byte-identical before and after later commits
  (views rule 5). The `RevisionNotFound` refusal keeps its `head` field. `ekr mcp` answers the same
  documents. The view caches are keyed on the revision alone.
- Replay checkpoints (design § 99): a seed writes one; a commit writes one only when the handle
  knows of none, the head is 4 revisions past it, the commits since hold 512 operations, or a
  validation since was made against a revision before its head; otherwise it appends a pointer.
  Propose, validate and a stale commit append nothing. Inside `ekr session` on a ~20 MB file store,
  the median commit went from 145 ms to 91 ms; one-shot use is unchanged.
- The kernel conformance scenarios for Propose, Validate and a stale Commit no longer expect
  `ekr.store.CheckpointWritten`.

### Known limits

- The eventlog file provider still re-hashes its log after a write and syncs 28 times per write
  verb (`task:eventlog-rehash-and-fsync-per-write`).
- A session request still costs 20–34 ms on a store seeded with 11 MB of evidence
  (`task:session-request-cost-grows-with-evidence`).

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
