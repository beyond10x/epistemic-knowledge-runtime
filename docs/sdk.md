# The `ekr-sdk` crate

`ekr-sdk` is a Rust library for driving an EKR store from a program. It runs a child
[`ekr session`](cli.md#ekr-session) and exchanges one JSON line per request with it, so a consumer
links neither the kernel nor the store. This page covers how the child is started, how requests
and replies are typed, how failures are handled, building the documents a consumer writes,
resolving references through a cache, committing in batches, typed reads, and how to test without
an `ekr` binary.

The API is blocking and depends on no async runtime in any feature. A Tokio program calls it inside
`spawn_blocking`.

## The binary

The consumer chooses which `ekr` binary to run and passes its path. The SDK never searches `PATH`.

```rust
use ekr_sdk::binary::EkrBinary;

let binary = EkrBinary::open("/opt/ekr/0.0.17/ekr")?;
binary.require_operations(&["CreateNode", "AddAssertion"])?;
```

`EkrBinary::open` runs `ekr --version` with an empty environment and refuses:

| refusal | when |
|---|---|
| `BinaryError::NoPath` | the path is empty |
| `BinaryError::BareName` | the path is a bare name such as `ekr`, which starting a process would look up on `PATH` |
| `BinaryError::TooOld` | the binary's version is below `MINIMUM_VERSION` (0.0.14, the first release whose `ekr session --create` serves `seed`); the message names both versions |
| `BinaryError::Run`, `BinaryError::Unexpected` | the binary does not run, or does not answer `--version` as `ekr <version>` |
| `BinaryError::TimedOut` | the binary has not answered within `PROBE_TIMEOUT` (5 s); it is killed |

`EkrBinary::open_with_minimum(path, minimum)` sets a higher minimum.
`EkrBinary::open_with(path, minimum, probe_timeout)` also sets the time limit, which then applies
to `--version` and to every later `operations` probe. `operations()` lists the operation kinds
that `ekr operations` prints. `require_operations` refuses a binary that lacks any of the given
kinds (`BinaryError::MissingOperations`).

## A session

```rust
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport};

let store = StoreConfig { host: "host.json".into(), store: "./store".into(), backend: Backend::File };
let mut session = ProcessSession::start(&binary, store, SessionOptions::default())?;
let reply = session.request(&Request::new(["seed", "-"]).with_stdin(seed_yaml))?;
```

`ProcessSession::start` runs `ekr --host … --store … --backend … session --create`. The child
serves `seed` as well as every other verb a session serves, whether or not the store exists yet. So
seeding and the first writes go through one process. Ids and hashes need no process: the SDK
computes them itself ([Documents](#documents)).
`processes_started()` reports how many `ekr` processes the session has started.

`SessionOptions` controls how the child runs:

| field | default | meaning |
|---|---|---|
| `environment` | `Environment::Store` | the child's environment. It is cleared and never inherits the consumer's. `Store` sets exactly `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND`. `Exact(list)` sets exactly `list`. The store is also passed as flags, so both forms reach the same store |
| `timeout` | 300 s | the longest one request may take, from writing it to reading its reply |
| `current_dir` | the consumer's | the child's working directory, which relative paths in a request resolve against |

The child is started with an absolute path and holds no `PATH` unless `Exact` gives it one. It
therefore starts when the consumer's own `PATH` is empty.

A binary that Linux refuses to start as busy (`ETXTBSY`: some process, possibly a child another
thread forked, still holds it open for writing) is retried for up to 630 ms before the refusal is
returned. This applies to the `--version` and `operations` probes, the session, one-shot
requests and `Viewer::spawn`. Every other start error is returned at once. The wait counts
against the probe timeout and a one-shot request's `timeout`, which end it as `TimedOut`, and a
cancel ends a one-shot's wait within 20 ms as `Cancelled`.

### Requests over the line cap

A session refuses a request line longer than 25,231,360 bytes (`LINE_CAP`,
`session-request-too-large`). The SDK handles such a request in one of two ways.

- **The argv reads `-`** (`seed -`, `propose -`, `resolve -`, `hash -`) and the request carries
  `stdin`. The SDK writes the text to a new temporary file that only the consumer's user can read
  (mode 0600, under the consumer's temporary directory). It replaces `-` with that file's path and
  sends the request inside the session, then deletes the file after the reply. So a `seed -` over
  the cap still leaves the session holding the store it created. A refusal that names its input
  names the temporary path instead of `-`.
- **Otherwise** (the argv reads no `-`, or even the rewritten line is over the cap), the SDK sends
  the request to a one-shot `ekr <argv>` process, using the same binary, environment, flags and
  working directory, with `stdin` on its standard input. It returns that process's exit status,
  its document and the whole of its stderr as the reply.

Either way, the session stays open and serves the next request.

### Failure, the latch and cancellation

Every failure stops the child and latches the session. After that, the call that failed and every
later call return an error, and the session is never restarted. To continue, start a new session.

| error | when |
|---|---|
| `TransportError::Died` | the child ended before answering. The message names the verb, how the child ended (for example `signal: 9 (SIGKILL)`) and the last 4096 bytes it wrote to stderr (`STDERR_TAIL_BYTES`) |
| `TransportError::TimedOut` | no reply within `timeout`; the child was killed |
| `TransportError::Protocol` | the child printed a line that is not a reply. `answer` holds the start of that line, raw and without its line end: at most 400 of the bytes printed (`ANSWER_BYTES`), cut back to a character boundary and ended with ` [cut]` (`ANSWER_CUT`) when it was longer. Bytes that are not UTF-8 read as U+FFFD. The message shows the answer escaped as a string literal (in quotes, with a quote, a backslash or a control character escaped), beside the verb, what was wrong with it and the stderr tail. The answer is what the process printed, and can include text the process echoed from the request |
| `TransportError::Cancelled` | the session was cancelled (below) |
| `TransportError::Latched` | a call made after any of the above. It names this call's verb and the original failure |

A one-shot process that times out, ends on a signal, or prints something other than JSON fails
only its own call. The error carries the last 4096 bytes of its stderr and, for output that is
not JSON, the start of what it printed in `answer`. The session is not latched.

`cancel_handle()` returns a `CancelHandle`. `cancel()` only stores `true` in an atomic flag, which
is async-signal-safe, so a signal handler can call it. A thread in the session checks the flag
every 20 ms and kills the child. A call in flight fails with `Cancelled`, and so does every later
call. The flag itself is `flag()`, so a program using `signal-hook` needs no handler code of its
own:

```rust
signal_hook::flag::register(signal_hook::consts::SIGTERM, session.cancel_handle().flag())?;
```

`close()` closes the child's input and waits up to `timeout` for it to exit. Before exiting, the
child writes the store's replay checkpoint. It returns `Result<(), TransportError>`:

| result | when |
|---|---|
| `Ok(())` | the child exited 0 |
| `TransportError::CloseFailed { status, killed, stderr_tail }` | the child exited with another status (`killed` is `false`), or was still running at `timeout` and was killed (`killed` is `true`, `status` is the signal). `stderr_tail` is the last 4096 bytes it wrote to stderr (`STDERR_TAIL_BYTES`), cut forward to a character boundary, for example why it could not write the checkpoint |
| `TransportError::Cancelled` | the session was cancelled, before or while closing, whatever its child's status |
| `TransportError::Latched` | an earlier call failed the session, whatever its child's status |

An unclean exit is an error rather than a value beside the status, so `session.close()?`
propagates a checkpoint the child could not write instead of dropping it. Up to 0.0.20, `close()`
returned the bare `ExitStatus` and no stderr.

Dropping a healthy session also closes it, but reports nothing. A cancel while closing kills the
child within 20 ms instead of waiting out the timeout. Dropping a failed or cancelled session kills
the child.

## Replies

A `Reply` is `{exit, document, stderr}`: the one-shot verb's exit status, the JSON document it
printed (`None` for nothing) and its stderr. `reply.answer()` returns an `Answer`:

| `Answer` | exit | contents |
|---|---|---|
| `Outcome(Outcome::Validated / Rejected)` | 0 | `validate`'s outcome. A rejection's document lists `issues` |
| `Outcome(Outcome::Committed / Stale)` | 0 | `commit`'s outcome. `Stale` means the head moved after validation |
| `Outcome(Outcome::Other)` | 0 | any other document: a read, a proposal record, a seed result, a minted id, a hash |
| `Refusal(Refusal { code, reason })` | 2 | read from `ekr: <code>: <reason>` on stderr, for example `ekr.kernel.TransactionNotFound` or `session-request-malformed` |
| `Usage(message)` | 2 | clap's usage message for an argv the verb does not accept |
| `Fault(Fault { exit, message })` | 1, or any other status | a provider, verification, input or configuration failure |

A refusal or a fault is still a reply. An `Err` from `request` means no reply was read.

## Documents

`ekr_sdk::document` builds every document a consumer writes. It runs in process and sends no
request, so a document can be built before a session exists, or with none.

| builder | document | read by |
|---|---|---|
| `TransactionBuilder` | `TransactionDocument`, `ekr.transaction-document/2` | [`ekr propose`](cli.md#ekr-propose) |
| `SeedBuilder`, from `OntologySpec::seed` | `SeedDocument`, `ekr-seed/2` | [`ekr seed`](cli.md#ekr-seed) |
| `TypedReference::new` | a typed reference | [`ekr resolve`](cli.md#ekr-resolve) |
| `OntologySpec` with `Ontology::ensure` | the schema operations a store lacks | `ekr propose`, as one transaction |

`TransactionDocument::to_yaml`, `SeedDocument::to_yaml` and `TypedReference::to_yaml` write the
three documents as YAML, with every variant as a tag (`!CreateNode`, `!Node <id>`,
`!HumanStatement`). A `SchemaChange` is not a document: `SchemaChange::transaction` builds its
`TransactionDocument`. The readers refuse the one-key JSON form of a tag, so a document is never
written with `serde_json`.

In the examples below, `session` is a `ProcessSession` on a store that does not exist yet
([A session](#a-session)), and `operator` is the host document's `context.operator`, an `AgentId`.

### Ids and hashes are local

Every builder that creates something mints its id: `NodeDraft::new`, `EdgeDraft::new`,
`Assertion::new`, `Evidence::for_payload`, `TransactionBuilder::new`, and `OntologySpec::seed` for
every type, property, the schema version and the graph root. A minted id is `ekr-core`'s
`NodeId::mint()` (`TypeId::mint()`, `TransactionId::mint()`, …), which `ekr_sdk::document`
re-exports and which is the function [`ekr mint`](cli.md#ekr-mint) runs. `NodeDraft::with_id`
and the other `with_id` methods set a given id instead, which a test replaying a recording needs
([Recording and replay](#recording-and-replay)).

`payload_hash(bytes)` is the `ekr.payload.v1` hash that [`ekr hash`](cli.md#ekr-hash) prints as
`content_hash`, trailing newline included. `EvidenceAddition::new` hashes its payload and mints
its evidence id, so an entry and its bytes cannot disagree. `crates/ekr-sdk/tests/document_drift.rs`
holds both to the verbs.

### The ontology by name and the seed

A consumer names types and properties; a store knows them by id. `OntologySpec` declares node types
(`NodeTypeSpec`), edge types (`EdgeTypeSpec`) and their properties (`PropertySpec`, of a
`ValueSpec`) by name. `Ontology` maps each name to the id a store holds: `Ontology::node_type`,
`Ontology::edge_type`, `Ontology::edge_property`, and `Ontology::property`, which finds a property
on the type or on its nearest ancestor that declares it.

`OntologySpec::seed(created_at)` returns a `SeedBuilder` declaring the spec, with an empty graph,
and the `Ontology` of what it declares. `SeedBuilder::node` adds a `NodeDraft` in the lifecycle
state its caller passes, stored as given and not looked up. An `OntologySpec` declares no
lifecycle, so a node of one of its types takes `None`. `SeedBuilder::edge`,
`SeedBuilder::assertion` and `SeedBuilder::evidence` add the rest; the last files the payload under
its hash in `evidence_payloads`. Every entity names `SeedBuilder::root_id`. `SeedBuilder::build`
cannot fail.

```rust
use ekr_sdk::document::{
    Cardinality, EdgeTypeSpec, NodeDraft, NodeTypeSpec, OntologySpec, PropertySpec, Timestamp,
    ValueSpec,
};
use ekr_sdk::transport::{Request, Transport};

let spec = OntologySpec::new()
    .with_node_type(NodeTypeSpec::new("Author"))
    .with_node_type(
        NodeTypeSpec::new("Book")
            .with_property(PropertySpec::new("title", ValueSpec::String).as_required()),
    )
    .with_edge_type(
        EdgeTypeSpec::new("WROTE", ["Author"], ["Book"]).with_cardinality(Cardinality::Many),
    );
let (seed, names) = spec.seed(Timestamp::from_millis(1_790_000_000_000))?;
let root = seed.root_id();
let author_type = names.node_type("Author").ok_or("no Author type")?;
let author =
    NodeDraft::new(root, author_type, "The Field Naturalist").with_alias("field-naturalist");
let seed = seed.node(author, None).build();
let seeded = session.request(&Request::new(["seed", "-"]).with_stdin(seed.to_yaml()?))?;
```

### Building a transaction

`TransactionBuilder::new(proposer)` starts a transaction under a minted id, and
`TransactionBuilder::push` appends an operation. Every payload type converts into its
`Operation` (`NodeDraft` into `!CreateNode`, `EvidenceAddition` into `!AddEvidence`,
`AliasAddition` into `!AddAlias`, `EvidenceAttachment` into `!AttachEvidence`, …), so a
push reads `.push(draft.into())`. `!DeleteEdge` carries only an `EdgeId`, which has no such
conversion: push `Operation::DeleteEdge(edge_id)`. The proposer is the host's `context.operator`.

`TransactionBuilder::build` fills in what the kernel checks against the operations: the `evidence`
list is exactly the set the `!AddAssertion`s cite and the `!AttachEvidence`s attach, and a schema
change names a minted `schema_version` (`TransactionBuilder::with_schema_version` sets a given
one). It refuses:

| refusal | when |
|---|---|
| `DocumentError::EmptyTransaction` | the transaction has no operation |
| `DocumentError::MixedSchemaTransaction` | it mixes schema changes and data operations, which every profile refuses |
| `DocumentError::Limit` | the document is past one of the ten limits ([below](#the-ten-limits)) |
| `DocumentError::Yaml` | the YAML writer refused a value |

The document is sent as `propose -`, then `validate <id>` and `commit <id>`, three ordinary
requests whose replies read as in [Replies](#replies). A commit answered `Outcome::Stale` applied
nothing: propose it again under a new transaction id, which building the document again mints.
A `Batcher` sends all three for operations given as groups, and handles `Stale` and rejections
itself ([Batches](#batches)).

```rust
use ekr_sdk::document::{
    payload_hash, Assertion, Confidence, EvidenceAddition, EvidenceSource, Object, Predicate,
    Subject, TransactionBuilder, Value,
};

let book_type = names.node_type("Book").ok_or("no Book type")?;
let title = names.property("Book", "title").ok_or("no Book.title")?;
let name = Value::String("A Field Guide to Lichens".into());
let book = NodeDraft::new(root, book_type, "A Field Guide to Lichens")
    .with_property(title, name.clone())
    .with_alias("lichen-guide");
let statement = b"The title page reads: A Field Guide to Lichens.\n".to_vec();
let hash = payload_hash(&statement); // what `ekr hash` prints for the same bytes
let evidence = EvidenceAddition::new(
    EvidenceSource::human("Catalogue desk"),
    operator,
    Timestamp::from_millis(1_790_000_000_000),
    Confidence::CERTAIN,
    statement,
);
let claim = Assertion::new(
    root,
    Subject::Node(book.id),
    Predicate::Property(title),
    Object::Value(name),
    operator,
)
.citing(evidence.evidence.id);
let document = TransactionBuilder::new(operator)
    .push(book.into())
    .push(evidence.into())
    .push(claim.into())
    .build()?;

let id = document.transaction.id.to_string();
let proposed =
    session.request(&Request::new(["propose", "-"]).with_stdin(document.to_yaml()?))?;
let validated = session.request(&Request::new(["validate", id.as_str()]))?;
let committed_book = session.request(&Request::new(["commit", id.as_str()]))?;
```

### Ensuring an ontology

`Ontology::read(printed)` reads the document [`ekr ontology`](cli.md#ekr-ontology) prints, as
JSON text. `Ontology::ensure(&spec, profile)` compares it with a spec and returns a
`SchemaChange`: the schema operations that make the store declare everything the spec declares,
and `SchemaChange::ontology`, the name-to-id map once they commit, holding the ids minted for what
they add. It sends nothing.

| the store | `ensure` emits |
|---|---|
| lacks a node type | `!DefineNodeType`, with the properties the spec declares on it |
| lacks an edge type | `!DefineEdgeType`, with its properties |
| holds the type and lacks one of its properties | `!ModifyProperty` on that type |
| declares the property differently on that type | `!ModifyProperty` redeclaring it under the id the store holds. The kernel decides whether the store's data admits it |
| holds an edge type without one of the spec's ends | `!WidenEdgeType`, writing each end whole |
| declares everything the spec declares | nothing: `SchemaChange::is_empty` |

What the store holds beyond the spec is left alone. `SchemaChange::operations` lists node types,
then edge types, then properties, then widenings. `SchemaChange::transaction(proposer)` builds them
into one schema-change transaction, `None` when nothing is missing. A `Batcher` given them as one
group does the same, never puts them in a batch with data, and skips the group when it is empty.

| `OntologyError` | when |
|---|---|
| `SchemaFixed { missing }` | the store runs profile v1 (below) and the spec needs `missing` schema operations |
| `Conflict { kind, name, reason }` | the store declares what no schema operation changes: a node type's parents or abstractness, an edge type's cardinality, a property an ancestor declares differently. Also a spec whose parents form a cycle |
| `UnknownName { kind, name }` | the spec names a node type it does not declare and the store does not hold |
| `DuplicateName { kind, name }` | two node types, two edge types or two properties of one type share a name, in the spec or in the store |
| `Read` | the text is not a document `ekr ontology` prints |

**Profile v1 fixes the schema.** `ensure` takes the store's validation profile, which it does not
read from the store: `ValidationProfile::V1` for a host whose `authority.validation_profile.ruleset`
is `ekr.p1-deterministic/1`, `ValidationProfile::V2` for `ekr.p2-deterministic/1` and
`ValidationProfile::V3` for `ekr.p3-deterministic/1`. Under v1 a spec the store already declares
gives an empty change, so `ensure` still checks a v1 store against a spec. A spec that needs any
operation is `SchemaFixed`, and no operation moves a store from v1 to v2: seed a new store under
v2 or v3 ([Evolve the schema](cli.md#evolve-the-schema)).

```rust
use ekr_sdk::batch::Batcher;
use ekr_sdk::document::{Ontology, ValidationProfile};

let printed = session.request(&Request::new(["ontology"]))?;
let held = Ontology::read(&printed.document.unwrap_or_default().to_string())?;
let grown = spec.with_node_type(
    NodeTypeSpec::new("Journal").with_property(PropertySpec::new("issn", ValueSpec::String)),
);
let change = held.ensure(&grown, ValidationProfile::V2)?;
let report = Batcher::new(operator).commit(&mut session, &[change.operations().to_vec()])?;
let names = change.ontology();
let journal_type = names.node_type("Journal").ok_or("no Journal type")?;
```

### The ten limits

`ekr.transaction-document/2` freezes ten limits ([Document limits](cli.md#document-limits)), and
`ekr propose` refuses a document past any of them. `TransactionBuilder::build` checks the document
it writes against the same limits first, counting what the kernel's reader counts, so such a
document is refused before any request. `TransactionDocument::check_limits` checks a document
assembled by hand. `TRANSACTION_LIMITS` holds the bounds. `DocumentLimit::ALL` lists the ten, each
with `DocumentLimit::name` and `DocumentLimit::bound`.

| `DocumentLimit` | name | at most |
|---|---|---|
| `InputBytes` | `input_bytes` | 8,388,608 bytes of YAML (8 MiB) |
| `Operations` | `operations` | 10,000 operations |
| `Evidence` | `evidence_elements` | 10,000 ids in the transaction's `evidence` |
| `Depth` | `container_depth` | 32 levels of nesting, a tag counting as one |
| `Nodes` | `expanded_nodes` | 1,048,576 values, keys and tags |
| `MappingEntries` | `mapping_entries` | 4,096 entries in one mapping |
| `SequenceElements` | `sequence_elements` | 16,384 elements in any other sequence |
| `StringBytes` | `string_bytes` | 65,536 bytes in one string or tag name |
| `KeyBytes` | `key_bytes` | 4,096 bytes in one mapping key |
| `TotalStringBytes` | `total_string_bytes` | 33,554,432 bytes of strings, keys and tag names in all |

The refusal is `DocumentError::Limit { limit, bound, value }`: the first limit the document is
past, its bound and the document's count. Its message names the limit as `ekr propose` would. Split
the change into several transactions, or let a `Batcher` pack groups under the caps.

An `!AddEvidence` payload is written one list element per byte, so a payload over 16,384 bytes is
refused as `sequence_elements`. Put a larger statement into the seed, or split it into several
evidence entries.

```rust
use ekr_sdk::document::{
    AliasAddition, DocumentError, DocumentLimit, NodeId, TransactionBuilder,
};

let node = NodeId::mint();
let too_many = (0..10_001).fold(TransactionBuilder::new(operator), |builder, n| {
    builder.push(AliasAddition::new(node, format!("alias-{n}")).into())
});
let refused = too_many.build().unwrap_err();
assert!(matches!(
    refused,
    DocumentError::Limit {
        limit: DocumentLimit::Operations,
        bound: 10_000,
        value: 10_001
    }
));
assert_eq!(
    refused.to_string(),
    "transaction document limit: operations (at most 10000): this document has 10001"
);
```

## Resolve before you create

A `Resolver` answers a typed reference from a cache, and asks
[`ekr resolve`](cli.md#ekr-resolve) only when the cache cannot answer. A consumer that names the
same organization in 5,000 messages sends one resolve for it, not 5,000.

```rust
use ekr_sdk::document::TypedReference;
use ekr_sdk::resolve::{Resolution, Resolver};

let mut resolver = Resolver::new(root_id, operator);
let reference = TypedReference::new(organization_type, ["acme"]);
match resolver.resolve(&mut session, &reference)? {
    Resolution::Resolved(node) | Resolution::Queued(node) => { /* use node */ }
    Resolution::Ambiguous(candidates) => { /* read them and decide */ }
}
let flushed = resolver.flush(&mut session)?;
```

The cache key is the reference's exact type id and its aliases, sorted and deduplicated, with the
empty alias removed. So `["b", "a", "a"]` and `["a", "b"]` are one key. A reference to a supertype
and one to its subtype are different keys.

| answer | what the resolver does | returns |
|---|---|---|
| `Resolved` | caches the node | `Resolution::Resolved(node)` |
| `ProposeNew` | mints a node id, queues a `CreateNode` of the reference's type carrying the aliases `ekr resolve` named, and caches the id | `Resolution::Queued(node)` |
| `Ambiguous` | caches nothing | `Resolution::Ambiguous(candidates)`, in id order |
| a refusal, a fault | nothing | `CallError::Unanswered`, with the answer |

`resolve` names a queued node by the reference's first non-empty alias and gives it no values.
`resolve_with(transport, reference, || draft)` takes the node from `draft` instead. The resolver
sets its id, type and aliases; the draft supplies the root, the name and the values.

`flush` commits every queued node through a [`Batcher`](#batches), one group per node. It returns a
`Flushed`: the `BatchReport` of that commit and of every flush since the last call, and `replaced`
(below). Flush before committing anything that names a queued node. A queued id does not exist in
the store until its flush commits it.

The cache stays correct in three ways:

- **Queued nodes are flushed before any resolve that shares an alias with them.** When a resolve
  the cache cannot answer shares an alias with a queued node of the same type, the queue is
  flushed first, so `ekr resolve` finds that node rather than proposing a second one with the same
  alias. Queuing a node also drops every other cached key of its type that shares one of its
  aliases, because once the node is committed `ekr resolve` no longer answers such a key with the
  node it cached; it may answer `Ambiguous`.
- **Invalidation is per alias.** `invalidate(type_id, alias)` drops every cached key of that type
  that holds the alias. `observe(&report, &groups)` records a consumer's own `Batcher` commits.
  Their revisions count as the SDK's, and every alias that a committed `CreateNode` or `AddAlias`
  gives is invalidated. An `AddAlias` invalidates the alias under every type, because the resolver
  does not know the node's type.
- **The cache is dropped when `head` shows a commit the SDK did not make.** Each flush reads
  `ekr head` first. If any revision since the last check is not one the resolver committed or
  observed, the resolver drops its cache and resolves every queued reference again. If another
  process has meanwhile committed a node with one of those aliases, that node replaces the queued
  one. The queued id is never created, and `Flushed::replaced` maps it to the new resolution. So a
  second process's node produces no duplicate and no `alias-already-exists`. The same happens if
  the commit itself is rejected with `alias-already-exists` because such a node arrived between
  the head check and the commit. That rejection stays in the report.

When a flush fails, nothing queued is lost:

- A node whose commit reply was lost stays queued. The next flush sees a revision the resolver
  does not know, and resolves the reference again. If it finds the queued id itself, the SDK's own
  commit landed: the node is cached as resolved, and it is neither committed again nor listed in
  `replaced`.
- A node refused with `alias-already-exists` by a flush that then failed is resolved again by the
  next flush, before anything is proposed. It ends up in `replaced` or queued again.
- A stopped batch (`report.refused`, such as a resolver whose proposer is not the host operator)
  leaves its nodes queued.

## Batches

A `Batcher` commits a consumer's operations, given as atomic dependency groups (`Vec<Operation>`
each). A group is never split. Put operations that depend on each other, such as a `CreateNode`
and the `AddAlias` or `CreateEdge` that names it, in one group.

```rust
use ekr_sdk::batch::Batcher;

let report = Batcher::new(operator).commit(&mut session, &groups)?;
for rejected in &report.rejected {
    // rejected.operation, rejected.rejection, rejected.batch, rejected.group
}
```

- **Batches respect the caps.** Groups are packed in order into batches of at most 10,000
  operations and 8 MiB of YAML, the `ekr.transaction-document/2` limits. A schema change never
  shares a batch with data, and an empty group is skipped. `with_limits(operations, bytes)` sets
  lower caps, each held between 1 and the kernel's.
- **`Stale` is retried.** A commit that finds the head moved is proposed again under a newly
  minted transaction id, then validated against the new head and committed, up to eight times in
  a row.
- **A rejection is bisected.** Each batch is proposed, validated and committed as one transaction.
  If it is rejected, or refused by `ekr propose` for its document (`ekr.kernel.StructurallyInvalid`,
  `ekr.kernel.ProposalAttribution`), it is split in two and each half is submitted again, down to
  the single group that is refused. Every other group is committed.
- **A refusal of the transaction as a whole stops the run.** Some refusals are about the
  transaction rather than an operation in it, so every transaction of the run would be refused the
  same way. These are `ekr.kernel.ProposalAttribution` naming a registered submitter other than
  the batcher's proposer (the proposer is not the host operator), and a rejection with a
  `proposer-is-validator` issue. Such a refusal is not bisected. The run stops after that one
  proposal and validation, and `report.refused` names the refusal once, for every group not
  committed. A `ProposalAttribution` naming the batcher's own proposer is about one operation's
  attribution (an assertion's `proposed_by` or an evidence entry's `extracted_by`), and is
  bisected like a rejection.
- **Evidence travels with the assertions citing it.** `commit_with_evidence(transport, groups,
  evidence)` is `commit` with an `EvidenceSet` (below). Each transaction it proposes carries an
  entry's `!AddEvidence` in the group of the first assertion citing it, ahead of that group's
  operations, until a transaction that carries it commits. A group committed later cites the
  existing id. Bisection rebuilds each half the same way, so an assertion is never submitted
  without the evidence it introduces: if the group that first cited an entry is rejected, the
  next group citing it carries the entry instead, and an entry only rejected groups cite is never
  committed. A batch that the moved entry would take past the operation limit of `with_limits`
  is split in two rather than proposed.

A `BatchReport` holds:

| field | what it lists |
|---|---|
| `committed` | every committed transaction, as `CommittedTransaction`: `transaction` (its id), `revision`, `batch`, the `groups` it carried (by index in the input), and the `stale` ids it was proposed under before |
| `rejected` | every operation of every refused group, as `RejectedOperation`: `batch`, `group`, `index` within the group, the `operation` itself, and the `rejection` |
| `refused` | `None`, or the `RefusedBatch` that stopped the run: the `batch`, every `groups` index it did not commit (that batch's and every later batch's), and the `rejection` |

A `Rejection` is `Rejected { transaction, issues }` (each `Issue` has `validator`, `code` and
`message`, as `ekr validate` prints them), `Refused(refusal)` for a propose refusal, or
`Document(reason)` when the SDK could not write the group alone within the limits. Groups are
atomic, so every operation of a refused group is listed with the group's rejection. An issue's
message names the operation it is about.

If a request gets no answer the SDK can act on, `commit` returns a boxed `BatchError`. Its `report`
lists what was committed and rejected until then, and its `cause` is a boxed `CallError`: `Transport`,
`Unanswered` (a refusal not about the document, a usage message or a fault), `Unexpected` (a
document the SDK cannot read), `Document`, or `StaleRetries`.

**An unanswered commit's outcome is unknown.** Sometimes a `commit` request is sent and no outcome
comes back: the reply is lost to a timeout, a cancel or a dead session, or the verb answers a
fault, or the receipt cannot be read. `ekr` may have applied that commit. `BatchError::outcome_unknown` then names it as
an `UnknownOutcome`: the `transaction` id, its `batch`, and the `groups` it carried. It is not in
`report.committed`. Settle it before sending those groups again. Read `ekr transactions --state
Committed` (from a new session if this one failed): if the id is listed, its groups were committed
and must not be sent again. If it is not listed, send them again.

### Evidence items

A consumer that holds its evidence outside the store, one statement per message or document,
cites it through `ekr_sdk::evidence`. An `EvidenceItem` is a source identity, an observed-at
`Timestamp` and the exact bytes. `EvidenceSet::new(operator)` holds the entries, extracted by the
host operator. `EvidenceSet::cite(item)` returns the `EvidenceId` to cite with
`Assertion::citing`. The first time, it hashes the bytes with `payload_hash`, which is what
[`ekr hash`](cli.md#ekr-hash) prints. It then mints the id with `EvidenceId::mint`, which is what
`ekr mint evidence` runs, and builds an `EvidenceAddition` of source `!HumanStatement {identity}`
and confidence `Confidence::CERTAIN`. No request is sent. Within one set, an item with the same
source, observed-at time and bytes gets the same id, so 100 assertions citing 40 items add exactly
40 entries. A new set knows nothing of the store: a consumer that restarts builds its set with
`EvidenceSet::from_store(transport, operator)`, which reads `ekr snapshot` at the head and keys
every `!HumanStatement` entry the store holds by its identity, observed-at time and content hash.
An item the store already holds then gets that entry's id, and is not added again. An entry
another process commits after that read is not known to the set.

```rust
use ekr_sdk::evidence::{EvidenceItem, EvidenceSet};

let mut evidence = EvidenceSet::new(operator);
let cites = evidence.cite(EvidenceItem::new("Catalogue desk", observed_at, message_bytes));
let groups = vec![vec![book.into(), claim.citing(cites).into()]];
let report = Batcher::new(operator).commit_with_evidence(&mut session, &groups, &mut evidence)?;
```

`EvidenceSet::is_committed(id)` says whether the store holds the entry: a transaction committed
with the set added it, or `from_store` read it. `EvidenceSet::entry(id)` returns the
`EvidenceAddition` of an entry the set minted; an entry read by `from_store` has none. Do not
push a set's entry into a group yourself: the batcher adds it, and a second copy is refused as
`duplicate-identity`. An entry carries at most 16,384 bytes (`sequence_elements`,
[the ten limits](#the-ten-limits)); a group citing a larger item is rejected as
`Rejection::Document`. The report lists each group's own operations, never the `!AddEvidence`
the batcher added to it. If a commit's outcome is unknown (above) and `ekr transactions` lists it
as committed, mark the entries its groups cite with `EvidenceSet::mark_committed`, or rebuild the
set with `EvidenceSet::from_store`, so no later run adds them again.

## Retaining supplied knowledge

`ekr_sdk::knowledge::Knowledge` uses the same transport to retain observations and local
interpretations independently of canonical transactions. Its inputs and results come from
`ekr_sdk::contracts`, generated from the ESS domains.

```rust
use ekr_sdk::knowledge::{observation_import, Knowledge};

let input = observation_import(
    "manual".into(),
    Some("health-entry-1".into()),
    "2026-10-03T00:00:00Z".into(),
    serde_json::from_str("\"FeedItem\"")?,
    b"Project health: on track\n",
);
let mut knowledge = Knowledge::new(&mut session);
let receipt = knowledge.import_observation(&input)?;
let repeated = knowledge.import_observation(&input)?;
assert_eq!(receipt.observation_id, repeated.observation_id);
```

The builder reuses the observation source key and hashes the exact payload bytes. Capture time
is an RFC 3339 string. Repeat the same input for an idempotent retry; changing metadata under an
existing key is refused. `observations()` lists retained metadata and `observation(id)` returns
the record and its exact payload encoded as base64.

`import_interpretation(&EkrIntegrateInterpretationImport)` takes a typed document and its exact
JSON bytes encoded as base64. The runtime checks local declarations, references, retained source
evidence and the immutable version before publication. Vocabulary gaps produce inspectable
blockers and parked receipts. `interpretations()` lists version coordinates;
`interpretation(&version)` reads the document, blockers and receipts with its exact digest.
Rejected interpretations do not remove their retained observations. These operations neither
seed a store nor create a canonical revision. Store migration currently refuses stores containing
these independent streams before publishing a destination, to prevent a partial copy.

## Typed reads

`ekr_sdk::read::Reader` sends a read over any transport (a `ProcessSession`, a `&mut` one, a
recording or a replay) and returns a typed value instead of a `Reply`:

```rust
use ekr_sdk::read::{ExpandQuery, Reader, Since};

let mut reader = Reader::new(&mut session);
let overview = reader.overview(None, Some(50))?;          // Overview
let found = reader.search("Globex", None, None)?;         // NodeMatches
let detail = reader.describe(found.matches[0].id, None)?; // NodeDetail
for page in reader.expand(ExpandQuery::new(vec![detail.node.id], 1, 500)) {
    let page = page?;                                      // Slice
}
let changed = reader.changes(Since::Revision(0), None, None, None)?; // Changes
```

| method | `ekr` verb | value |
|---|---|---|
| `overview(revision, limit)` | `overview` | `Overview`, `ekr.graph-overview/1` |
| `search(text, limit, revision)` | `search` | `NodeMatches`, `ekr.node-matches/1` |
| `describe(node, revision)` | `describe` | `NodeDetail`, `ekr.node-detail/1` |
| `expand(query)`, `expand_page(&query, after)` | `expand` | `Slice` pages, `ekr.graph-slice/1` |
| `timeline(&query)` | `timeline` | `Timeline`, `ekr.graph-timeline/1` |
| `changes(since, at, limit, after)` | `changes` | `Changes`, `ekr.graph-changes/1` |
| `head()` | `head` | `Head`: `revision` and `root` |
| `snapshot(at, valid_at)` | `snapshot` | `Snapshot`: `root` and the graph, every map keyed by id |
| `ontology(at)` | `ontology` | `Ontology`: node and edge types by name and id, the schema version |
| `transactions(state)` | `transactions` | `Transactions`: each id, state, proposer, time and operation count |
| `explain(assertion)` | `explain` | `Explanation`, `ekr.explanation/2`: `links`, one `ExplanationLink` per kind, each record by hash |
| `explain_documents(assertion)` | `explain --documents` | the same `Explanation` with the whole records: each evidence link's `payload` and `text` |
| `quality(revision)` | `quality` | `StoreQuality`, `ekr.store-quality/1`: evidenced assertions, constrained properties, `shared_names` |
| `rejections(from, to)` | `rejections` | `Rejections`, `ekr.rejections/1`: each `RejectedTransaction` with its `issues` |
| `code_names(files, at)` | `code-names` | `CodeNames`, `ekr.code-names/1`: each `CodeNameFinding` with `file`, `line`, `column`, `runtime_word` and `names` |

The first six are the [`ekr.views` reads](cli.md#session-views), which `ekr` serves in a session
only. Each reads the store as it stands when `ekr` reads the request, so a commit made by this
session or by another process is what the next call reads, and `None` for a revision reads the
newest. `ExpandQuery` holds the seeds, `depth`, `limit` and, optionally, `edges` and `revision`;
`TimelineQuery` holds `hops`, `limit` and, optionally, `row_type`, `bucket`, `subject` and
`revision`; `Since` is `Revision(n)`, `Valid(t)` or `Recorded(t)`.

`expand(query)` returns `ExpandPages`, an iterator that reads one `Slice` each time it is advanced
and ends after the page without `next`. Every page after the first reads the revision that the
first page read, so a commit between two pages neither drops nor repeats a node or an edge. After
an error, the iterator ends. `changes` pages by hand: pass each page's `next` as `after`, and the
first page's `meta.revision` as `at`.

The last three are the store checks ([`ekr quality`](cli.md#ekr-quality),
[`ekr rejections`](cli.md#ekr-rejections), [`ekr code-names`](cli.md#ekr-code-names)).
`quality(revision)` reads the head when `revision` is `None`; a share is `None` when its whole is
0. `assertions.with_seed_evidence` counts active assertions citing or attached to retained seed
evidence, and `properties.constrained_types` counts node and edge types directly declaring a
constrained property. Shares remain integer basis points; divide by 10000 for a proportion.
`rejections(from, to)` selects the basis revisions `from` to `to`, both included, and a `None`
bound is unbounded. `code_names(files, at)` takes the source paths as strings; `ekr` reads a
relative one from its working directory (`SessionOptions::current_dir`), and each finding's `file`
is the path as given. A finding is not an error: test `meta.findings` to fail on one. A match's
`kind` is a `CodeNameKind`; a kind a newer `ekr` adds reads as `CodeNameKind::Other`, with its
`id` and the rest of the finding intact, and writes back as `Other`.

```rust
let quality = reader.quality(None)?;                       // StoreQuality
let rejected = reader.rejections(Some(0), None)?;         // Rejections
let found = reader.code_names(["src/reader.ts"], None)?;  // CodeNames
for finding in &found.findings {
    println!("{}:{} {}", finding.file, finding.line, finding.literal); // and .runtime_word
}
```

`head`, `snapshot`, `ontology`, `transactions`, `explain`, `quality`, `rejections` and
`code_names` also run without a session.
`OneShotReader::new(&binary, store, options)` runs each read as its own
`ekr --host … --store … --backend … <verb>` process. It starts the process the way a session does,
with the same environment and working directory, and stops it after `options.timeout`. It returns
the same typed value that a session returns for the same store state.

A read that returns no value fails with a `ReadError` that names the verb:

| `ReadError` | when |
|---|---|
| `Refused { verb, refusal }` | a named refusal, for example `ekr.views.NodeNotFound`, `ekr.views.RevisionNotFound`, `ekr.views.LimitExceeded` or `ekr.kernel.AssertionNotFound` |
| `Usage { verb, message }` | `ekr` did not accept the argv, for example because the binary predates the verb |
| `Fault { verb, fault }` | a store that does not open or cannot be read, including one never seeded for `head` |
| `Document { verb, source }` | the document does not read as the verb's value |
| `Transport(error)` | no reply was read. A one-shot `ekr` that printed something other than JSON is `TransportError::Protocol`, whose `answer` holds the start of what it printed |

The values are serde models of what `ekr` prints. A reader ignores a field it does not know, so a
newer `ekr` does not break an older consumer. A kind it does not know does not break the read
either: every closed set of kinds in these values ends in `Other`, and a kind a newer `ekr` adds
reads as `Other` while the rest of the document reads as usual. For a `changes` page, that change
has `change: ChangeKind::Other` with its `revision`, `recorded_at`, `id` and the other fields this
SDK models intact, and every other change of the page is unaffected. The same holds for
`MatchTier`, `MatchField`, `TransactionState`, `OntologyCardinality` and `CodeNameKind`, and for
`ViewValue`, `OntologyValueType`, `ExplanationLink`, `SnapshotSubject` and `SnapshotPredicate`,
whose `Other` drops the unknown kind's own payload. `ontology`, `snapshot` and `explain` read
into these read-side types; the document builders keep the strict `Cardinality`, `Subject` and
`Predicate`, which have no `Other`. An
`Other` writes back as `Other`, not as the kind `ekr` printed, and `TransactionState::Other` is
no state `transactions` can filter by. `ChangeKind::EvidenceAdded`, evidence an `AddEvidence`
brought after the seed, is modelled, with its `locator` and `content_hash`. A value holding no
`Other` writes back exactly the document it was read from. `crates/ekr-sdk/tests/read.rs`
checks this against every document that the `ekr-views` conformance fixture stores render and
against real `ekr` output, so if a format gains a field or a kind without an SDK update, that
test fails and names it by its pointer; `crates/ekr-sdk/tests/check_reads.rs` does the same for
the three store checks. In `Snapshot` and `Explanation`, the
parts that vary by kind stay JSON `Value`s, read by their tag as [the page](cli.md#ekr-snapshot)
documents them. These are an assertion's `object`, `assessment` and `lifecycle`, an evidence
entry's `source`, and the origin links of an explanation.

`Reader::ocel(&OcelQuery)` and `OneShotReader::ocel` return an `OcelExport` with its typed
`document` and `counts`. Counts are read from the request's stderr summary, not recomputed from
JSON. The query carries optional `revision`, `events` type names, and repeatable `event_time`
selectors (for example `Alert.fired_at`). `events` and `event_time` are mutually exclusive.
Timestamp selectors resolve against the requested revision, with inherited properties supported.

`Reader::code_names_with_mode(files, revision, CodeNameMode::Words)` and its `OneShotReader`
counterpart select whole-word matches, including identifiers and comments. The existing
`code_names` call retains literal mode. A word-mode result carries `meta.mode: Some(Words)`;
the default result omits that field when serialized.

## Sampled fact checks

`Reader::draw_sample(seed, size, filter, revision)` calls `ekr sample`, returning a typed
`checks::FactSample`: the revision, request and population, then the drawn facts in draw order,
with their names, assertions and retained evidence text or base64. The optional filter is a
`TypeId`; a revision of `None` reads the head. The same seed, size and revision reproduce the
same sample. `OneShotReader` provides the same call.

The runtime does no judging. Implement `checks::Judge::judge` for your judge, returning one
`Verdict` for each supplied fact in the same order. `checks::judge_sample(&sample, batch_size,
&mut judge)` visits batches in sample order and returns a `FactJudgements` document with the
sample's origin. `batch_size` is a `NonZeroUsize`. A judge error or a batch with the wrong number
of verdicts returns an error, with no completed judgement document; no report is submitted.

`Reader::report_judged(&judgements, confidence)` and the identical `OneShotReader` call send
that document to `ekr fact-quality -`. Confidence is optional integer basis points (9500 by
default). `checks::FactQuality` contains the counts, rate and Wilson interval; an empty judgement
set has `rate: None`, serialized as explicit `null`, and bounds `0` and `1`. The transport's
named refusals, usage errors and faults remain `ReadError` values.

## Recording and replay

`RecordingTransport::record(inner)` passes each request to `inner` and records every request that
received a reply, along with that reply. A request that failed is not recorded; its error is
returned as usual. `into_recording()` returns the `Recording`; `to_json()` writes it and
`Recording::from_json` reads it back. Its `format` is `ekr-sdk.recording/1`.

`ReplayTransport::new(recording)` answers from a recording and starts no process, so a consumer's
tests run with no `ekr` installed. Each request must equal the next recorded request, in both
`argv` and `stdin`. Any other request, or a request after the last one, fails with
`TransportError::Replay`. `remaining()` reports how many recorded exchanges have not been used. A
consumer whose requests depend on something outside the replies, such as a clock or a locally
minted id, must fix that value in its tests.

## The viewer

`Viewer::spawn(&binary, &store, port)` runs `ekr view --port <port>` with the default session
environment. It reads the `{"url": …}` line that the viewer prints first, and `url()` returns it.
Port `0` lets the viewer choose a free port. `stop()`, or dropping the `Viewer`, kills the viewer.
If the viewer exits or prints anything other than that line (for example because the store does
not exist), `spawn` returns `ViewerError::NoUrl` with the viewer's stderr tail.
