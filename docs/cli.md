# The `ekr` command line

`ekr` is the command-line surface of the Epistemic Knowledge Runtime: a store of typed, evidence-backed
knowledge in which every change is a transaction that is proposed, validated by a deterministic
validator and only then committed as a new immutable revision. You describe your domain once, as a
schema in a seed document, and then record knowledge against it as assertions that each cite the
evidence they rest on.

This page is the reference for a person or an agent that has a built `ekr` binary and no source. The
binary also describes itself: `ekr guide` prints the workflow, `ekr operations` the operation kinds,
and `ekr example <format>` a complete document of each input format.

`crates/ekr/tests/docs_cli.rs` holds this page to the binary: every seed, transaction and host
document below is parsed by the real readers, the worked example and the schema evolution example
are seeded and committed on both storage providers, the verb, option, operation-kind and
value-kind tables are compared with what the binary has, and the refusal table is checked as
[its introduction](#common-refusals) says.

Contents:

- [Install](#install)
- [Configuration](#configuration), including [the host document](#the-host-document-ekrcli-host1)
- [Exit codes and output](#exit-codes-and-output)
- [Verbs](#verbs)
- [The workflow](#the-workflow)
- [The seed document](#the-seed-document-ekr-seed2): [ontology](#the-ontology-section),
  [value types](#value-types), [graph](#the-graph-section), [evidence](#evidence-and-evidence_payloads)
- [Transaction documents](#transaction-documents-ekrtransaction-document2) and
  [operation kinds](#operation-kinds)
- [Worked example: a library catalogue](#worked-example-a-library-catalogue)
- [Evolve the schema](#evolve-the-schema)
- [Designing a schema](#designing-a-schema)
- [Common refusals](#common-refusals)

## Install

Build from a checkout of this repository with a Rust toolchain (the minimum version is in
[`README.md`](../README.md)):

```console
cargo build --release -p ekr
./target/release/ekr --version
```

The binary is `target/release/ekr` (under `$CARGO_TARGET_DIR/release/` if you set that variable).
Copy it onto your `PATH`; the rest of this page calls it `ekr`.

## Configuration

Verbs that read or write a store need three settings, and take a fourth. Each is a flag or an
environment variable; the flag wins, and an empty variable counts as unset.

| flag | variable | value |
|---|---|---|
| `--host` | `EKR_HOST` | path to the trusted host document, an `ekr.cli-host/1` JSON file (below) |
| `--store` | `EKR_STORE` | where the data lives: a directory for `file`, a database file for `sqlite` |
| `--backend` | `EKR_BACKEND` | `file` or `sqlite`, lowercase |
| `--full-replay` | `EKR_FULL_REPLAY` | optional: replay from the seed (below); the variable is `1` or `true` for on, `0` or `false` for off |

The store need not exist before `ekr seed`: the file provider creates the directory and any missing
parents, the SQLite provider creates the database file but not its directory. `guide`, `operations`,
`example`, `mint`, `hash` and `schema` open no store, need none of the settings and ignore the variables.

A store this process may read but not write — on a read-only mount, or owned by another user —
still answers every verb that only reads it. Such a verb opens the store read-only, writes nothing
at the store's path — no lock file, no journal file, no replay checkpoint — and prints the same bytes it
prints on a writable store. A verb that writes is refused `store-read-only` (exit 2,
[Common refusals](#common-refusals)) before it opens anything.

- **A file store** is read through a private copy of the whole store directory, taken under a
  shared lock on its `writer.lock` that excludes every writer. The copy is a directory
  `ekr-read-only-<pid>-…` in the temporary directory (`TMPDIR`, else `/tmp`), which needs free space
  for one more copy of the store for each process reading it. It is removed when the verb ends;
  `ekr view` and `ekr mcp` also remove it when sent SIGTERM, SIGINT or SIGHUP, and then exit with
  128 plus the signal's number. A copy left by a process that was killed outright is removed by the
  next read-only open in the same temporary directory.
- **A SQLite database** is read into memory through a read-only connection, and no `-wal` or `-shm`
  file is created beside it. With no `-wal` there, it is read `immutable=1`: SQLite takes no lock,
  so the read does not exclude a writer; the database's size and modification time are compared
  before and after, and a read that saw them change is taken again. With a `-wal` there, it is
  read through SQLite's own locks, and a `-wal` whose `-shm` is gone is not read.
- **A long-lived reader** — `ekr session`, `ekr view`, `ekr mcp` — checks the store's files before
  each request that reads it (a file store's `events.jsonl`, `manifest.json` and `blobs`; a SQLite
  database and its `-wal`), and when they have changed since it read them, it reads the store
  again, as it does for a store replaced at its path. A commit another process made is what the
  next request reads.

```console
export EKR_HOST=host.json EKR_STORE=./store EKR_BACKEND=file
```

The store keeps a replay checkpoint: the verified head state, written by the seed, by every fifth
commit and by a commit that brings the transaction documents committed since the last one to
16 MiB. The next verb continues from it, replaying only the few commits after it, instead of every
transaction since the seed. `--full-replay` ignores it and replays the whole history from the seed,
re-deriving every retained decision; a verb answers the same either way.

### The host document (`ekr.cli-host/1`)

The host document says who the operator and the validator are. It is trusted local configuration,
not input: keep it next to the store and use the same file for every command against that store.
`ekr example ekr.cli-host/1` prints a complete one.

| field | meaning |
|---|---|
| `format` | exactly `ekr.cli-host/1` |
| `tenant` | the namespace of this knowledge base inside the provider, for example `library`. A different tenant on the same store is a different, unseeded lineage |
| `context.operator` | the agent id that proposes and commits. Every transaction's `proposer`, every assertion's `proposed_by` and every seed evidence entry's `extracted_by` must be this id |
| `context.validator` | the agent id that validates. It must differ from the operator |
| `authority.format` | exactly `ekr.authority-state/1` |
| `authority.agents` | a map from agent id to `{id, name, capabilities}`. The key must equal `id`; both the operator and the validator must be registered. `name` is a label. `capabilities` is a list of distinct strings that P1 records and does not interpret (the example uses `propose`, `read` and `validate`) |
| `authority.validation_profile` | one of the three deterministic validation profiles. Copy it from `ekr example ekr.cli-host/1` unchanged except `validator`, which must equal `context.validator`. That is profile v1 (`ruleset` `ekr.p1-deterministic/1`, `application` `ekr.p1-apply/1`), under which the schema is fixed at seeding. Profile v2 is the same with `ruleset` `ekr.p2-deterministic/1` and `application` `ekr.p2-apply/1`, and admits [schema changes](#evolve-the-schema). Profile v3 is v2 with `ruleset` `ekr.p3-deterministic/1` and the same `application` `ekr.p2-apply/1`: it admits schema changes as v2 does, and also refuses a `CreateNode` or `CreateEdge` whose id an earlier revision held, such as a deleted edge's (`identity-previously-held`). What this page says of profile v2 holds of v3. A store keeps the profile it was seeded under |

Unknown, missing or duplicated fields are refused. A host document that parses but whose profile is
none of the three, or whose agent registry the kernel does not accept, is refused when the
store is opened, by any store verb including
`ekr seed`: `ekr: opening the provider: invalid seed: seed-authority-profile`, exit 1 (an operator
equal to the validator reads `…: invalid seed: proposer-is-validator`). The host's authority is
retained with the seed:
after `ekr seed`, a host document whose authority differs is refused (`bootstrap-authority-mismatch`,
exit 1), and one naming another tenant finds no seed (exit 1).

To make your own, mint two agent ids (`ekr mint agent`), put them into the example in place of its
ids — in `context`, as the keys and `id`s of `authority.agents`, and as
`authority.validation_profile.validator` — and choose a tenant. The worked example below does
exactly that.

## Exit codes and output

| exit | meaning | output |
|---|---|---|
| 0 | a declared outcome | one JSON document on stdout. A validation that rejects (`"kind": "Rejected"`) and a commit that finds the head moved (`"kind": "Stale"`) are outcomes too: read `kind` |
| 1 | a fault: provider, verification, unreadable input, host configuration, a store that is not seeded, no store at `--store` (`store-not-found`) | a message on stderr |
| 2 | a named refusal or a usage error. Nothing was recorded | `ekr: ekr.kernel.<Name>: <reason>` on stderr, `ekr: store-read-only: <reason>` for a verb that writes a store this process may not write, or clap's usage message |

`guide`, `operations` and `example` print text; `session` prints one JSON line per request; every
other verb prints one JSON document. In JSON output a tagged value is an object with one key —
`{"Node": "<id>"}` — where a YAML input writes the tag `!Node <id>`. A proposal record's
`document_bytes` prints as one standard base64 string.

## Verbs

| verb | store | input | prints |
|---|---|---|---|
| `ekr seed` | writes | an `ekr-seed/2` file, or `-` for stdin; `--evidence <file>`, repeatable | the seed result: `result.revision` is `0` |
| `ekr propose` | writes | an `ekr.transaction-document/2` file, or `-` | the proposal record: `transaction_id` |
| `ekr validate` | writes | a transaction id; `--against <revision>` | the validation outcome: `kind` is `Validated` or `Rejected` (with `issues`) |
| `ekr commit` | writes | a transaction id | the commit outcome: `kind` is `Committed` (with `result.revision`) or `Stale` |
| `ekr snapshot` | reads | `--at <revision>`, `--valid-at <ms or YYYY-MM-DD>` | the whole graph at one revision |
| `ekr explain` | reads | an assertion id; `--documents` | the `ekr.explanation/2` document: the assertion, where it came from, what later changed it, and its evidence, each record by hash; with `--documents`, the whole records too |
| `ekr resolve` | reads | a `typed-reference` file, or `-`; `--at <revision>` | the resolution: `kind` is `Resolved` (with `node_id`), `ProposeNew` (with `type_id` and `aliases`) or `Ambiguous` (with `candidates`) |
| `ekr head` | reads | none | the head `revision` and its `root` |
| `ekr transactions` | reads | `--state <State>` | every retained transaction: id, state, proposer |
| `ekr rejections` | reads | `--from <revision>`, `--to <revision>` | the `ekr.rejections/1` document: each rejected transaction with its validation issues, by the revision it was validated against |
| `ekr ontology` | reads | `--at <revision>` | node types, edge types and properties with names and ids, and the schema version in force: `schema_version`, `schema_version_number`, `schema_version_parent` |
| `ekr code-names` | reads | one or more source files; `--at <revision>` | the `ekr.code-names/1` document: every literal in the files that equals one of the store's names, with file, line and what it names; exits 0 however many it finds |
| `ekr quality` | reads | `--revision <revision>` | the `ekr.store-quality/1` document: evidenced assertions, constrained properties, names shared within a type |
| `ekr ocel` | reads | `--revision <revision>`, `--events <type name>...` | the `ekr.ocel/1` document: the revision as an OCEL 2.0 event log in its `ocel` member, and `names` for its ids |
| `ekr sample` | reads | `--seed <integer>`, `--size <1–1000>`, `--type <type id>`, `--revision <revision>` | the `ekr.fact-sample/1` document: a reproducible sample of the revision's facts, each with its evidence bytes, for a judge |
| `ekr fact-quality` | none | an `ekr.fact-judgements/1` file, or `-`; `--confidence <basis points>` | the `ekr.fact-quality/1` document: the judged sample's pass rate and its Wilson interval |
| `ekr guide` | none | none | the workflow, as text |
| `ekr operations` | none | an operation kind, optionally | the kinds, or one kind's fields and example |
| `ekr example` | none | `ekr.transaction-document/2`, `ekr.transaction-document/1`, `schema-change`, `ekr-seed/2`, `ekr.cli-host/1`, `typed-reference` or `ekr.extraction-document/1` (aliases `transaction` for `/2`, `seed`, `host`, `extraction`) | a complete example document |
| `ekr mint` | none | an id kind | `{"id", "kind"}`: a fresh id |
| `ekr hash` | none | a payload file, or `-` | the payload's `content_hash` and its `payload_yaml` |
| `ekr schema` | none | `ekr.transaction-document/2`, `ekr.transaction-document/1`, `ekr-seed/2`, `ekr.cli-host/1`, `typed-reference` or `ekr.extraction-document/1` (aliases `transaction` for `/2`, `seed`, `host`, `extraction`) | the format's JSON Schema (draft 2020-12) |
| `ekr view` | reads | `--port <port>` (`0`, the default, picks a free one) | `{"url": "http://127.0.0.1:<port>/"}` as one line, then serves a read-only viewer until interrupted |
| `ekr session` | reads and writes | one JSON request per line on stdin, `{"argv": [...]}`, until it ends; `--create` also serves `seed` | one JSON answer per request, `{"exit", "stdout", "stderr"}`: what the verb exits with and prints |
| `ekr mcp` | reads | JSON-RPC 2.0 messages, one per line on stdin, until it ends | one JSON-RPC response per request: read-only MCP tools over the store (below) |
| `ekr migrate` | reads, and writes a new store | `--to <path>`: where the migrated store is written, holding no store yet | the `ekr.store-migration/1` report: `destination_seed_hash` and which record replaced which |

Every verb has `--help`.

The `store` column is the binary's own: on a store this process may not write, a verb that `writes`
is refused `store-read-only` (exit 2) and one that `reads` answers as on a writable store
([Configuration](#configuration)). `crates/ekr/tests/read_only_store.rs` runs every verb of both
kinds but `view`, which serves until interrupted, on a read-only store of each provider against
this table, and `session` and `migrate` on one too.

### `ekr seed`

Writes revision 0 from an `ekr-seed/2` document: the schema, the initial graph and the evidence
payloads. The seed is validated like a transaction first; a seed that does not validate is refused as
`ekr.kernel.InvalidSeed` with the reason. Seeding the same document again returns the original result
(exit 0); a different document on a seeded store is refused as `ekr.kernel.AlreadySeeded`.

Evidence payloads can come from files instead of the document. `--evidence <file>`, repeatable, adds
the file's exact bytes to `evidence_payloads` under their content hash — the `content_hash` that
`ekr hash <file>` prints for the evidence entry — so the document can say `evidence_payloads: {}`
and need not carry the bytes as a list. A payload both in the document and in a file lands once. The
kernel checks the completed document as it would a pasted one: an entry whose payload no file or key
supplies is refused as `seed-evidence-payload-missing`, and a file no entry cites is retained like
an uncited pasted payload. A file that cannot be read exits 1 before any store is opened.

```console
ekr hash corpus/a.md                              # -> content_hash for the evidence entry
ekr seed seed.yaml --evidence corpus/a.md --evidence corpus/b.md
```

### `ekr propose`

Records a transaction document, unvalidated, as the host operator, and prints its proposal record.
The document's `proposer` must be the host operator (`ekr.kernel.ProposalAttribution` otherwise).
A document that does not parse is refused as `ekr.kernel.StructurallyInvalid`. Proposing records
the transaction in state `Proposed`; nothing reaches the graph yet.

### `ekr validate`

Runs the deterministic validators over a proposed transaction against a committed revision — the
head, or `--against N` — as the host's validator. `kind: Validated` means it may be committed;
`kind: Rejected` lists `issues`, each with a `code`, the `validator` that raised it and a `message`.
Both are recorded outcomes and exit 0. A rejected transaction is final; fix the document and propose
it again under a new transaction id.

### `ekr commit`

Applies a validated transaction and publishes a new revision. `kind: Committed` carries
`result.revision`. If another commit moved the head after validation, the outcome is `kind: Stale`
and nothing is applied; propose the transaction again under a new id and validate it against the new
head. Committing a transaction that is not `Validated` is refused as
`ekr.kernel.TransactionStateConflict`.

### `ekr snapshot`

Prints the graph at the head, or at `--at N`: `root` (with `revision`), and `graph.graph` with
`nodes`, `edges`, `assertions` and `evidence`, each a map keyed by id, and `attachments` when
any assertion has evidence attached: assertion id → a list of `{evidence, revision}` in evidence id
order, absent when there is none. A plain snapshot returns every
assertion, including retracted and superseded ones, with `matching_assertions: null`. With
`--valid-at` (milliseconds since the Unix epoch, or `YYYY-MM-DD` for midnight UTC),
`matching_assertions` lists the ids of the assertions believed at that instant: accepted, not
retracted, with a valid time that contains it. Valid time is half-open: `from` is included, `to` is
not.

### `ekr explain`

Explains one assertion at the head, as the `ekr.explanation/2` document: `format`
(`ekr.explanation/2`), `assertion_id`, `at` (the head revision it was read at) and `links`, a list
of objects, each with a `kind`. The retained records a link stands for — a proposal, a commit
receipt, an evidence payload — are named by their hash, not printed:

| `kind` | what it is | when it appears |
|---|---|---|
| `Assertion` | an assertion as it stands at the head | first, for the requested assertion; again for each assertion that superseded it, followed by that one's own origin and lifecycle links |
| `Seed` | the seed the assertion came from | for an assertion written in the seed, in place of `Proposal`, `Validation` and `Commit` |
| `Proposal` | the proposal of the transaction that added it: `transaction_id`, `record_hash` (the proposal record's hash), `event_id`, `submitter`, `submitted_at`, `document_hash`, `operation_count`, and `operations`, only the document's operations about this assertion (its `AddAssertion`) | for an assertion added by a transaction |
| `Validation` | that transaction's validation receipt | with `Proposal` |
| `Commit` | that transaction's commit: `transaction_id`, `revision_id`, `event_id`, `committer`, `committed_at`, `result` (the root it produced, `result.revision` its number), `result_hash`, `record_hash` (the receipt's hash), `proposal_record_hash` and `validation_record_hash` | with `Proposal` |
| `Lifecycle` | a later committed retraction or supersession of it: `assertion_id`, `lifecycle`, and `commit`, that change's commit in the shape of a `Commit` link | once per such change |
| `Attachment` | evidence attached to it after it was added (`!AttachEvidence`): `assertion_id`, `evidence_id`, `revision` (the revision that attached it), and `commit`, the attaching commit in the shape of a `Commit` link | after its `Lifecycle` links, once per attachment the revision read holds, in evidence id order; an explanation at a revision before the attachment does not list it |
| `Evidence` | an evidence entry cited; its bytes are the ones at its `content_hash` | last, once per evidence id cited by an assertion in the chain, attached to one, or listed in the `evidence` of a transaction that added or changed one |

With `--documents`, each link also carries the whole record it names, read from the same revision:
a `Proposal` link the proposal record as `record` (its `document_bytes` one base64 string), a
`Commit` link and a `Lifecycle` or `Attachment` link's `commit` the commit receipt as `receipt`, and an `Evidence`
link `payload` (the retained bytes, base64) and `text` (the same bytes as a string, when they are
UTF-8). Without it the answer's size does not follow the size of the transactions on the chain.

Read links by their `kind` and, for `Assertion` and `Lifecycle`, by the assertion id they carry:
`id` on an `Assertion` link, `assertion_id` on a `Lifecycle` link. Do not read them by position, because the number and order of links depend on the assertion's
history. An unknown id is refused as `ekr.kernel.AssertionNotFound`.

### `ekr resolve`

Finds the node a typed reference names, at the head or at `--at N`, and writes nothing. Run it
before a `!CreateNode`: an agent that creates a node it could have found makes a duplicate. A
typed reference is a YAML document with two keys (`ekr example typed-reference`, `ekr schema
typed-reference`):

- `type_id` — the node type the node is an instance of, from `ekr ontology`: a concrete type, never
  an abstract one or one with a subtype;
- `aliases` — the names the node is known by. Each is compared byte for byte with a node's
  `aliases`; order and repeats do not matter, and an empty alias identifies nothing.

A node is a candidate when its `type_id` is exactly the reference's and one of its aliases is one
of the reference's. Its `canonical_name` is never compared: a name is not an identity. The
result is one JSON document; read `kind`:

| `kind` | when | what to do |
|---|---|---|
| `Resolved` | exactly one candidate, `node_id` | use that id; create nothing |
| `ProposeNew` | no candidate; `type_id` and the identifying `aliases`, sorted and deduplicated | mint an id (`ekr mint node`) and propose a `!CreateNode` of that type. It does not mean "retry with a looser reference" |
| `Ambiguous` | more than one candidate, every one in `candidates`, in id order | none is chosen: read them (`ekr snapshot`) and decide |

A reference that cannot be resolved at all is refused, exit 2, with `ekr: <code>: <reason>` on
stderr and nothing on stdout. The checks run in this order and the first that applies is the
answer:

| code | means | fix |
|---|---|---|
| `reference-without-identity` | the reference holds no alias but the empty string | give at least one alias |
| `reference-type-undeclared` | `type_id` is not a node type the ontology at that revision declares | take the id from `ekr ontology` |
| `reference-type-has-subtypes` | `type_id` is an abstract type or has a declared subtype | name the concrete type the node is an instance of |

Give the `!CreateNode` the reference's aliases: the created node is then a candidate at the next
revision. A `!CreateNode` naming an alias a node of its type already holds is `Rejected`
(`alias-already-exists`); resolve again and use the node it returns. A node that exists but lacks
the alias gains it with `!AddAlias` ([operation kinds](#operation-kinds)), and is a candidate for it
from the next revision on.

A document that is not a typed reference exits 1, `ekr: typed reference <file>: <reason>`, before
the store is opened. That includes a document over 1048576 bytes, a YAML alias (`*name`), a tag,
and a `type_id` or an alias that YAML reads as a number, a boolean or null (quote it: `"123"`), or
an `aliases` that is not a list: the reader refuses what `ekr schema typed-reference` refuses. It
also refuses, as `nested deeper than 64 levels`, a document holding more than 64 `[` or `{` in
all (counted everywhere, inside quoted aliases too) or more than 64 block indentation levels on a
line (the more-indented lines of a `|` or `>` block scalar are text and do not count); this is
checked before the YAML is loaded. A revision that does not exist is refused as `ekr.kernel.RevisionNotFound`,
exit 2, as for `ekr snapshot --at`.

### `ekr head`

Prints the newest revision number and its root hashes. Use it for `validate --against` and to see
that a commit landed.

### `ekr transactions`

Lists every retained transaction with `transaction_id`, `state`, `proposer`, `operation_count` and
`submitted_at`. `--state` filters by one of `Proposed`, `Validated`, `Committed`, `Rejected` or
`Stale`.

### `ekr rejections`

Lists each rejected transaction with the validation issues its rejection recorded, keyed on the
revision it was validated against: the `ekr.rejections/1` document (`ekr.kernel.RejectionsV1` in
`systems/ekr/domains/kernel.yaml`). `--from N` and `--to M` select the basis revisions `N` to `M`,
both included; either may be left out, and a range with `N` above `M` selects nothing. Each entry
of `rejections` carries `transaction_id`, `against` (the revision `ekr validate --against` named),
`proposer`, `rejected_at` and `issues`: every issue exactly as `ekr validate` printed it in its
`Rejected` result — `id`, `transaction_id`, `validator`, `code` and `message`, in the order the
rejection recorded them. Entries are ordered by `against`, then `transaction_id`; `from` and `to`
echo the request and are left out when it gave none.

Only rejections appear. A committed transaction has no issues: the kernel commits a transaction
only when no validator raised one. The document names neither the head nor the time of the read,
so two reads of one range print the same bytes, and a later commit changes nothing already
printed; only a new rejection in the range does.

### `ekr ontology`

Prints the schema in force at the head, or at `--at N` as of that committed revision:
`node_types` and `edge_types` with their ids, names, parents, endpoint types and properties, and the
schema version — `schema_version` (its id), `schema_version_number` (`0` at the seed, one more per
committed schema change) and `schema_version_parent` (the version it was derived from, `null` at the
seed). These are the ids a transaction document uses for `type_id`, `predicate: !Relation` and
property keys. A revision that does not exist is refused as `ekr.kernel.RevisionNotFound`, exit 2,
as for `ekr snapshot --at`.

### `ekr code-names`

`ekr code-names <file>... [--at N]` checks that code which reads a store stays generic over any
ontology: it reports every literal in the given source files that equals one of the store's names,
at the head or as of revision `N`. It reads the files and the store and writes nothing.

- **A literal** is the text between two quotes of the same character — `"`, `'` or a backtick — on
  one line, with `\` escaping the character after it. Each line is scanned twice and a literal either
  scan finds counts once: one pass over all three characters at once consumes whole literals, so
  the `"` in `'"'` or the `'` in `"can't"` opens nothing and the literal after it is still found;
  one pass per character, blind to the other two, finds `"…"` inside `'…'` as well. A quote with no
  partner on its line opens nothing, and no literal spans a line. The text is compared raw: no
  escape is decoded. A bare identifier (`Volume` outside quotes) is not a literal; comments are not
  recognised, so a quoted name in a comment is a literal like any other. The rule is the same for
  every language, and the scan is linear in the length of a file.
- **A store name** is a node type's or edge type's name, a property's name, or a node's canonical
  name or alias, at that revision. A literal equals a name when the two are the same text, case
  included.
- **Flagged, not dropped:** a store name that is also one of the runtime's own words — every field
  name, enum variant and union tag its specification declares (`name`, `aliases`, `kind`, `String`,
  `Accepted`, `CreateNode`, `ekr.graph-projection/1`, …), the words any reader of `ekr`'s documents
  uses — is reported with `"runtime_word": true`, and `meta.runtime_word_findings` counts those
  findings: the literal may be the runtime's word rather than the store's name, and you decide.
- **Never reported:** a name that is the text of one of the store's ids (a node's alias that is an
  old id, say). A literal equal to such a name is counted in `meta.exempt` instead.

It prints one `ekr.code-names/1` document:

```json
{
  "meta": {"format": "ekr.code-names/1", "revision": 0, "files": 2, "literals": 14, "exempt": 1,
           "findings": 1, "runtime_word_findings": 0},
  "findings": [
    {"file": "src/reader.ts", "line": 7, "column": 16, "literal": "Folio", "runtime_word": false,
     "names": [{"kind": "NodeType", "id": "<type id>"},
               {"kind": "Alias", "id": "<node id>", "type_id": "<its type id>", "type_name": "Volume"}]}
  ]
}
```

`file` is the path as given on the command line. `line` is 1-based, `column` the 1-based position
of the opening quote in characters. `names` lists every store name the literal equals: `kind` is
`NodeType`, `EdgeType`, `Property`, `CanonicalName` or `Alias`, `id` the type's, property's or
node's id, and for a node's name `type_id` and `type_name` its type. `runtime_word` is `true` when
the literal is also a runtime word. Files are read in order of their path and each once, findings
follow in file, line and column order, so the same files and revision print the same document in
any argument order.

**Findings are not a failure: the verb exits 0** and `meta.findings` is the count. To fail a build
on a finding, test that count. Exit 1 is a fault — a file that does not read or is not UTF-8 text
(the message names it), or no store at `--store`. A revision the store does not hold is refused as
`ekr.views.RevisionNotFound`, exit 2. `ekr session` serves the verb too.

### `ekr quality`

Prints how good the store's knowledge is at the head, or at `--revision N` as of that committed
revision, beyond how much of it there is: the `ekr.store-quality/1` document
(`ekr.views.ReportStoreQuality`). It counts that revision's canonical state only, so two reads of
one revision print the same bytes, before and after any later commit. Refused transactions are not
in it; `ekr transactions --state Rejected` lists them.

```console
ekr quality --revision 1
```

```json
{
  "assertions": {
    "active": 4,
    "with_evidence": 4,
    "with_evidence_share": 10000,
    "with_item_evidence": 1,
    "with_item_evidence_share": 2500
  },
  "meta": {
    "format": "ekr.store-quality/1",
    "revision": 1
  },
  "properties": {
    "constrained": 0,
    "constrained_share": 0,
    "declared": 1
  },
  "shared_names": [
    {
      "name": "Alice",
      "nodes": [
        "00000000-0000-4000-8000-000000000301",
        "00000000-0000-4000-8000-000000000901"
      ],
      "type": "00000000-0000-4000-8000-000000000201"
    }
  ],
  "sharing_nodes": 2
}
```

| field | what it counts |
|---|---|
| `assertions.active` | the revision's assertions whose lifecycle is `Active`; a retracted or superseded one is not counted |
| `assertions.with_evidence` | of those, the ones citing at least one evidence entry the store holds with its bytes. Every assertion the kernel admits cites evidence, so this equals `active` in a store `ekr` wrote |
| `assertions.with_item_evidence` | of those, the ones citing at least one evidence entry added after the seed by an `AddEvidence` ([Evidence after the seed](#evidence-after-the-seed)): the figure counts when evidence entered, not how finely it was cut: a seed that carries one evidence entry per assertion still reports `0` here |
| `properties.declared` | the property declarations of the revision's schema: each property each node type and edge type declares itself |
| `properties.constrained` | of those, the ones declaring at least one entry in `constraints` |
| `shared_names` | every name — a canonical name or an alias, compared exactly as text — that two or more nodes of one type hold: the `type`, the `name` and the `nodes`, by id. Ordered by type id, then name; the empty name is never listed |
| `sharing_nodes` | the distinct nodes `shared_names` lists |

A `_share` is basis points: 10000 times the count divided by its whole, rounded down, so `10000` is
all of it; it is left out when the whole is `0`. The document is printed as every verb prints its
JSON, keys in alphabetical order; `ekr session` answers it as `"stdout"`. A store never seeded is
refused as `ekr.views.NotSeeded` and a revision it does not hold as `ekr.views.RevisionNotFound`,
exit 2, as the `ekr.views` reads refuse them.

### `ekr ocel`

Prints the store at the head, or at `--revision N` as of that committed revision, as an
object-centric event log in the [OCEL 2.0](https://www.ocel-standard.org/) JSON format: the
`ekr.ocel/1` document (`ekr.views.ExportOcel`). Its `meta` names the revision, its `names` gives
every type and property id the ontology's name for it, and its `ocel` member is the log. The `ocel`
member is what an OCEL 2.0 reader reads; write it to a file of its own, for example with
`jq .ocel`. Two reads of one request print the same bytes, before and after any later commit.

```console
ekr ocel --revision 0
ekr ocel --events Person
```

The event types are EKR's one event-type rule, the same types the overview marks as events
(`roles.types[].event` of `ekr.graph-overview/1`), the timeline walks to and `GET /roles` marks
`event` or `observation` (§ Roles). The rule: a node is judged when it holds a timestamp-like
property value (a `Timestamp`, or an `Integer` of epoch milliseconds from 1,000,000,000,000 up to
10,000,000,000,000), or when it has at least two dated facts: `valid_time.from` of assertions of
any lifecycle whose subject it is, or whose relation object it is. It is instant when it is
timestamped or its dated facts lie within one hour. A type is an event type when it has a judged
node and at least 60% of its judged nodes are instant.
`--events <type name>...` names the event types instead: exactly the node types with those names,
a name two types share naming both; a name no node type holds is refused as
`ekr.views.EventTypeNotFound` (exit 2), naming it.

The example seed (`ekr example ekr-seed/2`) has no event type under the rule. Bob's two dated facts
lie six years apart and Alice has one, so Person has no instant node; Organization's Acme is the
relation object of facts spread over the same years. The log is every node as an object:

```json
{
  "meta": {
    "format": "ekr.ocel/1",
    "revision": 0
  },
  "names": {
    "edge_types": [
      {
        "id": "00000000-0000-4000-8000-000000000203",
        "name": "CEO_OF"
      }
    ],
    "node_types": [
      {
        "id": "00000000-0000-4000-8000-000000000201",
        "name": "Person"
      },
      {
        "id": "00000000-0000-4000-8000-000000000202",
        "name": "Organization"
      }
    ],
    "properties": [
      {
        "id": "00000000-0000-4000-8000-000000000801",
        "name": "legal_name"
      }
    ]
  },
  "ocel": {
    "eventTypes": [],
    "events": [],
    "objectTypes": [
      {
        "attributes": [],
        "name": "00000000-0000-4000-8000-000000000201"
      },
      {
        "attributes": [
          {
            "name": "00000000-0000-4000-8000-000000000801",
            "type": "string"
          }
        ],
        "name": "00000000-0000-4000-8000-000000000202"
      }
    ],
    "objects": [
      {
        "attributes": [],
        "id": "00000000-0000-4000-8000-000000000301",
        "relationships": [
          {
            "objectId": "00000000-0000-4000-8000-000000000303",
            "qualifier": "00000000-0000-4000-8000-000000000203"
          }
        ],
        "type": "00000000-0000-4000-8000-000000000201"
      },
      {
        "attributes": [],
        "id": "00000000-0000-4000-8000-000000000302",
        "relationships": [],
        "type": "00000000-0000-4000-8000-000000000201"
      },
      {
        "attributes": [],
        "id": "00000000-0000-4000-8000-000000000303",
        "relationships": [],
        "type": "00000000-0000-4000-8000-000000000202"
      }
    ]
  }
}
```

`ekr ocel --events Person` makes Alice and Bob events at their earliest dated fact,
`2020-01-01T00:00:00.000Z`, and the CEO_OF edge an event-to-object relationship on Alice.

| OCEL 2.0 | what the store gives it |
|---|---|
| event type | a node type the rule above marks, or one `--events` names |
| event | each node of an event type, at its time as the timeline places it: its least timestamp-like property value, else its earliest dated fact. A node of an event type with no time is left out, and so is every edge to it |
| object type | every other node type, one with no node included |
| object | each node of an object type |
| attribute | each property a node holds a value of; a type's attributes are the properties it declares or inherits |
| relationship | each edge, qualified by its type id: on the event when one end is an event and the other an object, on the source when both are objects; an edge between two events is left out, since OCEL 2.0 relates events to objects only. Relationships are a set: two edges of one type with the same holder and object give one relationship, and the second is counted as merged (`parallel_edges_merged`) |

Types, attributes and qualifiers are named by id, not by name: OCEL 2.0 identifies a type and an
attribute by its name, and two of a store's types or properties may share one. `names` maps each
id back: `node_types` and `edge_types` one entry per declared type, `properties` one entry per
property id and name a type declares it under, each ordered by id. A time is RFC 3339 in UTC with
milliseconds. Every attribute value is a string, as the OCEL 2.0 JSON schema requires: a single
value of a `One` property of a scalar kind is its text (an integer in decimal, a `Timestamp` as a
time, a `NodeRef` as the node's id), and any other value list is its JSON as the projection writes
`props`. An attribute typed `time` holds a time: a `Timestamp` outside the years 0000–9999 is left
out of the log and counted (`attribute_values_out_of_range`). An object's attribute values carry
the time `1970-01-01T00:00:00.000Z`, which OCEL 2.0 gives an object's initial values, or the time
of the log's earliest event when that is earlier, so no event precedes the values of the objects
it relates to; the store holds a node's properties without a time. A node's name, aliases and
lifecycle state, and an edge's properties, are not part of the log. `ekr session` serves the verb
too; the other refusals are `ekr quality`'s.

A known departure from OCEL 2.0: its Definition 2 makes an attribute name one type's, and a
property a type inherits is an attribute of that type and of every type inheriting it, under one
property id. The OCEL 2.0 JSON schema and common readers, the `process_mining` crate among them,
accept such a log.

### `ekr sample`

Prints a reproducible sample of the store's facts at the head, or at `--revision N`, each with the
bytes of the evidence it cites: the `ekr.fact-sample/1` document (`ekr.views.DrawFactSample`). It
is the first half of measuring fact quality: you, or an agent you run, judge each fact against its
evidence, and [`ekr fact-quality`](#ekr-fact-quality) reports the pass rate. The runtime judges
nothing.

```console
ekr sample --seed 42 --size 1
```

```json
{
  "items": [
    {
      "assertion": {
        "assessment": {
          "kind": "Accepted",
          "validators": [
            "00000000-0000-4000-8000-000000000102"
          ]
        },
        "evidence": [
          "00000000-0000-4000-8000-000000000402"
        ],
        "id": "00000000-0000-4000-8000-000000000511",
        "lifecycle": {
          "kind": "Active"
        },
        "object_kind": "Node",
        "object_ref": "00000000-0000-4000-8000-000000000303",
        "predicate": "00000000-0000-4000-8000-000000000203",
        "predicate_kind": "Relation",
        "recorded_from": 1790796575216,
        "valid_from": 1773273600000
      },
      "evidence": [
        {
          "content_hash": "bd1d4dc9ba5012df5e43505418aa7728c15bca052a1ad43b1a2d00d04b75bdd6",
          "id": "00000000-0000-4000-8000-000000000402",
          "kind": "HumanStatement",
          "locator": "Runtime operator",
          "text": "Bob became CEO of Acme on 2026-03-12."
        }
      ],
      "object_name": "Acme",
      "predicate_name": "CEO_OF",
      "subject": "00000000-0000-4000-8000-000000000302",
      "subject_kind": "Node",
      "subject_name": "Bob",
      "subject_type": "00000000-0000-4000-8000-000000000201"
    }
  ],
  "meta": {
    "drawn": 1,
    "format": "ekr.fact-sample/1",
    "population": 3,
    "revision": 0,
    "seed": 42,
    "size": 1
  }
}
```

| input | what it does |
|---|---|
| `--seed <integer>` | any integer, negative included; the same seed draws the same sample |
| `--size <1–1000>` | how many facts to draw; all of them when the revision holds fewer. Outside the range the verb is refused as `ekr.views.LimitExceeded` (exit 2) before the store is read |
| `--type <type id>` | only facts about nodes or edges of this type, or about the type itself; a type id from `ekr ontology`, compared exactly, so a subtype is another type. A type no fact is about draws nothing (`population` `0`); it is not refused |
| `--revision <revision>` | the committed revision to draw from; the head when absent |

The facts are the revision's `Active` assertions; a retracted or superseded one is not drawn.
Each is ranked by the SHA-256 of `ekr.fact-sample/1:<seed>:<assertion id>`, and the sample is the
`--size` lowest-ranked, listed in that order. So one seed, size, type and revision draw the same
facts on either provider and in every run, a larger size lists the smaller one's facts first, and
a later commit leaves the sample of an earlier revision as it was. `meta.population` counts the
facts the draw chose from and `meta.drawn` those it lists.

Each item carries the assertion as the projection renders it, its subject (`subject_kind`
`Node`, `Edge` or `Type`, `subject`, `subject_type`), the names a judge reads it by —
`subject_name` (a node's canonical name or a type's name), `predicate_name` (the property's or the
relation's name), `object_name` (a node object's canonical name), each left out where there is
none — and every evidence entry the assertion cites, with its bytes: `text` when they are UTF-8,
else `base64`. A store never seeded is refused as `ekr.views.NotSeeded` and a revision it does not
hold as `ekr.views.RevisionNotFound`, exit 2. `ekr session` serves the verb too.

### `ekr fact-quality`

Reads a judged sample and prints its pass rate with its Wilson score interval: the
`ekr.fact-quality/1` document (`ekr.views.ReportFactQuality`). It opens no store, so it needs no
`--host` or `--store`. The judged sample is an `ekr.fact-judgements/1` JSON file, or `-` for
stdin: one judgement per fact you judged from an [`ekr sample`](#ekr-sample), and the sample's
`meta` fields `revision`, `seed`, `size` and `type` under `sample` if you keep them, which the
report echoes and does not check.

```json
{
  "format": "ekr.fact-judgements/1",
  "sample": {"revision": 0, "seed": 42, "size": 3},
  "judgements": [
    {"assertion": "00000000-0000-4000-8000-000000000510", "verdict": "Pass"},
    {"assertion": "00000000-0000-4000-8000-000000000511", "verdict": "Pass"},
    {"assertion": "00000000-0000-4000-8000-000000000512", "verdict": "Fail"}
  ]
}
```

```console
ekr fact-quality judged.json
```

```json
{"meta":{"format":"ekr.fact-quality/1","confidence":9500,"z":1.9599639845400538,"sample":{"revision":0,"seed":42,"size":3}},"judged":3,"passed":2,"failed":1,"rate":0.6666666666666666,"lower":0.20765960080204776,"upper":0.9385080552796037}
```

Unlike the other verbs, which print their document indented with keys in alphabetical order,
`ekr fact-quality` prints the exact bytes the library wrote: one line, keys in the format's own
order. Its numbers are never parsed and printed again, which could move one to a neighbouring
binary64 value. `ekr session` embeds those bytes in its answer as they are.

`verdict` is `Pass` when the evidence supports the fact and `Fail` otherwise. `rate` is
`passed / judged`. `lower` and `upper` are the Wilson score interval at `--confidence`, basis
points from 1 to 9999 (9500, 95 %, when absent), with `z` the standard normal quantile at
`(1 + confidence / 10000) / 2`: `(2k + z² ∓ z·√(z² + 4k(n − k)/n)) / (2(n + z²))` for `k` passed of
`n` judged. `lower` is exactly `0` when nothing passed and `upper` exactly `1` when everything did;
the three are left out when nothing was judged. Compare `lower` with your bar to say, at that
confidence, that the pass rate is above it. The arithmetic uses only the operations IEEE 754
rounds exactly, so every host prints the same numbers.

A confidence outside 1–9999 is refused as `ekr.views.LimitExceeded` and an assertion judged twice
as `ekr.views.JudgedTwice`, naming it, both exit 2. A file that is not an `ekr.fact-judgements/1`
— another format, a missing or unknown key, a verdict other than `Pass` or `Fail` — is a fault,
exit 1. `ekr session` serves the verb too, with the judged sample as the request's `"stdin"`.

### `ekr guide`

Prints the workflow for an agent: roles, propose → validate → commit, exit codes, where ids come from,
how to add evidence after the seed and to a seed, and how to change the schema.

### `ekr operations`

Without an argument, lists the fifteen operation kinds, one per line, marking the four schema
changes (`[schema change: …]`) and the one kind that is not applied (`[not applied: …]`). With a
kind (`ekr operations AddAssertion`), prints its fields and an example operation.

### `ekr example`

Prints a complete document of one format. The three examples fit together: seed a store from the
example seed under the example host, and the example transaction commits. `ekr example
schema-change` prints a schema change against the same seed, which commits in a store seeded under
[validation profile v2](#evolve-the-schema). `ekr example typed-reference` prints a reference that,
against the example seed, resolves to `ProposeNew`: the node `ekr operations CreateNode` creates.
`ekr example ekr.extraction-document/1` prints an [extraction document](#extraction-documents-ekrextraction-document1)
the reader accepts against the example seed's ontology.

### `ekr mint`

Prints a fresh id: `ekr mint node`. Kinds: `node`, `edge`, `assertion`, `transaction`, `evidence`,
`type`, `property`, `agent`, `graph-root`, `schema-version`. Every id is a lowercase, hyphenated
UUID; any UUID in that form is accepted, and `ekr mint` is the easy way to get a fresh one.

### `ekr hash`

Prints the content hash of a payload file (or stdin): `content_hash` is
sha256(`ekr.payload.v1` || bytes) over the exact bytes, trailing newline included; `payload_yaml`
is the same bytes as a YAML list of byte values, ready to paste into `evidence_payloads`.

### `ekr schema`

Prints the JSON Schema (draft 2020-12) of one format, generated from the types the reader decodes:
`ekr schema ekr-seed/2`. It is a first check for an editor or a script, not a verdict: the reader
decides, and `ekr seed` or `ekr propose` can still refuse a document the schema passes, or accept one
it refuses. The YAML formats' schemas validate the document read as YAML and written as JSON, where
a tag `!Kind value` is the one-key object `{"!Kind": value}`; the readers do not accept that object
in place of the tag, so write the tag.

The printed `description` names every place the schema and the reader differ:

| the document holds | reader | schema |
|---|---|---|
| a tag on a scalar, a mapping or a list that names no variant (`proposer: !AgentId <id>`, `valid_time: !Range {…}`) | ignores the tag, accepts | refuses |
| a Float written `.nan` or `.inf` | accepts | refuses (JSON writes it as null) |
| an id or content hash written only in digits, unquoted | accepts | refuses (YAML reads a number); quote it |
| two texts YAML reads as one value in a uniqueItems list (`[true, True]`, `[1, 1.0]`) | accepts | refuses |
| one value twice in a uniqueItems list, including the same text plain and quoted (`[1, "1"]`) | refuses | cannot see it |
| a key written twice | refuses | cannot see it |
| a range or transaction time that ends before it starts | refuses | cannot see it |
| `1.0` where an integer belongs | refuses | cannot see it |
| a value's `value` before its `value_kind` (or `parameters` before `value_kind`), plain number, boolean or null for a text kind | refuses | cannot see it |
| a string or key within maxLength characters but over the byte limit (text outside ASCII) | refuses | cannot see it |
| a transaction document over its format's byte cap (8388608 bytes for `/2`, 262144 for `/1`), nested deeper than 32, with more than its format's values and keys (1048576, `/1` 32768) or text in all (33554432 bytes, `/1` 1048576) | refuses | cannot see it |
| a `ModifyProperty` written as a bare property declaration, without `owner` and `property` (the P1 shape) | accepts; validation then rejects it (`unsupported-operation` under profile v1, `modify-property-without-owner` under v2) | refuses |

### `ekr view`

Serves a read-only viewer of an existing store on 127.0.0.1 — never another address — until the
process is interrupted: `ekr view --port 8080`, or `--port 0` (the default) for a free port. A store
this process may not write it opens read-only, as every verb that reads does, and reads again once
its files change ([Configuration](#configuration)); SIGINT or SIGTERM removes its private copy. It prints one JSON line, `{"url": "http://127.0.0.1:<port>/"}`,
then answers:

| request | answer |
|---|---|
| `GET /` | the viewer page, built into the binary: the graph in 2D and 3D, a timeline with a heatmap and swimlanes, property history, the schema history, a command palette (Ctrl+K), navigation between committed revisions, a compact mode (the Compact button or the key C) that collapses both sidebars to a strip at their edges, with a tab at each edge of the graph collapsing one sidebar and each strip restoring its own, and the state in the URL after `#` (`compact=1`, `compact=left` or `compact=right` while collapsed). A window narrower than 970 px (the two sidebars, 290 + 360 px, and 320 px of graph) opens with both sidebars collapsed unless the address carries `compact`; there the page writes `compact=0` while both are shown, and a reload keeps it; back and forward to an address without `compact` show both. A new detail shown while the right sidebar is collapsed leaves it collapsed and marks its strip with a dot and a title naming what it shows (the detail the reader last saw, drawn again, marks nothing); the strip restores the sidebar showing it. Type chips take the keyboard: Enter or Space hides or shows a type, Shift+Enter or Shift+Space shows only that type (again: every type), and each chip's `aria-pressed` says whether its type is shown; the Compact button carries `aria-pressed`, and the tabs and strips are named in words |
| `GET /head` | `{"format":"ekr.view-head/1","head":N}`, the store's newest committed revision as it stands at the request, `application/json`. No `ekr.views` document carries the head, so a render of a revision is the same bytes before and after any later commit; the page reads the head here. It takes no query (any is 400 `invalid-query`) |
| `GET /projection` | the `ekr.graph-projection/1` document at the head, `application/json`, byte for byte what the projection renders |
| `GET /projection?revision=N` | the same as of revision `N`; a revision the store does not hold is 404 with `{"refusal": "ekr.views.RevisionNotFound", …}` |
| `GET /evidence/<evidence id>` | that evidence's retained bytes: `text/plain; charset=utf-8` when they are UTF-8, otherwise `application/octet-stream`; 404 for an id the head does not hold or bytes the store did not retain |
| `GET /overview[?revision=N&limit=L]` | the `ekr.graph-overview/1` document of revision `N` (the head when absent), listing the `L` highest-degree nodes (1 to 500, 300 when absent), `application/json` |
| `GET /expand?seeds=<id>,<id>&depth=D&limit=L[&edges=E][&after=A][&revision=N]` | the `ekr.graph-slice/1` page of the nodes within `D` hops of the seeds (`D` 0 to 2, at most `L` nodes, 1 to 2,000, and `E` edges, 1 to 5,000, 5,000 when absent, from cursor `A`), streamed as NDJSON (below) |
| `GET /node/<node id>[?revision=N]` | the `ekr.node-detail/1` document of that node, `application/json` |
| `GET /search?q=<text>[&limit=L][&revision=N]` | the `ekr.node-matches/1` document of the nodes whose name or an alias contains the text (at most `L`, 1 to 100, 20 when absent), `application/json` |
| `GET /timeline?[type=<id>&]hops=H&limit=L[&bucket=B][&subject=<id>][&revision=N]` | the `ekr.graph-timeline/1` document: one row per node of the row type `type` (the first the document ranks when absent) with the events related to it within `H` hops (1 to 3) — nodes of an event type, by the one rule in § `ekr ocel` — counted per time bucket, at most `L` rows (1 to 500), the most active first; `B` is the finest bucket, `day` or `week`; with `subject` the row of that node alone and its events; `application/json` |
| `GET /changes?since_revision=N\|since_valid=T\|since_recorded=T[&at=R][&limit=L][&after=A]` | the `ekr.graph-changes/1` page of what changed ([below](#changes-since)) after revision `N`, after valid time `T` or after transaction time `T` (milliseconds since the epoch), up to revision `R` (the head when absent): at most `L` changes (1 to 2,000, 500 when absent) from cursor `A`, `application/json` |

**A property two types define differently.** A type may redeclare a property it inherits with
another name or value kind; a `ModifyProperty` on a child type does. The projection and the
overview then list one `ontology.properties` entry per definition of that id, each naming in
`owners` every type that has that definition, whether it declares it or inherits it, so a type's
definition is found by lookup alone; the entries of one id are ordered by their first owner. A
property every type that has it defines alike has one entry and no `owners`, as before. The page
names a property, and shows its value kind, as the type of the node, edge or type chip shown has
it, and shows the property's id where no such type is known; the schema history names a version's
added property as that version named it.

Any other method is 405 and any other path 404. A request that announces a body (a
`Content-Length` above zero or any `Transfer-Encoding`) is 413; the body is never read. A request
head that does not parse, or is not complete within 16 KiB or 5 seconds of the connection being
accepted, is 400. At most 64 connections are served at once; one more is answered 503 (`busy`) at
once and closed. A request whose `Host` header is not exactly `127.0.0.1:<port>` or
`localhost:<port>` (on port 80 also `127.0.0.1` or `localhost` alone), or that has none, is 421
and is served nothing, so a web page that reaches the port under another name through DNS
rebinding reads nothing. Every response carries `X-Content-Type-Options: nosniff`,
`Cache-Control: no-store` and `Connection: close`, and none sets a cookie or allows another
origin. Evidence text is
never served as HTML. Like every read verb, `ekr view` opens an existing store only (a path holding
none is `store-not-found`, exit 1) and writes nothing to it.

<a id="replaced-store"></a>**A store replaced at its path.** `ekr view`, `ekr mcp` and
`ekr session` open the store when they start and keep it open. Before each request that reads
the store, each compares what is at `--store` now with the store it opened — the device and
inode of the file store's directory or of the SQLite database file, one `stat` of the path and no
read of the store. When a host has replaced the store there, by renaming another one into place,
the next request is answered from the store now at the path, with no restart, and nothing kept
from the replaced store — no loaded revision, no rendered answer — is used again. If what is at
the path does not open as a store (nothing, or a file that is not one), the request is refused
with `store-replaced`, naming the path and why it does not open, and is never answered from the
replaced store; each later request tries again. A file store replaced under the same device and
inode — deleted and created again, or its files replaced inside the directory — passes that
comparison, but the reader's next read finds the history there diverged from the one it holds;
it then opens the store at the path once and answers the request from it. `ekr view` answers
`store-replaced` 503 with `{"refusal": "store-replaced", …}`; `GET /` reads no store and is
served throughout. Move a SQLite database together with its `-wal` and `-shm` files.

The query of `/overview`, `/expand`, `/node/<id>`, `/search`, `/timeline` and `/changes` is
`name=value` pairs joined by `&`, each name one the path takes and at most once, each value
percent-decoded (`+` is a space) to UTF-8; `seeds` is node ids separated by commas, and `seeds`,
`depth` and `limit` are required by `/expand`, `q` by `/search`, `hops` and `limit` by
`/timeline`, and exactly one of `since_revision`, `since_valid` and `since_recorded` by
`/changes`. A query that breaks this, a `type` or `subject` that is not an id, a `bucket` other
than `day` or `week`, or an `at` that is not a revision number, is 400 with
`{"refusal": "invalid-query", …}`.
A `since_revision` below 0 is 400 `ekr.views.SinceMalformed`, and a bound outside its range 400
`ekr.views.LimitExceeded`, both before the store is read; a node or seed the revision does not
hold is 404 `ekr.views.NodeNotFound`, and a revision the store does not hold — an `at`, or a
`since_revision` beyond the head — 404 `ekr.views.RevisionNotFound` (`ekr.views.NotSeeded` for a
store never seeded), each a whole JSON refusal decided before any byte of an answer is sent.

<a id="changes-since"></a>`/changes` answers what the committed revisions up to `at` changed:
every node and edge created, every assertion added, superseded or retracted, and every evidence
entry added by an `!AddEvidence` ([Evidence after the seed](#evidence-after-the-seed)). Each change
carries the revision that made it and that revision's `recorded_at`, its `change`
(`NodeCreated`, `EdgeCreated`, `AssertionAdded`, `AssertionSuperseded`, `AssertionRetracted` or
`EvidenceAdded`), the `id` of the node, edge, assertion or evidence entry, the node's `type` and
`name`, the edge's `type`, `source` and `target`, the assertion's `subject_kind` and `subject` (and
`by` for a supersession), its `valid_time`, the evidence entry's `locator` (its source identity:
the `identity` of its `!HumanStatement`) and `content_hash` (the address of its payload), and
`evidence`: the evidence ids the assertion cites, or for a node or an edge those the same
revision's assertions about it cite, and empty for an evidence entry. The seed's own evidence is
not a change. The changes are ordered by revision, then change kind in that order, then id:

- `since_revision=N` chooses the changes of the revisions after `N`; one at or after `at` chooses
  none;
- `since_recorded=T` chooses the changes of the revisions committed after `T`, the seed's
  included when it was;
- `since_valid=T` chooses the assertion changes whose valid time is after `T` — an added or
  retracted assertion's `valid_from`, a supersession's `effective_from` — from every revision up
  to `at`. A node, an edge or an evidence entry has no valid time, and is never chosen by one.

`meta` echoes the since, `revision` (the revision read), `limit`, `after` and `total`; `next`,
present while changes remain, is the next page's `after`, and `remaining` counts what is left.
The answer never names the head: the same request naming the same `at` is the same bytes after
any later commit, and a request without `at` is the one naming the revision `meta.revision`
reports, so a reader that pages passes that as `at`. A property update, a named operation, an
edge deletion and a schema change are not changes here.

`/expand` answers `application/x-ndjson` with `Transfer-Encoding: chunked`: one JSON object per
line, each ending in `\n` and opening with its `kind`, flushed a chunk at a time as it is written:

1. `{"kind":"meta", …}` — the page's `ekr.graph-slice/1` meta (`format`, `revision`, `seeds`,
   `depth`, `after`, `node_total`, `edge_total`), alone in the first chunk;
2. the records in the slice's order: `{"kind":"node", …}` with a node's fields and
   `{"kind":"edge", …}` with an edge's, a node always before the edges it closes;
3. after every 256 records, `{"kind":"progress","sent":<records so far>}`, which ends its chunk;
4. last, `{"kind":"end","next":<cursor>|null,"remaining":<records after this page>}`; ask again
   with `after=<next>` for the next page.

The page is computed before the first byte, so a refusal is never sent mid-stream. The stream is
written from its own connection, never from the thread that reads the store, and counts as one of
the 64 connections; each write waits at most 5 seconds and the whole stream at most 60, and a
client that closes the connection ends it and frees its place.

A revision is loaded once. The first request of a revision to `/overview`, `/expand`, `/node`,
`/search`, `/timeline`, `/changes`, `/projection` or `/roles` loads and indexes it; every later request of
that revision, whichever path, reads the store's head and answers from that index, so it costs its
answer (a revision's first `/timeline` also ranks its row types once; `/changes` also reads the
store's retained transaction records and parses the transactions of the revisions it chooses,
and replays the seed only when it chooses the seed). The
indexes of the 3 revisions used most recently are kept. A committed revision never changes and no
answer names the head, so a commit loads nothing again; a request naming no revision reads the
new head. `/projection` and `/roles` also keep their rendered answers, byte for byte what the first
answer was, for at most 8 revisions, the one used longest ago going first.

The page reads `/head`, `/overview`, `/expand`, `/node/<id>`, `/search`, `/timeline` and
`/evidence/<id>` and nothing else, never `/projection`: the overview once per revision and the
head after it, a neighbourhood as it streams in, a node's detail when it is opened, and the
timeline's rows — one per subject of the chosen row type, with its events within the chosen
hops — and a subject's swimlanes from `/timeline`. It fetches evidence only by an id an assertion
it has read cites, and writes everything a store holds as text.

#### Roles

`GET /roles` (the head) and `GET /roles?revision=N` answer, as `application/json`, which node types
the viewer lays out as events, subjects and observations:

```json
{"format":"ekr.view-roles/1","revision":0,"node_types":[{"type_id":"<type id>","role":"event"}]}
```

`role` is `event`, `subject` or `observation`; entries are ordered by `type_id`, and a type the rule
below does not place has no entry (the viewer still shows it). The body is computed from the same
loaded revision `/projection` renders and is not part of `ekr.graph-projection/1`. It is refused
exactly as `/projection` is: 404 `ekr.views.RevisionNotFound` for a revision the store does not
hold, 400 `invalid-query` for any query but exactly `revision=N` with `N` in ASCII decimal
digits (an empty pair, a second pair, a sign or a space is refused).

The rule reads the store's event types, its observation type and its edge types and nothing else
— never a type, edge-type, property or entity name — so a store whose every name is changed gets
the same roles, id for id. To apply it by hand to the revision:

1. **Event types.** The event types are EKR's one event-type rule, given in § `ekr ocel`: a type
   is an event type when at least 60% of its judged nodes are instant, that is, carry the
   timeline's time at one moment — a timestamp-like property value, or dated facts that lie
   within one hour. They are the types the overview marks `roles.types[].event` and the timeline
   and `ekr ocel` treat as events, at the same revision. The *observation type* is the
   overview's `roles.observation_type`, one of the event types, the one the viewer lays out as the
   observation (ties go to the lowest type id).
2. **Arcs.** First widen each edge type's `source_types` and `target_types` to every node type that
   conforms to one of them: the listed types and all their descendants through `parents`,
   transitively, which is how the runtime checks an edge's endpoints. Each edge type then gives an
   arc from every widened source type to every widened target type; a `symmetric` edge type gives
   the reverse arcs too. An arc from a type to itself is dropped. Abstract types count like any
   other.
3. **Sources.** A type's *sources* are the other types with an arc to it.

Each node type then takes the first role whose condition holds:

| role | condition |
|---|---|
| `observation` | the observation type |
| `event` | any other event type |
| `subject` | at least one source |
| none | anything else: no entry |

So the `event` and `observation` entries together are exactly the event types the overview, the
timeline and `ekr ocel` use, and the `observation` entry is the overview's observation type. An
event type is an `event` or the `observation` whatever its arcs; a type that is no event type and
that no other type points at after widening has no role.

Before the one event-type rule, `/roles` judged events by its own structural rule: a type with any
valid-time assertion that pointed at another type was an `event`, and a type nothing pointed at
that pointed at one was an `observation`. Now a node type counts as an event type only by the
timeline's rule above. A store whose nodes carry one dated fact each, or whose dated facts lie
more than an hour apart, has no event type at all: its types lose their `event` and `observation`
roles, and keep `subject` where another type points at them.

Roles move both ways across revisions. A revision that adds a dated fact can make a type an event
type (a node's second dated fact, within an hour of its first) or stop it being one (a dated fact
more than an hour from the others makes its node judged but not instant), and the observation type
can move to another event type as the overview's choice among them changes.

### `ekr session`

Serves the verbs that print one JSON document through one process that opens the store once. A
host that makes many calls — resolve, mint, propose, validate, commit, over and over — pays for
opening and verifying the store once instead of once per call. The session reads the
configuration (`--host`, `--store`, `--backend`, `--full-replay` or their variables) and opens the
store when it starts — or starts without one where there is none yet, as below — then reads
standard input one line at a time until it ends. Each line is one request:

```console
{"argv": ["resolve", "reference.yaml"]}
{"argv": ["propose", "-"], "stdin": "format: ekr.transaction-document/2\ntransaction: ..."}
{"argv": ["commit", "00000000-0000-4000-8000-000000000902"]}
```

- `"argv"` — a verb and its arguments, exactly as `ekr` takes them after its name, parsed by the
  same definitions: `["snapshot", "--at", "0"]` is `ekr snapshot --at 0`. Relative paths are read
  from the session's working directory.
- `"stdin"` — optional text that `-` reads, for `propose -`, `resolve -` and `hash -`; when absent,
  `-` reads nothing.

Each request is answered by one line on standard output, written and flushed before the next
request is read, in request order:

```console
{"exit":0,"stdout":{"kind":"Resolved","node_id":"00000000-0000-4000-8000-000000000901"},"stderr":""}
{"exit":2,"stdout":null,"stderr":"ekr: ekr.kernel.TransactionNotFound: transaction 00000000-0000-4000-8000-000000000999 does not exist\n"}
```

- `"exit"` — the exit status the one-shot verb would have returned (0, 1 or 2, [as above](#exit-codes-and-output)).
- `"stdout"` — the JSON document the one-shot verb prints, as a JSON value: printed pretty with a
  newline, it is byte for byte what the verb prints. `null` when the verb printed nothing.
- `"stderr"` — the text the one-shot verb writes to stderr, newline included: a named refusal, a
  fault or clap's usage message. Empty when it wrote nothing.

Every verb runs against the store as it stands when the request is read, so a transaction a
request commits is what the next `head`, `snapshot` or `resolve` reads, and so is one another
process committed. A write goes through `propose`, `validate` and `commit` exactly as it does one
verb at a time. The session exits 0 when its input ends, after writing the store's replay
checkpoint of the newest head it reached if that head is past the last checkpoint, so that the
next verb does not replay the commits made since. A request never ends it: a request the
verb refuses, and a line that is not a request, are answered and the next line is read. If the
configuration does not resolve, or an existing store does not open, the session answers nothing
and exits as a store verb does. A path holding no store is not such a failure:

**Before a store exists.** A session also starts on a `--store` that holds no store yet, and
creates nothing there by starting. It serves `mint`, `hash` and `schema` exactly as the one-shot
verbs do, and answers each store verb as the one-shot verb answers on that path —
`store-not-found`, `"exit": 1` — then reads the next line. Started as `ekr session --create`, it
also serves `seed`, with the arguments and document `ekr seed` takes: `--evidence <file>`,
repeatable, and `-` reading the request's `"stdin"`. The seed that creates the store leaves the
session holding it, opened once as a session opens an existing store when it starts, and every
verb after it is served over that store. A seed on a store that exists — a second seed in the
same session, or one in a `--create` session started on an existing store — answers what
`ekr seed` answers there: the original result for the same document, `ekr.kernel.AlreadySeeded`
for another. So a host building a store sends every `hash` and `mint`, the `seed` and the first
writes through one process instead of one each:

```console
{"argv": ["hash", "corpus/a.md"]}
{"argv": ["mint", "node"]}
{"argv": ["seed", "-", "--evidence", "corpus/a.md"], "stdin": "format: ekr-seed/2\n..."}
{"argv": ["propose", "first.yaml"]}
```

A session serves `propose`, `validate`, `commit`, `snapshot`, `explain`, `resolve`, `head`,
`transactions`, `rejections`, `ontology`, `quality`, `ocel`, `sample`, `fact-quality`, `mint`, `hash` and `schema`, the `ekr.views` reads
([below](#session-views)), and `seed` when it was started with `--create`. It refuses these, each
answered with `"exit": 2`, `"stdout": null` and `ekr: <refusal>: <reason>` as `"stderr"`:

| refusal | exit | what it means | what to fix |
|---|---|---|---|
| `session-request-malformed` | 2 | the line is not a JSON object with `argv`, a list of strings, and at most `stdin`, a string; an empty line included | send `{"argv": [...]}` on one line |
| `session-request-too-large` | 2 | the line is longer than 25231360 bytes, its newline excluded: three times the 8388608-byte `ekr.transaction-document/2` cap, the most JSON escaping can make of it, and 65536 bytes for `argv` and the framing. The session holds no more of the line than that; it reads the rest up to the newline, drops it and serves the next line | send the document as a file (`["propose", "doc.yaml"]`), or a smaller one |
| `session-verb-unknown` | 2 | `argv` is empty, or its first word is neither a verb of `ekr` nor one of the `ekr.views` reads below | a verb from the list above |
| `session-verb-refused` | 2 | the verb is `seed` in a session started without `--create`, or `view`, `session`, `mcp`, `migrate`, `guide`, `operations` or `example`, or the request asks for help — the `help` verb (`["help"]`, `["help", "head"]`), `--help` or `--version`: these create a store, serve until interrupted or until their own input ends, nest, write a second store, or print text | run it as its own `ekr` process; for `seed`, or start the session as `ekr session --create` |
| `session-option-refused` | 2 | the request sets `--host`, `--store`, `--backend` or `--full-replay` | the session's store is fixed when it starts; start another session for another store |

A session opens its store as a verb that reads does, so on a store this process may not write it
starts, holds the store read-only, reads it again once its files change, and serves every read; a `propose`, `validate`, `commit` or
`seed` request is answered `"exit": 2` with `ekr: store-read-only: <reason>`, as the one-shot verb
is refused ([Common refusals](#common-refusals)), and the session serves the next line.

Any other `argv` the verbs' definitions do not accept — an unknown flag
(`["--sto", "x", "head"]`, `["head", "-V"]`), a missing argument, a value of the wrong kind
(`["snapshot", "--at", "zero"]`) — is answered as the one-shot verb answers the same argv:
`"exit": 2`, `"stdout": null` and clap's usage message, byte for byte, as `"stderr"`: the
message `ekr <argv>` prints with the store configured through `EKR_HOST`, `EKR_STORE` and
`EKR_BACKEND`, since clap's usage line repeats the global options an argv gives.

<a id="session-views"></a>**The `ekr.views` reads.** A session also serves six verbs that `ekr`
has no one-shot verb for: the bounded reads [`ekr view`](#ekr-view) serves over HTTP and
[`ekr mcp`](#ekr-mcp) as tools. Each answers, as its `"stdout"`, the document `ekr view` serves on
the route in the last column for the same query at the same revision, read through the same
`ekr.views` index, which the session loads once per revision of the store it holds and keeps for
the 3 revisions used most recently:

| verb | arguments (required in bold) | answers | as |
|---|---|---|---|
| `overview` | `--revision N`, `--limit L` (1 to 500, 300 when absent) | the `ekr.graph-overview/1` document | `GET /overview` |
| `search` | **the text** (empty matches every node; put `--` before a text that starts with `-`), `--limit L` (1 to 100, 20 when absent), `--revision N` | the `ekr.node-matches/1` document | `GET /search` |
| `describe` | **the node id**, `--revision N` | the `ekr.node-detail/1` document | `GET /node/<node id>` |
| `expand` | the seed node ids (none answers an empty page), **`--depth D`** (0 to 2), **`--limit L`** (1 to 2,000 nodes), `--edges E` (1 to 5,000, 5,000 when absent), `--after A` (a cursor), `--revision N` | the whole `ekr.graph-slice/1` page as one document, `next` naming the next page's `--after` | `GET /expand`, as one document rather than NDJSON |
| `timeline` | `--type T` (a node type id), **`--hops H`** (1 to 3), **`--limit L`** (1 to 500), `--bucket day\|week`, `--subject S` (a node id), `--revision N` | the `ekr.graph-timeline/1` document | `GET /timeline` |
| `changes` | exactly one of **`--since-revision N`**, **`--since-valid T`** and **`--since-recorded T`** (milliseconds since the epoch), `--at R` (the last revision read, the head when absent), `--limit L` (1 to 2,000, 500 when absent), `--after A` (a cursor) | the `ekr.graph-changes/1` page ([what it lists](#changes-since)); pass the first page's `meta.revision` as `--at` for the rest | `GET /changes` |

```console
{"argv": ["search", "Globex", "--limit", "5"]}
{"argv": ["expand", "00000000-0000-4000-8000-000000000301", "--depth", "1", "--limit", "500"]}
{"argv": ["changes", "--since-revision", "0"]}
```

Every read is of the store as it stands when the request is read, so a transaction this session
or another process committed is what the next one reads, and a request naming no revision reads
the newest. What `ekr view` refuses with a JSON `{"refusal", "message"}` body a session answers
`"exit": 2` with `ekr: <refusal>: <message>`, the same name and message, in the same order: a
`--since-revision` below 0 is `ekr.views.SinceMalformed` and a bound outside its range
`ekr.views.LimitExceeded`, both before the store is read; a store never seeded
`ekr.views.NotSeeded`; a revision the store does not hold `ekr.views.RevisionNotFound`; and a node
or seed the revision does not hold, or a `describe` id that is no node id,
`ekr.views.NodeNotFound`. A query `ekr view` answers `invalid-query` — a bound that is not an
optional `-` and ASCII digits, a revision that is not ASCII digits (so `+5` is neither), a seed,
`--type` or `--subject` that is not an id, a `--bucket` other than `day` or `week`, no since or
more than one — is an argv these verbs do not take: clap's usage message, `"exit": 2`. On a path
holding no store each answers `store-not-found`, as every store verb does; once another process
has created a store there, the first views verb opens it and the session holds it from then on,
as after its own seed, so its indexes are loaded once per revision there too.

**A store replaced at its path** ([as for `ekr view`](#replaced-store)) is followed before each
store verb; `mint`, `hash` and `schema` read no store and are served throughout. A store now at
the path that does not open is answered `"exit": 1` with `ekr: store-replaced: <reason>`, a fault
as `store-not-found` is. A session holding a transaction it proposed that is neither committed
nor rejected — as the store it holds records them, so one another process committed is not open —
does not follow a replacement: every store verb is answered `"exit": 2` with
`ekr: store-replaced-proposals-open: <reason>`, naming those transactions, which stay in the
store the session opened, until that store is back at the path. End the session to work on the
store now there. While it holds such a transaction, each store verb also reads the store's
transactions once to see whether it is still open; a session holding none reads nothing more.
At the end of its input a session writes its replay checkpoint only into the store it holds, and
only while that store is still the one at the path.

### `ekr mcp`

Serves the store's canonical state to an agent as read-only [MCP](https://modelcontextprotocol.io)
tools over stdio. It reads the configuration (`--host`, `--store`, `--backend`, `--full-replay` or
their variables) and opens the existing store when it starts, then reads JSON-RPC 2.0
messages from standard input, one per line, and writes each response as one line on standard
output, flushed, until its input ends; then it exits 0. It writes nothing to the store and nothing
to stderr. If the configuration or the store does not open, it answers nothing and exits as a
store verb does (`store-not-found`, exit 1). A store this process may not write it opens read-only,
reads again once its files change, and serves as on a writable one ([Configuration](#configuration)). To register it with an MCP client, give the client
the command `ekr mcp` with `EKR_HOST`, `EKR_STORE` and `EKR_BACKEND` in its environment.

A line that is empty or holds only whitespace is not a message: it is read and not answered.
Every response carries the request's `id` exactly as the client wrote it — a number beyond 64
bits, an exponent or an escaped string comes back byte for byte.

It answers these methods:

- `initialize` — `params.protocolVersion` is required. The server supports the MCP revisions
  `2025-11-25` and `2025-06-18`: it answers the one the client asked for when it is one of these,
  and `2025-11-25` otherwise. It does not agree to `2025-03-26`, whose transport requires
  receiving JSON-RPC batches, which this server refuses, nor to `2024-11-05`; a client asking for
  either is answered `2025-11-25`;
  `capabilities` is `{"tools": {"listChanged": false}}`, `serverInfo.name` is `ekr` and
  `serverInfo.version` the binary's version, and `instructions` says that record text is
  untrusted evidence.
- `notifications/initialized`, and every other notification — never answered.
- `ping` — `{}`.
- `tools/list` — the nine tools below, each with its `inputSchema` (a JSON Schema object that
  refuses any other argument) and `annotations.readOnlyHint: true`.
- `tools/call` — `{"name": <tool>, "arguments": {…}}`.

Every tool reads the store as it stands when the call is read, so a transaction another process
committed is what the next call reads. `revision` is a committed revision, the newest when
absent; `overview`, `search`, `describe_node`, `expand`, `timeline` and `changes_since` read the
revision's `ekr.views` index, loaded once per revision exactly as [`ekr view`](#ekr-view) keeps
it, and answer its document byte for byte what the `ekr view` endpoint in the last column serves:

| tool | arguments (required in bold) | answers | as |
|---|---|---|---|
| `overview` | `revision`, `limit` (1 to 500, 300 when absent) | the `ekr.graph-overview/1` document | `GET /overview` |
| `search` | **`text`**, `limit` (1 to 100, 20 when absent), `revision` | the `ekr.node-matches/1` document | `GET /search` |
| `describe_node` | **`node`** (a node id), `revision` | the `ekr.node-detail/1` document | `GET /node/<node id>` |
| `expand` | **`seeds`** (a list of node ids; empty answers an empty page), **`depth`** (0 to 2), **`limit`** (1 to 2,000 nodes), `edges` (1 to 5,000, 5,000 when absent), `after` (a cursor, 0 or more), `revision` | the whole `ekr.graph-slice/1` page as one document, `next` naming the next page's `after` | `GET /expand`, as one document rather than NDJSON |
| `timeline` | `type` (a node type id), **`hops`** (1 to 3), **`limit`** (1 to 500), `bucket` (`day` or `week`), `subject` (a node id), `revision` | the `ekr.graph-timeline/1` document | `GET /timeline` |
| `changes_since` | exactly one of `since_revision` (a revision), `since_valid` (a valid time) and `since_recorded` (a transaction time), both times in milliseconds since the epoch; `at` (the last revision read, the head when absent), `limit` (1 to 2,000, 500 when absent), `after` (a cursor, 0 or more) | the `ekr.graph-changes/1` page ([what it lists](#changes-since)), `next` naming the next page's `after`; pass the first page's `meta.revision` as `at` for the rest | `GET /changes` |
| `explain` | **`assertion`** (an assertion id), `documents` (`true` or `false`, `false` when absent) | what `ekr explain <assertion>` prints, byte for byte, or with `documents: true` what `ekr explain <assertion> --documents` prints | `ekr explain [--documents]` |
| `resolve` | **`type_id`** (a string), **`aliases`** (a list of strings) — the [`typed-reference`](#ekr-resolve) document's fields, taken as the JSON strings hold them, every character included — and `at` (a revision) | what `ekr resolve` prints for that reference, byte for byte | `ekr resolve [--at N]` |
| `head` | none: any argument is -32602 | `{"format":"ekr.view-head/1","head":N}`, the newest committed revision as it stands at the call, byte for byte what `GET /head` serves; `ekr.views.NotSeeded` for a store never seeded | `GET /head` |

A tool's answer is a result with one text content item holding the document, the same document
parsed as `structuredContent`, and `"isError": false`:

```console
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"describe_node","arguments":{"node":"00000000-0000-4000-8000-000000000301"}}}
{"id":3,"jsonrpc":"2.0","result":{"content":[{"text":"{\"format\":\"ekr.node-detail/1\",…}","type":"text"}],"isError":false,"structuredContent":{"format":"ekr.node-detail/1",…}}}
```

A refusal is a result too, with `"isError": true` and the document `{"message": <reason>,
"refusal": <name>}` — the body `ekr view` answers for the same refusal, and the name and reason
`ekr explain` and `ekr resolve` write to stderr. The server keeps serving after it. The refusals,
in the order they are decided: a `since_revision` below 0 is `ekr.views.SinceMalformed` and a
bound outside its range `ekr.views.LimitExceeded`, both before the store is read; a store
replaced at the path by one that does not open `store-replaced` ([as for `ekr view`](#replaced-store));
a store never seeded `ekr.views.NotSeeded`; a revision the store does not hold — for
`changes_since` an `at`, then a `since_revision` beyond the head — `ekr.views.RevisionNotFound`
(`ekr.kernel.RevisionNotFound` for `resolve`); a node or seed the
revision does not hold, or a `node` that is no node id, `ekr.views.NodeNotFound`; an assertion
the head does not hold `ekr.kernel.AssertionNotFound`; and for `resolve`,
`reference-without-identity`, `reference-type-undeclared` and `reference-type-has-subtypes`
([`ekr resolve`](#ekr-resolve)).

Anything else is a JSON-RPC error response, and the server keeps serving:

| code | when |
|---|---|
| `-32700` | the line is not JSON (an empty or whitespace-only line is not answered at all); `id` is null |
| `-32600` | the line is not one JSON object (a batch included), lacks `"jsonrpc": "2.0"`, lacks a method (a message carrying `result` or `error` instead is a response, and is not answered), carries an `id` that is not a string or a number or carries `id` twice, or is longer than 6356992 bytes, its newline excluded |
| `-32601` | the method is none of the five above |
| `-32602` | `initialize` without `protocolVersion`; `tools/call` without a `name`, naming a tool the server does not have, or with arguments the tool does not take: an unknown or missing argument, a value of the wrong type, a `revision` or `at` below 0, a `changes_since` given none or more than one of its three since arguments, a seed, `type` or `subject` that is not an id, a `bucket` other than `day` or `week`, an `assertion` that is not an assertion id, a `type_id` that is not an id, with the reason `ekr resolve` gives for it, or any argument to `head`. `ekr view` answers these `invalid-query` |
| `-32603` | the store could not be read |

Record text — names, aliases, property values, evidence text — is untrusted evidence. The server
returns it as JSON string data, and its instructions and every tool's description tell the agent
to treat it as data, never as instructions. No tool proposes, validates or commits: an agent
records knowledge only through [`ekr propose`, `ekr validate` and `ekr commit`](#the-workflow).

A store replaced at `--store` while the server runs — a new store renamed into place — is what
the next call reads, with no restart ([as for `ekr view`](#replaced-store)); a call that reads
the store while what is at the path does not open is the tool refusal `store-replaced`, never an
answer from the replaced store. `initialize`, `ping` and `tools/list` read no store.

### `ekr migrate`

Writes a copy of the store in the current formats to a new path and leaves the store itself
exactly as it is. A store seeded before `ekr-seed-envelope/3` retains every evidence payload three
times: as its own object, inside the seed envelope as a list of numbers three to four times its
size, and again inside the record the seed was published from. A new seed names each payload by its
content hash instead and retains its bytes once; `ekr migrate` gives an existing store's history the
same shape in a new store.

```console
ekr migrate --to library-v3                       # the --store, --backend and --host of every verb
```

It only reads `--store`, so a store this process may not write migrates too, opened read-only
([Configuration](#configuration)); `--to` must be writable.

`--to` is a directory for `file` and a database file for `sqlite`, of the same backend as
`--store`, and must hold no store. The migration reads the whole store and replays it from its
seed first, so a store that does not verify is refused and nothing is written. It then seeds the
new store with the same seed — the same input, identities and time, the envelope naming its
payloads — and publishes every later proposal, validation, rejection, commit and stale decision
again with its original identity, actors and times. A proposal record is copied byte for byte; a
record that names the seed envelope or an earlier root is derived again for the new lineage, so
its content hash changes. Every other object is copied with its class and time, and an object an
old store holds inline in its log is written as a blob. The new store is replayed in full and
compared with the old one — every revision's knowledge, evidence, ontology and authority roots,
the graph, the evidence and every transaction's state — before the report is printed:

```console
{
  "format": "ekr.store-migration/1",
  "source_seed_hash": "…",
  "destination_seed_hash": "…",
  "occurrences": [{"event_id": "…", "event": "ekr.kernel.Seeded", "source_record_hash": "…", "destination_record_hash": "…"}, …],
  "carried_objects": ["…"],
  "legacy_objects": [],
  "map_hash": "…"
}
```

The report is also kept in the new store, at `map_hash`. Nothing is ever written to `--store`.
Its refusals are faults (exit 1). These are refused before any event is written to `--to`, so a
later `ekr migrate` can still write there: `migrate-destination-not-empty` for a `--to` that holds
a store; `migrate-destination-is-source`, `migrate-destination-inside-source` and
`migrate-destination-contains-source` for a `--to` that is `--store` (a symbolic link to it or,
for `sqlite`, one of its side files included), lies inside it, or contains it — `--to` is resolved
through its nearest existing directory, and nothing is created; `migrate-unresolved-preparation`
for a decision a command elected and never published — run that command again first — and a
`--store` that does not replay.

A migration that stops after it began writing — `migrate-verification-disagrees`, a new store
that does not replay to the old one's state, a full disk, an interrupted process — leaves the store
at `--to` as it is, and that store is not the migrated one: the migration marks it as begun before
its first write and as finished after its last, the report included, and every verb refuses a
store marked begun and not finished as `migrate-incomplete` (exit 1). A second `ekr migrate` to
the same path refuses it as `migrate-destination-not-empty`: remove it, then migrate again.
`ekr migrate` is not served in `ekr session`.

## The workflow

```console
ekr example ekr.cli-host/1 > host.json        # or write your own, see above
ekr seed seed.yaml                              # revision 0: schema, graph, evidence
ekr ontology                                    # type and property ids
ekr mint assertion                              # fresh ids for what you create
ekr propose change.yaml                         # -> transaction_id, state Proposed
ekr validate <transaction_id>                   # -> kind Validated or Rejected
ekr commit <transaction_id>                     # -> kind Committed, result.revision
ekr head                                        # the new revision
ekr snapshot --valid-at 2026-01-01              # what is believed at that date
ekr explain <assertion_id>                      # why; --documents for the whole records
```

Where values come from:

| value | source |
|---|---|
| a new id | `ekr mint <kind>`; ids are never derived from names |
| an existing type, edge type or property id | `ekr ontology` |
| the graph root, node, edge, assertion and evidence ids | `ekr snapshot` |
| a revision number | `ekr head`; a commit prints its revision |
| a time | milliseconds since the Unix epoch, UTC |
| a content hash | `ekr hash <file>` |

## The seed document (`ekr-seed/2`)

A seed is a YAML document with four top-level fields. It declares the first version of the schema.
Under validation profile v1, the example host's, **the schema cannot change after seeding**; under
profile v2 a transaction can add types and add or redeclare properties
([Evolve the schema](#evolve-the-schema)), but nothing removes a type or a property. Design the
schema before you seed.

| field | content |
|---|---|
| `format` | exactly `ekr-seed/2` |
| `ontology` | the schema: [the ontology section](#the-ontology-section) |
| `graph` | the initial graph: [the graph section](#the-graph-section) |
| `evidence_payloads` | the evidence bytes, keyed by content hash: [evidence](#evidence-and-evidence_payloads) |

Every object in the seed refuses unknown fields; a misspelt field name is an
`ekr.kernel.InvalidSeed` naming the field, the fields it expected and the line. Maps keyed by id
refuse duplicate keys. Comments are ignored.

### The ontology section

| field | content |
|---|---|
| `version` | `{id, number, parent, created_at}`: a fresh schema-version id, `number: 0`, `parent: null`, `created_at` in milliseconds (0 is fine) |
| `node_types` | a list of [node types](#node-types) |
| `edge_types` | a list of [edge types](#edge-types) |

The ontology is checked as a whole before anything else; a document whose declarations do not cohere
is refused as `seed-ontology` with the reason. The rules:

- every type id (node and edge together) is declared once;
- every type named anywhere — a parent, an endpoint, an `inverse`, a `NodeRef`'s allowed types — is
  declared in this ontology;
- parents form no cycle;
- a property is filed under its own id (the map key equals `id`);
- a `NodeRef` allows at least one type and an `Enum` has at least one variant, at every depth;
- an edge type has at least one source type and one target type;
- two parents that do not specialise each other may not declare one property differently;
- a lifecycle names only its own states, and an operation's `transition` is one its type's lifecycle
  declares.

Not checked: that an operation's `name` equals its map key. Keep them equal yourself; `!Invoke` uses
the key.

### Node types

| field | type | meaning |
|---|---|---|
| `id` | type id | the type's identity; stable forever |
| `name` | string | the name a reader sees. Not an identity: two types may share it, and nothing refers to a type by name |
| `parents` | list of node type ids | types this one specialises. A node carries its parents' properties too, and a node of this type is accepted wherever a parent type is allowed (endpoints, `NodeRef`). Default `[]` |
| `properties` | map property id → [property definition](#property-definitions) | the properties this type declares itself. Default `{}` |
| `abstract_type` | bool | an abstract type has no nodes; it exists to be a parent. Default `false` |
| `lifecycle` | [lifecycle](#lifecycles-and-named-operations) or `null` | the states a node of this type moves through. Default `null` |
| `operations` | map name → [operation](#lifecycles-and-named-operations) | the named operations `!Invoke` may call. Default `{}` |

### Property definitions

| field | type | meaning |
|---|---|---|
| `id` | property id | the property's identity; equal to its map key |
| `name` | string | the name a reader sees |
| `value_type` | [value type](#value-types) | what every value must be |
| `cardinality` | `One` or `Many` | how many values a node may carry. Default `One` |
| `required` | bool | whether a node must carry at least one value. Default `false`. Checked when a node is created and on every update |
| `constraints` | list of strings | reserved. P1 has no constraint evaluator: any write touching a type with a non-empty `constraints` list is refused as `unsupported-constraint`. Leave it `[]` |

### Value types

A value type is written with `value_kind` and, for compound kinds, `parameters`. A value is written
with the same `value_kind` and a `value`. These are all eleven kinds:

| kind | declared as | a value is written as | notes |
|---|---|---|---|
| `String` | `{value_kind: String}` | `{value_kind: String, value: Lichens}` | text |
| `Boolean` | `{value_kind: Boolean}` | `{value_kind: Boolean, value: true}` | |
| `Integer` | `{value_kind: Integer}` | `{value_kind: Integer, value: 212}` | a signed 64-bit whole number |
| `Float` | `{value_kind: Float}` | `{value_kind: Float, value: 310.5}` | may be declared, but **no Float value can be committed**: it is refused as `inadmissible-value`, at any depth. Use `Integer` in a fixed unit or `Decimal` |
| `Decimal` | `{value_kind: Decimal}` | `{value_kind: Decimal, value: "24.90"}` | a number meant to be exact, held as a string. Quote it. P1 does not check that the string is a number: any text is accepted and compared as text, so `"24.90"` and `"24.9"` are different values. Write one canonical form |
| `Timestamp` | `{value_kind: Timestamp}` | `{value_kind: Timestamp, value: 1554076800000}` | milliseconds since the Unix epoch, UTC |
| `Duration` | `{value_kind: Duration}` | `{value_kind: Duration, value: 21600000}` | a signed whole number; the runtime does not interpret its unit, so state one (milliseconds is the convention) |
| `NodeRef` | `{value_kind: NodeRef, parameters: {allowed_types: [<node type id>, ...]}}` | `{value_kind: NodeRef, value: <node id>}` | the node must exist and be of an allowed type or a subtype of one |
| `Enum` | `{value_kind: Enum, parameters: {variants: [a, b]}}` | `{value_kind: Enum, value: a}` | one of the declared variants, case-sensitive |
| `List` | `{value_kind: List, parameters: <element value type>}` | `{value_kind: List, value: [<value>, ...]}` | one value that is an ordered sequence; each element is a full value |
| `Record` | `{value_kind: Record, parameters: {<field>: <value type>, ...}}` | `{value_kind: Record, value: {<field>: <value>, ...}}` | exactly the declared fields, no more and no fewer |

`List` and `Record` nest: a list of records, a record with a list field. In block YAML the same
forms are:

```yaml
value_type:
  value_kind: Record
  parameters:
    height:
      value_kind: Integer
    width:
      value_kind: Integer
```

A property's values are always a list: `properties: {<property id>: [<value>, ...]}`. `cardinality`
bounds the length of that list; a `List` value is a single entry in it. An empty list is refused in
a seed (leave the property out instead); in `!UpdateProperty`, `values: []` clears the property.

### Edge types

| field | type | meaning |
|---|---|---|
| `id` | type id | the edge type's identity. It is also the relation id an assertion uses: `predicate: !Relation <id>` |
| `name` | string | the name a reader sees, for example `WROTE` |
| `source_types` | list of node type ids, not empty | the types an edge may start at (subtypes accepted) |
| `target_types` | list of node type ids, not empty | the types an edge may end at (subtypes accepted) |
| `cardinality` | `One` or `Many` | how many edges of this type may leave one source node. Checked for `!CreateEdge`, not for relation assertions. Default `One` |
| `properties` | map property id → property definition | properties an edge of this type carries |
| `inverse` | edge type id or `null` | the edge type that reads this one backwards. Must be declared; P1 records it and derives nothing from it |
| `symmetric` | bool | the relation holds both ways. Recorded; P1 derives nothing from it |
| `transitive` | bool | the relation composes with itself. Recorded; P1 derives nothing from it |

### Lifecycles and named operations

A lifecycle gives the nodes of a type a state that only named operations move.

| lifecycle field | meaning |
|---|---|
| `initial` | the state a new node is in; one of `states` |
| `states` | every state, as strings |
| `transitions` | a list of `{from, to}` moves; a pair not listed is not a move |

A node of a type with a lifecycle carries its state in `type_state`. In a seed, write the `initial`
state there; a node of a type without a lifecycle has `type_state: null`. A node created by a
transaction starts in `initial`.

| operation field | meaning |
|---|---|
| `name` | the operation's name. Write it equal to its map key: `!Invoke` finds an operation by its **map key**, never by `name`, and the seed does not check that the two agree, so a `name` that differs from its key is accepted and then cannot be used to invoke anything |
| `arguments` | map argument name → value type. An invocation must pass every argument, and only these, each of its declared type |
| `preconditions` | reserved; must be `[]`. An operation with preconditions is refused when invoked (`unsupported-constraint`) |
| `transition` | `{from, to}` or `null`. Invoking moves the node from `from` to `to`; a node in any other state is refused (`transition-refused`) |
| `emits` | reserved; must be `[]` for the operation to be invocable |

In P1 an invocation changes only the node's `type_state`; its arguments are type-checked and
recorded with the transaction.

### The graph section

```yaml
graph:
  format: ekr.graph-document/2
  graph:
    root: {...}
    revision: 0
    nodes: {...}
    edges: {...}
    assertions: {...}
    evidence: {...}
```

All five maps are required; write `{}` for an empty one. Every map is keyed by the id its entries
carry.

| `root` field | value |
|---|---|
| `id` | a fresh graph-root id (`ekr mint graph-root`); every entity's `root_id` is this |
| `space` | `Canonical` |
| `schema_version_id` | the ontology's `version.id` |
| `parent` | `null` |
| `created_at` | milliseconds; 0 is fine |

`revision` is `0`.

| node field | value |
|---|---|
| `id`, `root_id`, `type_id` | its id, the root id, a concrete (not abstract) node type |
| `canonical_name` | the name a reader sees; a property, not an identity |
| `aliases` | other names, a list of strings. A `CreateNode` sets them the same way, and a later `AddAlias` appends one; no operation removes one |
| `type_state` | the lifecycle's `initial` state, or `null` for a type without a lifecycle |
| `properties` | map property id → non-empty list of values, satisfying the type's definitions (required properties present) |

| edge field | value |
|---|---|
| `id`, `root_id`, `type_id` | its id, the root id, an edge type |
| `source`, `target` | node ids of an allowed source and target type |
| `properties` | as for nodes |

An assertion in a seed is written exactly as in [`!AddAssertion`](#assertions), with
`assessment: Proposed`; seeding accepts it. `proposed_by` must be the host operator.

### Evidence and `evidence_payloads`

Every assertion cites evidence. Evidence enters with the seed, as here, or later through a
transaction's [`!AddEvidence`](#evidence-after-the-seed). An evidence entry records where a
statement came from; its payload is the statement's exact bytes.

| evidence field | value |
|---|---|
| `id` | a fresh evidence id; equal to its map key |
| `source` | `!HumanStatement` with `identity: <who or what said it>` (the only source kind P1 seeds accept) |
| `content_hash` | the payload's hash from `ekr hash`, and the key of its `evidence_payloads` entry |
| `extracted_by` | the host operator's id |
| `observed_at` | milliseconds |
| `confidence` | basis points, 0 to 10000 (10000 is certain) |

`evidence_payloads` maps each content hash to the payload's bytes as a list of byte values — the
`payload_yaml` that `ekr hash` prints. Every evidence entry's hash must be a key
(`seed-evidence-payload-missing` otherwise) and every key must be the hash of its bytes
(`seed-evidence-payload-mismatch`). To add evidence: write the statement to a file, run
`ekr hash file`, put `content_hash` into the evidence entry, and either pass the file to
`ekr seed --evidence file` or paste `payload_yaml` into `evidence_payloads`. The hash is over the
file's exact bytes, so a trailing newline changes it.

## Transaction documents (`ekr.transaction-document/2`)

```yaml ekr.transaction-document/2
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000799        # a fresh transaction id: ekr mint transaction
  proposer: 00000000-0000-4000-a000-000000000011  # the host operator
  operations:                                     # one or more, each tagged with its kind
  - !DeleteEdge 00000000-0000-4000-a000-000000000601
  evidence: []                                    # the evidence the AddAssertions cite
```

A schema change adds one key, `schema_version`, after `evidence`: the id of the version it
produces, from `ekr mint schema-version` ([Evolve the schema](#evolve-the-schema)). Every other
transaction omits it.

`operations` is a non-empty list applied in order, all or nothing. `evidence` is exactly the set of
evidence ids cited by the transaction's `!AddAssertion` operations — no more, no fewer
(`evidence-set-mismatch`) — and `[]` when it adds no assertion. Every cited evidence id must be
retained — seeded, or added by an earlier commit — or be added by an `!AddEvidence` of the same
transaction ([Evidence after the seed](#evidence-after-the-seed)). `ekr example
ekr.transaction-document/2` prints a complete document, and `ekr operations <Kind>` prints each
kind's fields.

### Document limits

The format version fixes how large one document may be, and each version's limits are frozen.
Write `ekr.transaction-document/2`. An `ekr.transaction-document/1` document, the original format,
is still read, validated and replayed under its own limits; the two differ in nothing else. Past
any limit `ekr propose` refuses with exit 2,
`ekr.kernel.StructurallyInvalid: transaction document limit: <name> (<bound>)`, where the bound is
the document's own version's, and records nothing; split a larger change into several transactions.

| name | `/2` bound | `/1` bound |
|---|---|---|
| `input_bytes` | at most 8388608 bytes (8 MiB) | at most 262144 bytes |
| `operations` | 1 to 10000 operations | 1 to 256 operations |
| `evidence_elements` | at most 10000 evidence entries | at most 1024 evidence entries |
| `container_depth` | nesting at most 32 deep | nesting at most 32 deep |
| `expanded_nodes` | at most 1048576 values and keys | at most 32768 values and keys |
| `mapping_entries` | at most 4096 entries per map | at most 4096 entries per map |
| `sequence_elements` | at most 16384 elements per sequence | at most 4096 elements per sequence |
| `string_bytes`, `key_bytes` | at most 65536 bytes per string, 4096 per key | at most 65536 bytes per string, 4096 per key |
| `total_string_bytes` | at most 33554432 bytes of text in all | at most 1048576 bytes of text in all |

The byte cap is chosen before the document is parsed, from its top-level `format:` line. Write
that line on its own, as every example here does: a document over 262144 bytes whose format is not
on such a line is held to the `/1` cap.

### Operation kinds

There are sixteen kinds. Eleven are applied under every validation profile. Four are **schema
changes**, applied only under profile v2 and only in a transaction of their own that names its
`schema_version` ([Evolve the schema](#evolve-the-schema)); under profile v1 validation rejects them
with the issue code `unsupported-operation`, so the schema is fixed at seeding. One, `MergeEntity`,
parses but is **refused** under either profile, with the same code.

| kind | applied | what it does |
|---|---|---|
| `CreateNode` | applied | creates a node: `id`, `root_id`, `type_id`, `canonical_name`, `properties`, and optionally `aliases` (a list of strings, the names a typed reference is matched against; absent means none; a non-empty alias another node of the same type holds, or that another `CreateNode` of the transaction gives, is refused) |
| `UpdateProperty` | applied | sets all values of one property of one node: `node`, `property`, `values` (`[]` clears it) |
| `CreateEdge` | applied | creates an edge: `id`, `root_id`, `type_id`, `source`, `target`, `properties` |
| `DeleteEdge` | applied | removes an edge: `!DeleteEdge <edge id>` |
| `AddAssertion` | applied | adds an assertion that cites evidence ([below](#assertions)) |
| `RetractAssertion` | applied | withdraws an accepted, active assertion with a reason: `assertion`, `reason`. It is kept, marked retracted |
| `Invoke` | applied | calls an operation of the node's type: `node`, `operation` (the operation's key under the type's `operations`, not its `name` field), `arguments` (map name → value) |
| `SupersedeAssertion` | applied | replaces an accepted, active assertion from an instant on: `assertion`, `by` (the replacement, which may be added in the same transaction), `effective_from`. [Rules below](#supersession) |
| `AddEvidence` | applied | adds one evidence entry and the bytes it rests on: `evidence` (an entry as in the seed) and `payload` (its bytes). [Rules below](#evidence-after-the-seed) |
| `AddAlias` | applied | appends one alias to a node that exists, so that [`ekr resolve`](#ekr-resolve) finds it by that alias from the next revision on: `node` (a node id from `ekr snapshot`, or one a `CreateNode` of the same transaction creates) and `alias` (a non-empty string). Refused: an alias that node or another node of its type already holds (`alias-already-exists`), one alias given twice for a type in one transaction (`duplicate-alias`), the empty alias (`empty-alias`), a node that does not exist (`unresolved-node`). No operation removes an alias |
| `AttachEvidence` | applied | attaches evidence to an assertion the store holds, accepted and active, leaving the assertion unchanged: `assertion` (an assertion id from `ekr snapshot`) and `evidence` (an evidence id the store retains, or one an `AddEvidence` of the same transaction adds), listed in `transaction.evidence`. [Rules below](#evidence-attached-to-a-held-assertion) |
| `DefineNodeType` | schema change | declares a node type: `id`, `name`, `parents`, `properties`, `abstract_type`, `lifecycle`, `operations`, as in the seed |
| `DefineEdgeType` | schema change | declares an edge type: `id`, `name`, `source_types`, `target_types`, `cardinality`, `properties`, `inverse`, `symmetric`, `transitive`, as in the seed |
| `ModifyProperty` | schema change | adds a property to a type or redeclares one it declares: `owner` (the node or edge type) and `property` (a [property definition](#property-definitions)) |
| `WidenEdgeType` | schema change | adds declared node types to an edge type's ends: `edge_type`, and `source_types` and `target_types` written whole, each holding every type it holds now plus the ones added ([below](#4-widen-an-edge-type)) |
| `MergeEntity` | refused | would merge two nodes |

### Assertions

An assertion is a claim with evidence and a valid time. It is what `snapshot --valid-at`,
`explain`, `!RetractAssertion` and `!SupersedeAssertion` act on.

| field | value |
|---|---|
| `id` | a fresh assertion id |
| `root_id` | the graph root id |
| `subject` | `!Node <node id>`, `!Edge <edge id>` or `!Type <type id>` |
| `predicate` | `!Relation <edge type id>` — the subject and object nodes are related — or `!Property <property id>` — the subject has this property value |
| `object` | `!Node <node id>`, `!Type <type id>` or `!Value <value>` |
| `evidence` | a non-empty list of retained evidence ids |
| `proposed_by` | the host operator |
| `assessment` | `Proposed`. Committing makes it `Accepted` by the validator. Any other value is refused, in one of two places: a bare word such as `assessment: Accepted` is not a valid assessment at all and `ekr propose` refuses the document as `ekr.kernel.StructurallyInvalid` (exit 2, nothing recorded); a complete other assessment such as `assessment: !Accepted {validators: [<id>]}` is recorded by `propose` and then rejected by `ekr validate` with the issue `assertion-states-its-own-verdict` (exit 0, `kind: Rejected`) |
| `lifecycle` | `Active` |
| `valid_time` | `{from: <ms or null>, to: <ms or null>}`: when the claim is true in the world; `null` is unbounded |
| `transaction_time` | `{recorded_from: 0, recorded_to: null}`; the kernel sets it at commit |

A relation assertion (`!Relation`) needs `!Node` subject and object of the edge type's source and
target types. A property assertion (`!Property`) needs a property the subject's type declares, and
an object of its value type: `!Value {value_kind: ..., value: ...}`, or `!Node` for a `NodeRef`
property.

### Supersession

`!SupersedeAssertion` ends one assertion's valid time where its replacement's begins. It is
validated as a whole, and any rule below that fails is the issue `invalid-supersession`:

- the replacement (`by`) is a different assertion, accepted and active — one added with
  `assessment: Proposed` in the same transaction counts;
- the replacement's `valid_time.from` is **exactly** `effective_from`, never `null`;
- `effective_from` lies inside the old assertion's valid time: not before its `from`, not after
  its `to`.

Committing it closes the old assertion's `valid_time.to` at `effective_from` and sets its
`lifecycle` to superseded, naming the replacement. `snapshot --valid-at` still returns the old assertion for instants before
`effective_from`, and the replacement from `effective_from` on. The old assertion must be
accepted and active (`assertion-lifecycle-state` otherwise), one transaction may change an
assertion's lifecycle once (`conflicting-assertion-lifecycle`), and a chain of supersessions may
not lead back to where it started (`supersession-cycle`).

### Evidence after the seed

`!AddEvidence` brings one evidence entry into the store together with the bytes it rests on, so
that an assertion can cite one statement rather than a whole seeded file. The entry is written
exactly as a seed's [evidence entry](#evidence-and-evidence_payloads); `payload` is its bytes as a
list of byte values, the `payload_yaml` that `ekr hash` prints.

One byte is one element of that list, so the document's `sequence_elements` limit
([Document limits](#document-limits)) is the payload's ceiling: at most 16,384 bytes in an
`ekr.transaction-document/2` and 4,096 in an `ekr.transaction-document/1`. One byte more and
`ekr propose` refuses the document as `transaction document limit: sequence_elements`, exit 2,
recording nothing. A larger statement goes into the seed with `ekr seed --evidence <file>`, or is
split into several evidence entries of at most that size.

```yaml ekr.transaction-document/2
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000798        # a fresh transaction id: ekr mint transaction
  proposer: 00000000-0000-4000-a000-000000000011  # the host operator
  operations:
  - !AddEvidence
    evidence:
      id: 00000000-0000-4000-a000-000000000409    # a fresh evidence id: ekr mint evidence
      source: !HumanStatement
        identity: Runtime operator
      content_hash: d665088b6d8d615784418d2e9e79245f5aad71d0565a60fd45ed4649cb8c425c
      extracted_by: 00000000-0000-4000-a000-000000000011
      observed_at: 1773273600000
      confidence: 10000
    payload: [66, 111, 98, 32, 105, 115, 32, 67, 69, 79, 32, 111, 102, 32, 65, 99, 109, 101, 46]
  evidence: []                                    # no assertion here cites it yet
```

An `!AddAssertion` in the same transaction, or in any later one, may cite the new id; list it in
`transaction.evidence` only in a transaction whose assertions cite it or whose `!AttachEvidence`
attaches it ([below](#evidence-attached-to-a-held-assertion)). Committing applies the entry
and stores the payload as an object of its own, in the Provenance class the seed's payloads use;
`ekr explain --documents` prints it for every assertion that cites it, and
[`/changes`](#changes-since) lists the entry once, as an `EvidenceAdded` of the revision that
committed it, whether or not an assertion cites it. Validation refuses, as named issues:

| issue | validator | when |
|---|---|---|
| `evidence-payload-mismatch` | Provenance | the payload's bytes do not hash to `content_hash`; the message names both hashes |
| `evidence-unsupported-source` | Provenance | `source` is not `!HumanStatement` |
| `identity-already-exists` | Structural | the id is one the store already retains, seeded or added |
| `duplicate-identity` | Structural | the transaction adds the same id twice |
| `unresolved-evidence` | Reference | an assertion cites an id the store does not retain and the transaction does not add |

An entry whose `extracted_by` is not the host operator is refused by `ekr propose` as
`ekr.kernel.ProposalAttribution`, exit 2, and nothing is recorded.

### Evidence attached to a held assertion

`!AttachEvidence` attaches evidence to an assertion the store already holds, for example the
exact message a claim rests on where it first cited a whole file. The assertion is not changed —
its subject, predicate, object, valid time, lifecycle and the evidence it was added with stay as
they are — and no supersession is needed. The attachment is a record of its own: which assertion,
which evidence, which revision. [`ekr explain`](#ekr-explain) lists it as an `Attachment` link,
and the evidence among its `Evidence` links, from the revision that attached it on;
[`ekr snapshot`](#ekr-snapshot) lists it under `attachments`; [`ekr quality`](#ekr-quality)
counts it as it counts cited evidence. A supersession does not carry it to the replacement: it
stays with the assertion it was attached to.

```yaml ekr.transaction-document/2
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000799        # a fresh transaction id: ekr mint transaction
  proposer: 00000000-0000-4000-a000-000000000011  # the host operator
  operations:
  - !AddEvidence
    evidence:
      id: 00000000-0000-4000-a000-000000000410    # a fresh evidence id: ekr mint evidence
      source: !HumanStatement
        identity: Runtime operator
      content_hash: d665088b6d8d615784418d2e9e79245f5aad71d0565a60fd45ed4649cb8c425c
      extracted_by: 00000000-0000-4000-a000-000000000011
      observed_at: 1773273600000
      confidence: 10000
    payload: [66, 111, 98, 32, 105, 115, 32, 67, 69, 79, 32, 111, 102, 32, 65, 99, 109, 101, 46]
  - !AttachEvidence
    assertion: 00000000-0000-4000-a000-000000000521  # an assertion id from ekr snapshot
    evidence: 00000000-0000-4000-a000-000000000410
  evidence:
  - 00000000-0000-4000-a000-000000000410          # attached evidence is listed, as cited evidence is
```

Evidence the store already retains is attached without an `!AddEvidence`. Validation refuses, as
named issues:

| issue | validator | when |
|---|---|---|
| `unresolved-assertion` | Reference | the store holds no such assertion; an assertion the same transaction adds is not held — cite the evidence on it instead |
| `unresolved-evidence` | Reference | the evidence is neither retained nor added by an `!AddEvidence` of the transaction |
| `assertion-not-active` | Structural | the assertion is not accepted and active, or the same transaction retracts or supersedes it |
| `evidence-already-attached` | Structural | the assertion already cites the evidence or has it attached, or the transaction attaches it twice |
| `evidence-set-mismatch` | Structural | `transaction.evidence` leaves the attached id out |

### Relations: assertion, edge or both

A relation can be recorded two ways, and often both are wanted. The assertion is the claim, with
evidence and valid time. `!CreateEdge` is the structural record: no evidence, no valid time, held to
the edge type's endpoint types and cardinality, and visible to a reader of the graph's edges.

## Extraction documents (`ekr.extraction-document/1`)

An agent that extracts knowledge from sources — messages, pages, tickets — writes what it read as
one extraction document: the types it needs, the named things it found, the facts it read about
them and the evidence each fact rests on. The consumer runs its extractor, its agent and its
sandbox; the engine starts none of them. `ekr example ekr.extraction-document/1` prints a complete
document for a store seeded from the example seed, and `ekr schema ekr.extraction-document/1` its
JSON Schema. No verb applies a document yet; until one does, record what it says with `ekr
propose`, `ekr validate` and `ekr commit`.

Types, properties and relations are named, never identified: the document is written before the
ids of the types it adds exist, and a name maps to the id the store holds for it.

```yaml ekr.extraction-document/1
format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Project
    parents: []
    abstract_type: false
    properties:
    - name: status
      value:
        value_kind: Enum
        parameters: {variants: [active, closed]}
      cardinality: One
      required: false
  edge_types:
  - name: LEADS
    source_types: [Person]
    target_types: [Project]
    cardinality: Many
    properties: []
entities:
- node_type: Person
  aliases: [Carol]
- node_type: Project
  aliases: [Apollo]
facts:
- !Property
  subject: {node_type: Organization, aliases: [Globex]}
  property: legal_name
  value: {value_kind: String, value: Globex Corporation}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Relation
  subject: {node_type: Person, aliases: [Carol]}
  relation: CEO_OF
  object: {node_type: Organization, aliases: [Globex]}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence:
- evidence:
    id: 00000000-0000-4000-8000-000000000403        # a fresh evidence id: ekr mint evidence
    source: !HumanStatement
      identity: Quarterly report
    content_hash: 7afeb9c852d895a2cdf8d7717a49354badd8b8dc79049925137b1f95f4651af5
    extracted_by: 00000000-0000-4000-8000-000000000101  # the host operator
    observed_at: 1773273600000
    confidence: 10000
  payload: [67, 97, 114, 111, 108, 44, 32, 67, 69, 79, 32, 111, 102, 32, 71, 108, 111, 98, 101, 120, 32, 67, 111, 114, 112, 111, 114, 97, 116, 105, 111, 110, 44, 32, 108, 101, 97, 100, 115, 32, 112, 114, 111, 106, 101, 99, 116, 32, 65, 112, 111, 108, 108, 111, 46]
```

| key | holds |
|---|---|
| `format` | exactly `ekr.extraction-document/1` |
| `ontology` | optional. `node_types` (each `name`, `parents` by name, `abstract_type`, `properties`) and `edge_types` (each `name`, `source_types` and `target_types` by node type name, `cardinality`, `properties`). A property is `name`, `value`, `cardinality` and `required`; `value` is a [value type](#value-types), written as one is (`parameters: {variants: [...]}` for an `Enum`), except that a `NodeRef` names its node types in `parameters: {allowed_types: [...]}` rather than listing their ids. An `Enum`'s variants and a `NodeRef`'s types are at least one, each once; a `Record` field is written once. A type the store lacks is added; one it holds by that name is kept |
| `entities` | optional. Named things, each `node_type` (a node type's name) and `aliases` (the names it is known by, compared byte for byte). Each resolves as a [typed reference](#ekr-resolve) of that type before anything is created |
| `facts` | `!Property` (`subject`, a named thing; `property`, declared on the subject's type or an ancestor; `value`, a value as a transaction writes one) or `!Relation` (`subject`, `relation`, an edge type's name, and `object`). Each lists in `evidence` the ids of the evidence items it rests on: at least one |
| `evidence` | the evidence items, each the `evidence` entry and `payload` bytes an [`!AddEvidence`](#evidence-after-the-seed) carries, under the same rules: a fresh id, `source: !HumanStatement`, `extracted_by` the host operator and a payload that hashes to `content_hash` |

The reader refuses, by code, a document it cannot read:

| code | when |
|---|---|
| `extraction-document-too-large` | over 8388608 bytes, the transaction document's cap |
| `extraction-document-too-deep` | containers nested deeper than 32, the transaction document's limit. The YAML loader stops at the first container past it, so a deeper document is refused at once |
| `extraction-yaml-alias` | a YAML alias (`*name`): write each value out |
| `extraction-document-malformed` | anything else that is not one document of the format: another `format`, an unknown or missing key, a mapping key written twice, a fact written other than as a `!Property` or `!Relation` tag, an alias that is not a YAML string (`~`, `true`, `1.0` and `0x10` are not; quote them), a second document |

Against the ontology of the store it is read for it refuses, naming the first in document order:

| code | when |
|---|---|
| `extraction-name-duplicate` | a node type, an edge type, a property of one type, an `Enum` variant or a `NodeRef` type the document declares twice |
| `extraction-type-conflict` | a type the store holds, redeclared with other `parents` or another `abstract_type` (a node type) or another `cardinality` (an edge type): no schema operation changes those |
| `extraction-type-undeclared` | a node type or edge type neither the document's `ontology` nor the store declares, named by a parent, an edge type's end, a `NodeRef`, a named thing, a fact's subject or object, or a relation |
| `extraction-value-type-empty` | an `Enum` with no variant or a `NodeRef` to no node type |
| `reference-without-identity` | a named thing, or a fact's subject or object, with no alias but the empty string |
| `extraction-property-undeclared` | a `!Property` fact's property, named `Type.property`, that the subject's type and its ancestors do not declare in the document or the store |
| `extraction-value-mismatch` | a `!Property` fact's value its property's type does not hold: another kind, an `Enum` variant it does not list, a `Record` without exactly its fields, or any `Float`, which is never committed |
| `extraction-relation-ends` | a `!Relation` whose subject is not of a source type of its edge type, or whose object is not of a target type, a subtype counting as its parent |
| `fact-without-evidence` | a fact, named `facts[<index>]`, whose `evidence` is empty or absent |
| `fact-evidence-unlisted` | a fact citing an id no item of the document's `evidence` carries |
| `duplicate-identity` | two evidence items under one id |
| `evidence-payload-mismatch` | an evidence item whose `payload` does not hash to its `content_hash` (`ekr hash` prints the right one) |

## Worked example: a library catalogue

A small catalogue of books and their authors, from schema to a committed and explained assertion.
Every block below with a file name is exactly the file the test suite writes and runs, in this
order, on both providers.

### Design

- **Node types.** `Book` and `Author`. A book is a `Publication`, an abstract parent that declares
  the one property every publication has, `title`; later publication types can reuse it.
- **Properties.** Facts that belong to a book and are not in dispute — its page count, binding,
  dimensions — are properties. `Book` declares one of each value kind so this page can show them
  all. `weight_grams` is a `Float` to show what happens to one.
- **Lifecycle.** A book is a `manuscript`, then `published`, then `out_of_print`, and only the
  operations `publish` and `withdraw` move it.
- **Edge type.** `WROTE`, from `Author` to `Book`, with a `contribution` property. Many edges may
  leave one author.
- **Assertions.** "This author wrote this book" is a claim someone made, with a date it became
  true, so it is an assertion citing evidence, and additionally an edge.

### 1. The host

Two fresh agent ids and a tenant, in the shape of `ekr example ekr.cli-host/1`:

```json ekr.cli-host/1 file=host.json
{
  "format": "ekr.cli-host/1",
  "tenant": "library",
  "context": {
    "operator": "00000000-0000-4000-a000-000000000011",
    "validator": "00000000-0000-4000-a000-000000000012"
  },
  "authority": {
    "format": "ekr.authority-state/1",
    "agents": {
      "00000000-0000-4000-a000-000000000011": {
        "id": "00000000-0000-4000-a000-000000000011",
        "name": "Catalogue operator",
        "capabilities": ["propose", "read"]
      },
      "00000000-0000-4000-a000-000000000012": {
        "id": "00000000-0000-4000-a000-000000000012",
        "name": "Catalogue validator",
        "capabilities": ["validate"]
      }
    },
    "validation_profile": {
      "format": "ekr.p1-validation-profile/1",
      "ruleset": "ekr.p1-deterministic/1",
      "checks": ["Structural", "Reference", "Type", "Cardinality", "OntologyConstraint", "Provenance", "Authorization"],
      "validator": "00000000-0000-4000-a000-000000000012",
      "proposer_separation": "distinct-authenticated-actor/1",
      "provenance": "retained-admissible-evidence/1",
      "application": "ekr.p1-apply/1"
    }
  }
}
```

The ids here are written by hand so that they are easy to follow (`…0011` operator, `…01xx` types,
`…02xx` properties, `…03xx` nodes, `…04xx` evidence, `…05xx` assertions, `…06xx` edges, `…07xx`
transactions). In your own schema, use `ekr mint`.

```console
export EKR_HOST=host.json EKR_STORE=./library-store EKR_BACKEND=file
```

### 2. The evidence

Two statements, each in its own file:

```text file=wrote.txt
The Field Naturalist wrote A Field Guide to Lichens.
```

```text file=published.txt
A Field Guide to Lichens was first published on 2019-04-01.
```

```console
$ ekr hash wrote.txt
{
  "algorithm": "sha256(\"ekr.payload.v1\" || bytes)",
  "byte_len": 53,
  "content_hash": "b40f56ebe8e746953148394120d6ee1f8d7997f9dbabbcd4fae159087bf391f4",
  "payload_yaml": "[84, 104, 101, 32, 70, 105, 101, 108, 100, 32, 78, 97, 116, 117, 114, 97, 108, 105, 115, 116, 32, 119, 114, 111, 116, 101, 32, 65, 32, 70, 105, 101, 108, 100, 32, 71, 117, 105, 100, 101, 32, 116, 111, 32, 76, 105, 99, 104, 101, 110, 115, 46, 10]"
}
```

`byte_len` is 53 because the file ends with a newline, and the hash covers it.

### 3. The seed

```yaml ekr-seed/2 file=seed.yaml
format: ekr-seed/2
ontology:
  version:
    id: 00000000-0000-4000-a000-000000000001
    number: 0
    parent: null
    created_at: 0
  node_types:
  - id: 00000000-0000-4000-a000-000000000101
    name: Publication
    parents: []
    properties:
      00000000-0000-4000-a000-000000000201:
        id: 00000000-0000-4000-a000-000000000201
        name: title
        value_type:
          value_kind: String
        cardinality: One
        required: true
        constraints: []
    abstract_type: true
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-a000-000000000102
    name: Book
    parents:
    - 00000000-0000-4000-a000-000000000101
    properties:
      00000000-0000-4000-a000-000000000202:
        id: 00000000-0000-4000-a000-000000000202
        name: in_print
        value_type:
          value_kind: Boolean
      00000000-0000-4000-a000-000000000203:
        id: 00000000-0000-4000-a000-000000000203
        name: page_count
        value_type:
          value_kind: Integer
      00000000-0000-4000-a000-000000000204:
        id: 00000000-0000-4000-a000-000000000204
        name: weight_grams
        value_type:
          value_kind: Float
      00000000-0000-4000-a000-000000000205:
        id: 00000000-0000-4000-a000-000000000205
        name: list_price
        value_type:
          value_kind: Decimal
      00000000-0000-4000-a000-000000000206:
        id: 00000000-0000-4000-a000-000000000206
        name: first_published
        value_type:
          value_kind: Timestamp
      00000000-0000-4000-a000-000000000207:
        id: 00000000-0000-4000-a000-000000000207
        name: reading_time_ms
        value_type:
          value_kind: Duration
      00000000-0000-4000-a000-000000000208:
        id: 00000000-0000-4000-a000-000000000208
        name: translation_of
        value_type:
          value_kind: NodeRef
          parameters:
            allowed_types:
            - 00000000-0000-4000-a000-000000000102
      00000000-0000-4000-a000-000000000209:
        id: 00000000-0000-4000-a000-000000000209
        name: binding
        value_type:
          value_kind: Enum
          parameters:
            variants:
            - hardcover
            - paperback
            - ebook
      00000000-0000-4000-a000-000000000210:
        id: 00000000-0000-4000-a000-000000000210
        name: subjects
        value_type:
          value_kind: String
        cardinality: Many
      00000000-0000-4000-a000-000000000211:
        id: 00000000-0000-4000-a000-000000000211
        name: chapter_titles
        value_type:
          value_kind: List
          parameters:
            value_kind: String
      00000000-0000-4000-a000-000000000212:
        id: 00000000-0000-4000-a000-000000000212
        name: dimensions_mm
        value_type:
          value_kind: Record
          parameters:
            height:
              value_kind: Integer
            width:
              value_kind: Integer
    abstract_type: false
    lifecycle:
      initial: manuscript
      states:
      - manuscript
      - published
      - out_of_print
      transitions:
      - from: manuscript
        to: published
      - from: published
        to: out_of_print
    operations:
      publish:
        name: publish
        arguments:
          imprint:
            value_kind: String
        preconditions: []
        transition:
          from: manuscript
          to: published
        emits: []
      withdraw:
        name: withdraw
        arguments: {}
        preconditions: []
        transition:
          from: published
          to: out_of_print
        emits: []
  - id: 00000000-0000-4000-a000-000000000103
    name: Author
    parents: []
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  edge_types:
  - id: 00000000-0000-4000-a000-000000000104
    name: WROTE
    source_types:
    - 00000000-0000-4000-a000-000000000103
    target_types:
    - 00000000-0000-4000-a000-000000000102
    cardinality: Many
    properties:
      00000000-0000-4000-a000-000000000213:
        id: 00000000-0000-4000-a000-000000000213
        name: contribution
        value_type:
          value_kind: Enum
          parameters:
            variants:
            - sole
            - joint
    inverse: null
    symmetric: false
    transitive: false
graph:
  format: ekr.graph-document/2
  graph:
    root:
      id: 00000000-0000-4000-a000-000000000002
      space: Canonical
      schema_version_id: 00000000-0000-4000-a000-000000000001
      parent: null
      created_at: 0
    revision: 0
    nodes:
      00000000-0000-4000-a000-000000000301:
        id: 00000000-0000-4000-a000-000000000301
        root_id: 00000000-0000-4000-a000-000000000002
        type_id: 00000000-0000-4000-a000-000000000103
        canonical_name: The Field Naturalist
        aliases:
        - field-naturalist
        type_state: null
        properties: {}
      00000000-0000-4000-a000-000000000302:
        id: 00000000-0000-4000-a000-000000000302
        root_id: 00000000-0000-4000-a000-000000000002
        type_id: 00000000-0000-4000-a000-000000000102
        canonical_name: A Field Guide to Lichens
        aliases: []
        type_state: manuscript
        properties:
          00000000-0000-4000-a000-000000000201:
          - value_kind: String
            value: A Field Guide to Lichens
          00000000-0000-4000-a000-000000000202:
          - value_kind: Boolean
            value: true
          00000000-0000-4000-a000-000000000203:
          - value_kind: Integer
            value: 212
          00000000-0000-4000-a000-000000000205:
          - value_kind: Decimal
            value: "24.90"
          00000000-0000-4000-a000-000000000206:
          - value_kind: Timestamp
            value: 1554076800000
          00000000-0000-4000-a000-000000000207:
          - value_kind: Duration
            value: 21600000
          00000000-0000-4000-a000-000000000209:
          - value_kind: Enum
            value: paperback
          00000000-0000-4000-a000-000000000210:
          - value_kind: String
            value: lichens
          - value_kind: String
            value: field guides
          00000000-0000-4000-a000-000000000211:
          - value_kind: List
            value:
            - value_kind: String
              value: Reading a thallus
            - value_kind: String
              value: Crusts, leaves and shrubs
          00000000-0000-4000-a000-000000000212:
          - value_kind: Record
            value:
              height:
                value_kind: Integer
                value: 210
              width:
                value_kind: Integer
                value: 148
    edges: {}
    assertions: {}
    evidence:
      00000000-0000-4000-a000-000000000401:
        id: 00000000-0000-4000-a000-000000000401
        source: !HumanStatement
          identity: Library catalogue desk
        content_hash: b40f56ebe8e746953148394120d6ee1f8d7997f9dbabbcd4fae159087bf391f4
        extracted_by: 00000000-0000-4000-a000-000000000011
        observed_at: 1788220800000
        confidence: 10000
      00000000-0000-4000-a000-000000000402:
        id: 00000000-0000-4000-a000-000000000402
        source: !HumanStatement
          identity: Library catalogue desk
        content_hash: 59cd802ae9923ba9d60cfe25e4bb3cfa88b4535a4a724b5c663d7ddab68f3543
        extracted_by: 00000000-0000-4000-a000-000000000011
        observed_at: 1788220800000
        confidence: 9000
evidence_payloads:
  b40f56ebe8e746953148394120d6ee1f8d7997f9dbabbcd4fae159087bf391f4: [84, 104, 101, 32, 70, 105, 101, 108, 100, 32, 78, 97, 116, 117, 114, 97, 108, 105, 115, 116, 32, 119, 114, 111, 116, 101, 32, 65, 32, 70, 105, 101, 108, 100, 32, 71, 117, 105, 100, 101, 32, 116, 111, 32, 76, 105, 99, 104, 101, 110, 115, 46, 10]
  59cd802ae9923ba9d60cfe25e4bb3cfa88b4535a4a724b5c663d7ddab68f3543: [65, 32, 70, 105, 101, 108, 100, 32, 71, 117, 105, 100, 101, 32, 116, 111, 32, 76, 105, 99, 104, 101, 110, 115, 32, 119, 97, 115, 32, 102, 105, 114, 115, 116, 32, 112, 117, 98, 108, 105, 115, 104, 101, 100, 32, 111, 110, 32, 50, 48, 49, 57, 45, 48, 52, 45, 48, 49, 46, 10]
```

Points to notice: the `Publication` parent's `title` is `required`, so the book must carry it;
the book is in its lifecycle's `initial` state; `weight_grams` is declared and carries no value;
`subjects` has two values because its cardinality is `Many`, while `chapter_titles` is one `List`
value. The author carries the alias `field-naturalist`, which [step 8](#8-resolve-before-you-create)
resolves.

```console
ekr seed seed.yaml       # -> "result": {"revision": 0, ...}
ekr ontology             # the four types and thirteen properties, by name and id
```

### 4. Record who wrote the book

One transaction adds the claim, citing the first statement, and the structural edge:

```yaml ekr.transaction-document/2 file=wrote.yaml outcome=Committed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000701
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !AddAssertion
    id: 00000000-0000-4000-a000-000000000501
    root_id: 00000000-0000-4000-a000-000000000002
    subject: !Node 00000000-0000-4000-a000-000000000301
    predicate: !Relation 00000000-0000-4000-a000-000000000104
    object: !Node 00000000-0000-4000-a000-000000000302
    evidence:
    - 00000000-0000-4000-a000-000000000401
    proposed_by: 00000000-0000-4000-a000-000000000011
    assessment: Proposed
    lifecycle: Active
    valid_time:
      from: 1554076800000
      to: null
    transaction_time:
      recorded_from: 0
      recorded_to: null
  - !CreateEdge
    id: 00000000-0000-4000-a000-000000000601
    root_id: 00000000-0000-4000-a000-000000000002
    type_id: 00000000-0000-4000-a000-000000000104
    source: 00000000-0000-4000-a000-000000000301
    target: 00000000-0000-4000-a000-000000000302
    properties:
      00000000-0000-4000-a000-000000000213:
      - value_kind: Enum
        value: sole
  evidence:
  - 00000000-0000-4000-a000-000000000401
```

```console
ekr propose wrote.yaml                                    # "transaction_id": "…0701"
ekr validate 00000000-0000-4000-a000-000000000701         # "kind": "Validated"
ekr commit 00000000-0000-4000-a000-000000000701           # "kind": "Committed", "revision": 1
```

### 5. Read it back

```console
ekr snapshot --valid-at 2020-01-01
ekr explain 00000000-0000-4000-a000-000000000501 --documents
```

The snapshot's `matching_assertions` contains `00000000-0000-4000-a000-000000000501`: the claim is
believed on 2020-01-01, because its valid time starts on 2019-04-01. For this assertion, which a
transaction added and nothing has retracted or superseded, `ekr explain` prints five links:
`Assertion` (now `Accepted` by the validator), `Proposal`, `Validation`, `Commit` and `Evidence`.
With `--documents` the evidence link's `text` is the statement in `wrote.txt`.

### 6. Publish the book and add a translation

The second transaction uses the other applied kinds: a new node with a `NodeRef`, a property
update, the `publish` operation, and a property assertion.

```yaml ekr.transaction-document/2 file=publish.yaml outcome=Committed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000702
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !CreateNode
    id: 00000000-0000-4000-a000-000000000303
    root_id: 00000000-0000-4000-a000-000000000002
    type_id: 00000000-0000-4000-a000-000000000102
    canonical_name: A Field Guide to Lichens (Spanish translation)
    properties:
      00000000-0000-4000-a000-000000000201:
      - value_kind: String
        value: A Field Guide to Lichens (Spanish translation)
      00000000-0000-4000-a000-000000000208:
      - value_kind: NodeRef
        value: 00000000-0000-4000-a000-000000000302
  - !UpdateProperty
    node: 00000000-0000-4000-a000-000000000302
    property: 00000000-0000-4000-a000-000000000203
    values:
    - value_kind: Integer
      value: 224
  - !Invoke
    node: 00000000-0000-4000-a000-000000000302
    operation: publish
    arguments:
      imprint:
        value_kind: String
        value: Riverside Nature Press
  - !AddAssertion
    id: 00000000-0000-4000-a000-000000000502
    root_id: 00000000-0000-4000-a000-000000000002
    subject: !Node 00000000-0000-4000-a000-000000000302
    predicate: !Property 00000000-0000-4000-a000-000000000206
    object: !Value
      value_kind: Timestamp
      value: 1554076800000
    evidence:
    - 00000000-0000-4000-a000-000000000402
    proposed_by: 00000000-0000-4000-a000-000000000011
    assessment: Proposed
    lifecycle: Active
    valid_time:
      from: null
      to: null
    transaction_time:
      recorded_from: 0
      recorded_to: null
  evidence:
  - 00000000-0000-4000-a000-000000000402
```

After proposing, validating and committing it (revision 2), `ekr snapshot` shows the book with
`type_state: published` and `page_count` 224, and the new node with its `translation_of` reference.

### 7. What is refused

A transaction that writes a `Float` value and tries to declare a new type:

```yaml ekr.transaction-document/2 file=refused.yaml outcome=Rejected:inadmissible-value,unsupported-operation
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000703
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !UpdateProperty
    node: 00000000-0000-4000-a000-000000000302
    property: 00000000-0000-4000-a000-000000000204
    values:
    - value_kind: Float
      value: 310.5
  - !DefineNodeType
    id: 00000000-0000-4000-a000-000000000105
    name: Journal
    parents: []
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  evidence: []
```

`ekr propose` records it (exit 0). `ekr validate` exits 0 with `"kind": "Rejected"` and two issues:
`inadmissible-value` from the `Type` validator and `unsupported-operation` from the `Structural`
validator. `ekr commit` then refuses with `ekr.kernel.TransactionStateConflict` (exit 2). Nothing
changed: the head is still revision 2. To store a weight, the schema would need an `Integer` or
`Decimal` property. This store runs validation profile v1, whose schema is fixed at seeding, so here
that means a new seed in a new store; a store seeded under profile v2 can add the property with
`!ModifyProperty` ([Evolve the schema](#evolve-the-schema)).

### 8. Resolve before you create

Before cataloguing another book by the same author, find the author instead of creating a second
one. A typed reference names the `Author` type and an alias:

```yaml typed-reference file=author.yaml outcome=Resolved:00000000-0000-4000-a000-000000000301
type_id: 00000000-0000-4000-a000-000000000103
aliases:
- field-naturalist
```

An author nobody has catalogued yet:

```yaml typed-reference file=new-author.yaml outcome=ProposeNew
type_id: 00000000-0000-4000-a000-000000000103
aliases:
- moss-collector
```

```console
ekr resolve author.yaml        # "kind": "Resolved", "node_id": "…0301"
ekr resolve new-author.yaml    # "kind": "ProposeNew", "type_id": "…0103", "aliases": ["moss-collector"]
```

The first names the author the seed holds: use `…0301` as the edge's `source`. The second matches
nothing, so `ProposeNew` hands back the type and the alias: mint a node id (`ekr mint node`) and
propose a `!CreateNode` of type `…0103`. Neither run changes the store. A reference to
`Publication` (`…0101`) is refused with `reference-type-has-subtypes`: it is abstract, so name
`Book`.

## Evolve the schema

A store seeded under **validation profile v2** can change its schema after seeding: a committed
transaction adds a node type (`!DefineNodeType`), adds an edge type (`!DefineEdgeType`), adds a
property to a type or redeclares one it declares (`!ModifyProperty`), or adds node types to an edge
type's source and target types (`!WidenEdgeType`). Each committed change produces the next schema
version, numbered one more than the last and naming it as its parent. Nothing already committed
changes: every earlier revision keeps the schema it was committed under, and
`ekr ontology --at <revision>` prints it. No operation removes a type or a property, or narrows an
edge type's ends.

Three rules decide whether a schema change is applied:

- **The store runs profile v2.** A store keeps the profile it was seeded under, from the host
  document's `authority.validation_profile`. The worked example's store runs profile v1, so its
  schema stays the seed's; nothing moves a store from v1 to v2.
- **A schema change travels alone.** A transaction that holds a schema change holds nothing but
  schema changes (`mixed-schema-transaction`). Commit the change, then write data against it.
- **It names the version it produces.** The transaction carries `schema_version`, a fresh id from
  `ekr mint schema-version` (`schema-version-missing` without one, `schema-version-reused` for an
  id the lineage already has).

The kernel then checks the change against the canonical state it would govern, and refuses one
that state would violate — a property made required that a node lacks, a cardinality narrowed
below what a node holds, a value type that no longer admits a held value — with the codes in
[the refusal table](#common-refusals).

This example evolves the catalogue of the worked example in a second store. Every block below with
an `evolve=` name is exactly the file the test suite writes and runs, in this order, on both
providers, beside the worked example's files.

### 1. A host under profile v2

The worked example's host with the v2 `ruleset` and `application`, and nothing else changed:

```json ekr.cli-host/1 evolve=host-v2.json
{
  "format": "ekr.cli-host/1",
  "tenant": "library",
  "context": {
    "operator": "00000000-0000-4000-a000-000000000011",
    "validator": "00000000-0000-4000-a000-000000000012"
  },
  "authority": {
    "format": "ekr.authority-state/1",
    "agents": {
      "00000000-0000-4000-a000-000000000011": {
        "id": "00000000-0000-4000-a000-000000000011",
        "name": "Catalogue operator",
        "capabilities": ["propose", "read"]
      },
      "00000000-0000-4000-a000-000000000012": {
        "id": "00000000-0000-4000-a000-000000000012",
        "name": "Catalogue validator",
        "capabilities": ["validate"]
      }
    },
    "validation_profile": {
      "format": "ekr.p1-validation-profile/1",
      "ruleset": "ekr.p2-deterministic/1",
      "checks": ["Structural", "Reference", "Type", "Cardinality", "OntologyConstraint", "Provenance", "Authorization"],
      "validator": "00000000-0000-4000-a000-000000000012",
      "proposer_separation": "distinct-authenticated-actor/1",
      "provenance": "retained-admissible-evidence/1",
      "application": "ekr.p2-apply/1"
    }
  }
}
```

Seed the worked example's `seed.yaml` into a new store under it:

```console
export EKR_HOST=host-v2.json EKR_STORE=./evolving-store EKR_BACKEND=file
ekr seed seed.yaml       # -> "result": {"revision": 0, ...}
```

The format of `validation_profile` stays `ekr.p1-validation-profile/1` for both profiles.

### 2. Add a type

The catalogue starts taking journals. A journal is a `Publication`, so it inherits the required
`title`; it adds an `issn`, and an editor is an `Author` linked by a new `EDITED` edge type. Mint
the version id first; `ModifyProperty` may name a type defined earlier in the same transaction:

```console
ekr mint schema-version   # {"id": "…", "kind": "schema-version"}; this page uses …0003
```

```yaml ekr.transaction-document/2 evolve=journal.yaml outcome=Committed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000711
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !DefineNodeType
    id: 00000000-0000-4000-a000-000000000105
    name: Journal
    parents:
    - 00000000-0000-4000-a000-000000000101
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  - !ModifyProperty
    owner: 00000000-0000-4000-a000-000000000105
    property:
      id: 00000000-0000-4000-a000-000000000214
      name: issn
      value_type:
        value_kind: String
      cardinality: One
      required: false
      constraints: []
  - !DefineEdgeType
    id: 00000000-0000-4000-a000-000000000106
    name: EDITED
    source_types:
    - 00000000-0000-4000-a000-000000000103
    target_types:
    - 00000000-0000-4000-a000-000000000105
    cardinality: Many
    properties: {}
    inverse: null
    symmetric: false
    transitive: false
  evidence: []
  schema_version: 00000000-0000-4000-a000-000000000003
```

```console
ekr propose journal.yaml                                  # "transaction_id": "…0711"
ekr validate 00000000-0000-4000-a000-000000000711         # "kind": "Validated"
ekr commit 00000000-0000-4000-a000-000000000711           # "kind": "Committed", "revision": 1
```

### 3. Use it

The next transaction is ordinary data against the new version:

```yaml ekr.transaction-document/2 evolve=issue.yaml outcome=Committed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000712
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !CreateNode
    id: 00000000-0000-4000-a000-000000000304
    root_id: 00000000-0000-4000-a000-000000000002
    type_id: 00000000-0000-4000-a000-000000000105
    canonical_name: The Lichenologist's Quarterly
    properties:
      00000000-0000-4000-a000-000000000201:
      - value_kind: String
        value: The Lichenologist's Quarterly
      00000000-0000-4000-a000-000000000214:
      - value_kind: String
        value: "0000-0019"
  - !CreateEdge
    id: 00000000-0000-4000-a000-000000000602
    root_id: 00000000-0000-4000-a000-000000000002
    type_id: 00000000-0000-4000-a000-000000000106
    source: 00000000-0000-4000-a000-000000000301
    target: 00000000-0000-4000-a000-000000000304
    properties: {}
  evidence: []
```

It commits as revision 2. The schema version is still `…0003`: only a schema change makes a new
one.

### 4. Widen an edge type

Journals have authors too, but `WROTE` runs from `Author` to `Book`, so an edge from an author to
the journal is refused (`edge-endpoint-type`). Do not define a second edge type for it: widen
`WROTE`. `!WidenEdgeType` names the edge type and writes both ends whole, as they are to be. Each
end keeps every type it has; here `target_types` adds `Journal` and `source_types` stays as it is:

```yaml ekr.transaction-document/2 evolve=widen.yaml outcome=Committed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000714
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !WidenEdgeType
    edge_type: 00000000-0000-4000-a000-000000000104
    source_types:
    - 00000000-0000-4000-a000-000000000103
    target_types:
    - 00000000-0000-4000-a000-000000000102
    - 00000000-0000-4000-a000-000000000105
  evidence: []
  schema_version: 00000000-0000-4000-a000-000000000004
```

It commits as revision 3 and schema version `…0004`, whose parent is `…0003`. The seeded `WROTE`
edge from the author to the book is unchanged and still valid: a widening only adds, so every edge
the type held still fits it. The next transaction links the author to the journal:

```yaml ekr.transaction-document/2 evolve=wrote-journal.yaml outcome=Committed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000715
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !CreateEdge
    id: 00000000-0000-4000-a000-000000000603
    root_id: 00000000-0000-4000-a000-000000000002
    type_id: 00000000-0000-4000-a000-000000000104
    source: 00000000-0000-4000-a000-000000000301
    target: 00000000-0000-4000-a000-000000000304
    properties: {}
  evidence: []
```

It commits as revision 4. An end never shrinks: writing `target_types` as `Journal` alone, which
leaves out `Book`, is refused.

```yaml ekr.transaction-document/2 evolve=narrow.yaml outcome=Rejected:edge-endpoint-removed
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000716
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !WidenEdgeType
    edge_type: 00000000-0000-4000-a000-000000000104
    source_types:
    - 00000000-0000-4000-a000-000000000103
    target_types:
    - 00000000-0000-4000-a000-000000000105
  evidence: []
  schema_version: 00000000-0000-4000-a000-000000000005
```

`ekr validate` exits 0 with `"kind": "Rejected"` and the issue `edge-endpoint-removed` from the
`OntologyConstraint` validator, naming the end, the edge type and the type it would lose. A widening
that names an edge type or a node type the schema does not declare is `unknown-edge-type` or
`unknown-endpoint-type`, and one that names exactly the ends the type has is
`schema-change-without-effect`.

### 5. What the state refuses

Making `translation_of` required on `Book` would leave the seeded book, which has no translation
source, invalid:

```yaml ekr.transaction-document/2 evolve=required.yaml outcome=Rejected:required-property-missing
format: ekr.transaction-document/2
transaction:
  id: 00000000-0000-4000-a000-000000000713
  proposer: 00000000-0000-4000-a000-000000000011
  operations:
  - !ModifyProperty
    owner: 00000000-0000-4000-a000-000000000102
    property:
      id: 00000000-0000-4000-a000-000000000208
      name: translation_of
      value_type:
        value_kind: NodeRef
        parameters:
          allowed_types:
          - 00000000-0000-4000-a000-000000000102
      cardinality: One
      required: true
      constraints: []
  evidence: []
  schema_version: 00000000-0000-4000-a000-000000000005
```

`ekr validate` exits 0 with `"kind": "Rejected"` and the issue `required-property-missing` from the
`OntologyConstraint` validator, naming the property, the type and how many instances it has.
The head stays at revision 4 and schema version `…0004`. A rejected transaction does not put its
version on the lineage, so `…0005` is still free after `narrow.yaml` was refused.

### 6. Read each version back

```console
ekr ontology --at 0      # schema_version …0001, schema_version_number 0, parent null: no Journal
ekr ontology --at 2      # schema_version …0003, number 1, parent …0001: Journal and EDITED; WROTE to Book
ekr ontology             # revision 4: schema_version …0004, number 2, parent …0003: WROTE to Book and Journal
```

`ekr snapshot --at 0` reads the graph as seeded, and `ekr snapshot` shows the journal at the head.

## Designing a schema

Under validation profile v1 a schema is fixed at seeding, and under v2 it can grow but never
shrink: no operation removes a type or a property, and a change that existing state would violate
is refused. Either way most of the work is in the seed. What holds up:

- **Ids are identities, names are labels.** Every type, property, node, edge, assertion and piece of
  evidence is identified by a UUID that never changes. A name — a node's `canonical_name`, a type's
  or property's `name` — is text a reader sees; nothing refers to anything by name, and two things
  may share one. Never derive an id from a name, and never look a thing up by name when you can keep
  its id. `ekr ontology` and `ekr snapshot` give you names and ids side by side. A name that may
  change or that you want to cite evidence for belongs in a `String` property as well.
- **Types are for things that behave differently.** Make a node type when its instances have their
  own properties, their own lifecycle or their own place as an edge endpoint. A difference that is
  just a label (hardcover or paperback) is an `Enum` property, not a type. Use an abstract parent to
  share properties between types, and to let an edge or a `NodeRef` accept all of them.
- **Properties are for settled attributes; assertions are for claims.** A property value has no
  evidence and no valid time: it is what the catalogue currently records. An assertion carries
  evidence, a valid time, an assessment and a history — it can be retracted or superseded and
  explained. If someone might ask "who says so?" or "since when?", make it an assertion
  (`!Property` for an attribute, `!Relation` for a relationship). The worked example records the
  publication date both ways to show each.
- **Relationships are edge types.** Declare an edge type for each kind of relationship, with the
  narrowest source and target types that are true. When the same relationship later holds between
  more types, widen the edge type under profile v2 ([`!WidenEdgeType`](#4-widen-an-edge-type))
  rather than declaring a second one for the new pair. Use `cardinality: One` only when a second
  edge from the same source would be an error. Record the claim as a `!Relation` assertion, and add
  an edge when readers of the graph's edges should see it.
- **Pick value kinds that can be committed.** No `Float` values; use `Integer` in a stated unit
  (`weight_grams`, `reading_time_ms`) or `Decimal`. Use `Timestamp` for instants, `NodeRef` for
  references to other nodes (not a string holding a name), `Enum` for closed sets, `List` for an
  ordered sequence that is one value, `Record` for a fixed group of fields, and `cardinality: Many`
  for several independent values.
- **Keep reserved fields empty.** `constraints`, `preconditions` and `emits` must be `[]` for writes
  and invocations to be accepted in P1.
- **Declare lifecycles only for real state machines.** A lifecycle's state changes only through a
  named operation with a declared transition; there is no other way to move it.
- **Put the evidence your first assertions need in the seed.** Every assertion must cite retained
  evidence, or evidence its transaction adds with [`!AddEvidence`](#evidence-after-the-seed). Seed
  the statements the seed's own assertions rest on, and add later ones as they arrive.

## Common refusals

There are three forms, and the `exit` column says which one each refusal takes:

- **A named refusal, exit 2.** Nothing was recorded. stderr is `ekr: ekr.kernel.<Name>: <reason>`.
  A refused seed is `ekr.kernel.InvalidSeed: <code>`, followed by `: <detail>` for most codes. A
  write to a store this process may not write is `ekr: store-read-only: <reason>`.
- **A fault, exit 1.** stderr is `ekr: <message>`. A host document the kernel does not accept is
  reported while the provider is opened, as `ekr: opening the provider: invalid seed: <code>`.
- **A validation issue, exit 0.** `ekr validate` records `"kind": "Rejected"`, and each issue carries
  the `code`. A seed runs the same validators, so a seed with the same defect is refused as
  `ekr.kernel.InvalidSeed: <code>: <detail>`, exit 2.

The `where` column names the verbs that report a refusal; `any store verb` means every verb that
opens the store. `crates/ekr/tests/docs_cli.rs` triggers every row that is not a validation issue
against the worked example's files, through each verb its `where` cell names (two different store
verbs for `any store verb`), and checks the exit status in this table and that stderr names the
refusal in the form above. For a validation-issue row it checks that the code is a whole string a
validator in `crates/ekr-kernel/src/validate` raises, or, for a schema change, one of the ontology's
own codes that validator raises as they are. It does not run those rows, except the schema-change
rows: it lists every code a schema change can be refused with and checks that each is a row here or
one no transaction document can reach, and it draws each of those rows from the worked seed under
validation profile v2. The worked example itself produces `inadmissible-value` and
`unsupported-operation`.

| refusal | where | exit | what it means | what to fix |
|---|---|---|---|---|
| `seed-decode` | seed | 2 | the YAML does not have the expected shape: an unknown or missing field, a wrong type, a duplicate key, an empty property value list | the field and line it names |
| `seed-ontology` | seed | 2 | the ontology does not cohere | the rule it names ([the ontology section](#the-ontology-section)) |
| `seed-ontology-lineage` | seed | 2 | the ontology's `version` is not a first version: `number` is not `0` or `parent` is not `null` | `number: 0`, `parent: null` |
| `seed-space` | seed | 2 | the graph root's `space` is not `Canonical`: a seed is canonical state, and a `Transient` root is refused | `space: Canonical` |
| `seed-root-lineage` | seed | 2 | the graph is not revision 0: `revision` is not `0` or `root.parent` is not `null` | `revision: 0`, `parent: null` |
| `seed-schema-version` | seed | 2 | the graph root's `schema_version_id` is not the ontology's `version.id` | make them equal |
| `seed-initial-lifecycle` | seed | 2 | a node's `type_state` is not its type's `initial` state, or is set for a type without a lifecycle | write `initial`, or `null` |
| `seed-attribution-mismatch` | seed | 2 | an assertion's `proposed_by` or an evidence entry's `extracted_by` is not the host operator | use `context.operator` |
| `seed-misfiled-entity` | seed | 2 | a node, edge or assertion is filed under a key that is not its own `id` | make the key equal the id |
| `seed-misrooted-entity` | seed | 2 | an entity's `root_id` is not the graph root's `id` | use the root id |
| `seed-unsupported-source` | seed | 2 | an evidence entry's `source` is not `!HumanStatement` | P1 seeds accept only `!HumanStatement` |
| `seed-evidence-payload-missing` | seed | 2 | an evidence entry's `content_hash` is not a key of `evidence_payloads` | pass the payload file with `--evidence`, or paste the hash `ekr hash` prints as the key |
| `seed-evidence-payload-mismatch` | seed | 2 | a payload's bytes do not hash to its key | re-run `ekr hash` on the exact bytes |
| `ekr.kernel.AlreadySeeded` | seed | 2 | the store already holds a different seed | use a new store or tenant |
| `seed-authority-profile` | any store verb | 1 | the host's `validation_profile` is no accepted profile exactly — an unknown `ruleset`, or a `ruleset` of one profile with an `application` it is not paired with — or its agent registry does not fit it for these agents; reported as `opening the provider: invalid seed: seed-authority-profile` | copy the profile from the example and keep `ruleset` and `application` a pair: `ekr.p1-deterministic/1` with `ekr.p1-apply/1` (v1) , `ekr.p2-deterministic/1` with `ekr.p2-apply/1` (v2) or `ekr.p3-deterministic/1` with `ekr.p2-apply/1` (v3); set `validator` to `context.validator` |
| `store-not-found` | propose, validate, commit, snapshot, explain, head, transactions, ontology, resolve | 1 | `--store` names a path that holds no store: nothing, an empty directory, an empty file, a symlink to nothing, a SQLite database without the runtime's tables, or a file-store directory holding only what `ekr seed` writes before its manifest; nothing is created there. Only `ekr seed` creates a store, and a seed that is refused creates none | check `--store` or `EKR_STORE`; run `ekr seed` first |
| `store-read-only` | seed, propose, validate, commit | 2 | this process may not write the store at `--store` — a file store's directory, `writer.lock`, `events.jsonl` or `blobs`; a SQLite database, its directory or its `-wal` or `-shm` file — so the verb, which writes, is refused before it opens anything and nothing is written. The verbs that only read answer on the same store ([Configuration](#configuration)) | run the verb as a user that may write the store, or on a writable copy of it |
| `bootstrap-authority-mismatch` | any store verb | 1 | the store was seeded under a host document whose authority differs from this one | use the host document the store was seeded with |
| `ekr.kernel.ProposalAttribution` | propose | 2 | the document's `proposer`, an assertion's `proposed_by` or an added evidence entry's `extracted_by` is not the host operator | use `context.operator` |
| `ekr.kernel.StructurallyInvalid` | propose | 2 | the transaction document does not parse, for example a bare `assessment: Accepted` | the field it names; compare with `ekr operations <Kind>` |
| `ekr.kernel.TransactionNotFound` | validate, commit | 2 | no transaction has that id | take the id `ekr propose` printed, or `ekr transactions` |
| `ekr.kernel.TransactionStateConflict` | validate, commit | 2 | the transaction is not in the state the verb needs: validating one that is already validated, committing one that is only proposed or was rejected | propose a corrected document under a new id |
| `ekr.kernel.AssertionNotFound` | explain | 2 | no assertion has that id at the head | take the id from `ekr snapshot` |
| `unsupported-operation` | validation issue | 0 | a `MergeEntity` operation under either profile, or a `DefineNodeType`, `DefineEdgeType`, `ModifyProperty` or `WidenEdgeType` operation under validation profile v1 | none for `MergeEntity`; a schema change needs a store seeded under [profile v2](#evolve-the-schema) |
| `merge-into-itself` | validation issue | 0 | a `MergeEntity` whose `absorbed` and `into` are the same node | none: `MergeEntity` is not applied under either profile |
| `schema-version-missing` | validation issue | 0 | a schema change without `schema_version` (profile v2) | add `schema_version: <ekr mint schema-version>` |
| `schema-version-without-schema-change` | validation issue | 0 | a `schema_version` on a transaction none of whose operations changes the schema | remove it |
| `mixed-schema-transaction` | validation issue | 0 | a schema change and an operation of another kind in one transaction (profile v2) | propose the schema change alone, commit it, then the rest |
| `modify-property-without-owner` | validation issue | 0 | a `ModifyProperty` written as a bare property declaration (`id`, `name`, `value_type`, …) without `owner` and `property`, the shape the 0.0.2 and 0.0.3 pages showed (profile v2; profile v1 rejects it as `unsupported-operation`) | write `{owner: <type id from ekr ontology>, property: <the declaration>}`, as `ekr operations ModifyProperty` prints |
| `schema-version-reused` | validation issue | 0 | a `schema_version` that is already a version of this store's lineage, such as the seed's | `ekr mint schema-version` |
| `unknown-property-owner` | validation issue | 0 | a `ModifyProperty` whose `owner` is not a declared node or edge type, nor one defined earlier in the same transaction | take the type id from `ekr ontology` |
| `incoherent-schema` | validation issue | 0 | the evolved schema does not cohere, for example an edge type whose endpoint type is not declared; it names the rule | the rule it names ([the ontology section](#the-ontology-section)) |
| `schema-change-without-effect` | validation issue | 0 | the changes leave the schema exactly as it was, such as a property redeclared unchanged or an edge type widened to the ends it has | drop the transaction, or change something |
| `unknown-edge-type` | validation issue | 0 | a `WidenEdgeType` whose `edge_type` is not a declared edge type, nor one defined earlier in the same transaction | take the id from `ekr ontology` `edge_types` |
| `unknown-endpoint-type` | validation issue | 0 | a `WidenEdgeType` whose `source_types` or `target_types` names a type that is not a declared node type, nor one defined earlier in the same transaction | take node type ids from `ekr ontology` `node_types`, or define the type first |
| `edge-endpoint-removed` | validation issue | 0 | a `WidenEdgeType` whose `source_types` or `target_types` leaves out a type that end already has; an end only grows | write every type the end has, plus the ones to add |
| `required-property-missing` | validation issue | 0 | a property made `required` on a type one of whose nodes or edges has no value of it | give every instance a value first, or leave it optional |
| `cardinality-narrowed` | validation issue | 0 | a property made `One` where an instance holds several values | reduce the values first, or keep `Many` |
| `value-kind-not-admitted` | validation issue | 0 | a property's value type changed to one that does not admit a kind of value instances hold, including the objects of active property assertions | keep a value type that admits what is held |
| `value-type-narrowed` | validation issue | 0 | a value type of the same kind that no longer admits a held value: an `Enum` variant or a `NodeRef` type dropped, at any depth | keep what is held admissible |
| `constraint-changed` | validation issue | 0 | a property's `constraints` changed on a type that has instances | leave `constraints` as they are (in this release, `[]`) |
| `unknown-type` | validation issue | 0 | a `type_id` or `!Relation` id the ontology does not declare | take ids from `ekr ontology` |
| `abstract-type` | validation issue | 0 | a node of an abstract type | use a concrete subtype |
| `undeclared-property` | validation issue | 0 | a property the node's or edge's type (and its parents) does not declare | take property ids from `ekr ontology` |
| `wrong-type` | validation issue | 0 | a value of the wrong kind, an undeclared `Enum` variant, a `NodeRef` to a disallowed type, a `Record` with missing or extra fields | match the property's `value_type` |
| `inadmissible-value` | validation issue | 0 | a `Float` value anywhere | use `Integer` or `Decimal` |
| `missing-required-property` | validation issue | 0 | a required property with no value | supply it |
| `property-cardinality` | validation issue | 0 | more values than a `cardinality: One` property allows | one value, or a `Many` property |
| `edge-cardinality` | validation issue | 0 | a second edge of a `cardinality: One` edge type from the same source | delete the old edge first, or use `Many` |
| `edge-endpoint-type` | validation issue | 0 | an edge's or relation assertion's source or target is not of an allowed type | check the edge type's `source_types` and `target_types` |
| `unresolved-node` | validation issue | 0 | a node id that does not exist | take ids from `ekr snapshot`, or create the node earlier in the same transaction |
| `unresolved-evidence` | validation issue | 0 | an assertion cites, or an `AttachEvidence` attaches, evidence that is not retained and that no `!AddEvidence` of its transaction adds | cite or attach retained evidence, or add it with `!AddEvidence` |
| `assertion-not-active` | validation issue | 0 | an `AttachEvidence` names an assertion that is not accepted and active, or one the same transaction retracts or supersedes | attach only to current assertions; check `lifecycle` in `ekr snapshot` |
| `evidence-already-attached` | validation issue | 0 | an `AttachEvidence` attaches evidence the assertion already cites or already has attached, or attaches it twice in one transaction | attach each piece of evidence to an assertion once |
| `unresolved-edge` | validation issue | 0 | an edge id (`!DeleteEdge`, an `!Edge` subject) that does not exist | take ids from `ekr snapshot` |
| `unresolved-assertion` | validation issue | 0 | a retraction or supersession names an assertion (or a `by`) that does not exist, or an `AttachEvidence` names one the store does not hold (one the same transaction adds included) | take ids from `ekr snapshot`, or add the replacement in the same transaction; cite evidence on an assertion you add rather than attaching it |
| `unresolved-graph-root` | validation issue | 0 | an entity's `root_id` is not the graph root | use the root id from `ekr snapshot` (`root_id` of any node) |
| `assertion-lifecycle-state` | validation issue | 0 | a retraction or supersession of an assertion that is not accepted and active, for example one already retracted or superseded | act only on current assertions; check `lifecycle` in `ekr snapshot` |
| `conflicting-assertion-lifecycle` | validation issue | 0 | one transaction retracts or supersedes the same assertion twice | one lifecycle change per assertion per transaction |
| `invalid-supersession` | validation issue | 0 | a [supersession rule](#supersession) fails, most often a replacement `valid_time.from` that is not exactly `effective_from` | set the replacement's `from` to `effective_from` |
| `supersession-cycle` | validation issue | 0 | supersessions would lead back to the assertion they started from | supersede toward a new assertion |
| `evidence-set-mismatch` | validation issue | 0 | `transaction.evidence` is not exactly the evidence the assertions cite and the `AttachEvidence` operations attach | list exactly those ids |
| `assertion-without-evidence` | validation issue | 0 | an assertion cites no evidence | cite at least one evidence id |
| `assertion-states-its-own-verdict` | validation issue | 0 | an assertion written with a complete assessment other than `Proposed`, such as `!Accepted {validators: [...]}` (a bare `Accepted` is refused earlier, as `ekr.kernel.StructurallyInvalid`) | write `assessment: Proposed` |
| `evidence-payload-mismatch` | validation issue | 0 | an `!AddEvidence` payload whose bytes do not hash to the entry's `content_hash`; the message names both hashes | re-run `ekr hash` on the exact bytes, and paste its `content_hash` and `payload_yaml` |
| `evidence-unsupported-source` | validation issue | 0 | an `!AddEvidence` entry whose `source` is not `!HumanStatement` | add evidence from a `!HumanStatement` source |
| `identity-already-exists` | validation issue | 0 | a create or an `!AddEvidence` reuses an id that already exists | `ekr mint` a fresh id |
| `identity-previously-held` | validation issue | 0 | under profile v3, a `CreateNode` or `CreateEdge` takes an id an earlier revision held for a node or an edge, such as a deleted edge's | `ekr mint` a fresh id |
| `duplicate-identity` | validation issue | 0 | one transaction creates or adds the same id twice | `ekr mint` one id per created thing |
| `alias-already-exists` | validation issue | 0 | a `CreateNode` gives a non-empty alias that a node of the same type already holds, or an `AddAlias` gives a node an alias that it or another node of its type already holds | resolve the reference and use that node instead of creating one or aliasing another |
| `duplicate-alias` | validation issue | 0 | two `CreateNode` or `AddAlias` operations of one transaction give the same non-empty alias to nodes of one type | give each alias to one node, once |
| `empty-alias` | validation issue | 0 | an `AddAlias` gives the empty alias, which identifies nothing | give the alias a typed reference names |
| `conflicting-write` | validation issue | 0 | one transaction writes the same property of a node twice with different values, moves one node's lifecycle twice, declares one property of one type in two `ModifyProperty`, or widens one edge type in two `WidenEdgeType` | one write per property, one state move per node, one declaration per property and one widening per edge type per transaction |
| `operation-not-declared` | validation issue | 0 | `!Invoke` names no operation **key** of the node's type (an operation's `name` field is not consulted) | use the key under `operations` |
| `transition-refused` | validation issue | 0 | the node is not in the operation's `from` state | check the node's `type_state` |
| `missing-argument` | validation issue | 0 | `!Invoke` omits a declared argument | pass every declared argument |
| `undeclared-argument` | validation issue | 0 | `!Invoke` passes an argument the operation does not declare | remove it |
| `unsupported-constraint` | validation issue | 0 | the affected type has property `constraints`, or the operation has `preconditions` or `emits` | keep them `[]` in the seed. Under profile v2 a `ModifyProperty` can set a property's `constraints` to `[]` while its type has no instances (`constraint-changed` once it has any); `preconditions` and `emits` belong to a type's operations, which no operation changes after the type is declared |
