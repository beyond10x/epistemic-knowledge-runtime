# The `ekr-sdk` crate

`ekr-sdk` is a Rust library for driving an EKR store from a program. It runs a child
[`ekr session`](cli.md#ekr-session) and exchanges one JSON line per request with it, so a consumer
links neither the kernel nor the store. This page covers how the child is started, how requests
and replies are typed, how failures are handled, resolving references through a cache, committing
in batches, typed reads, and how to test without an `ekr` binary.

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
hashing, minting, seeding and the first writes all go through one process.
`processes_started()` reports how many `ekr` processes the session has started.

`SessionOptions` controls how the child runs:

| field | default | meaning |
|---|---|---|
| `environment` | `Environment::Store` | the child's environment. It is cleared and never inherits the consumer's. `Store` sets exactly `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND`. `Exact(list)` sets exactly `list`. The store is also passed as flags, so both forms reach the same store |
| `timeout` | 300 s | the longest one request may take, from writing it to reading its reply |
| `current_dir` | the consumer's | the child's working directory, which relative paths in a request resolve against |

The child is started with an absolute path and holds no `PATH` unless `Exact` gives it one. It
therefore starts when the consumer's own `PATH` is empty.

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
| `TransportError::Protocol` | the child printed a line that is not a reply |
| `TransportError::Cancelled` | the session was cancelled (below) |
| `TransportError::Latched` | a call made after any of the above. It names this call's verb and the original failure |

A one-shot process that times out, ends on a signal, or prints something other than JSON fails
only its own call. The error carries the last 4096 bytes of its stderr, and the session is not
latched.

`cancel_handle()` returns a `CancelHandle`. `cancel()` only stores `true` in an atomic flag, which
is async-signal-safe, so a signal handler can call it. A thread in the session checks the flag
every 20 ms and kills the child. A call in flight fails with `Cancelled`, and so does every later
call. The flag itself is `flag()`, so a program using `signal-hook` needs no handler code of its
own:

```rust
signal_hook::flag::register(signal_hook::consts::SIGTERM, session.cancel_handle().flag())?;
```

`close()` closes the child's input and waits up to `timeout` for it to exit. Before exiting, the
child writes the store's replay checkpoint. Dropping a healthy session does the same. A cancel
while closing kills the child within 20 ms instead of waiting out the timeout. Dropping a failed
or cancelled session kills the child.

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
  alias.
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
| `explain(assertion)` | `explain` | `Explanation`: `links`, one `ExplanationLink` per kind |

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

`head`, `snapshot`, `ontology`, `transactions` and `explain` also run without a session.
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
| `Transport(error)` | no reply was read |

The values are serde models of what `ekr` prints. A reader ignores a field it does not know, so a
newer `ekr` does not break an older consumer. Each value writes back exactly the document it was
read from. `crates/ekr-sdk/tests/read.rs` checks this against every document that the `ekr-views`
conformance fixture stores render and against real `ekr` output, so if a format gains a field
without an SDK update, that test fails and names the field. In `Snapshot` and `Explanation`, the
parts that vary by kind stay JSON `Value`s, read by their tag as [the page](cli.md#ekr-snapshot)
documents them. These are an assertion's `object`, `assessment` and `lifecycle`, an evidence
entry's `source`, and the origin links of an explanation.

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
