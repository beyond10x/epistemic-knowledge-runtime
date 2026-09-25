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
document below is parsed by the real readers, the worked example is seeded and committed on both
storage providers, the verb, option, operation-kind and value-kind tables are compared with what the
binary has, and the refusal table is checked as [its introduction](#common-refusals) says.

Contents:

- [Install](#install)
- [Configuration](#configuration), including [the host document](#the-host-document-ekrcli-host1)
- [Exit codes and output](#exit-codes-and-output)
- [Verbs](#verbs)
- [The workflow](#the-workflow)
- [The seed document](#the-seed-document-ekr-seed2): [ontology](#the-ontology-section),
  [value types](#value-types), [graph](#the-graph-section), [evidence](#evidence-and-evidence_payloads)
- [Transaction documents](#transaction-documents-ekrtransaction-document1) and
  [operation kinds](#operation-kinds)
- [Worked example: a library catalogue](#worked-example-a-library-catalogue)
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

Verbs that read or write a store need three settings. Each is a flag or an environment variable;
the flag wins, and an empty variable counts as unset.

| flag | variable | value |
|---|---|---|
| `--host` | `EKR_HOST` | path to the trusted host document, an `ekr.cli-host/1` JSON file (below) |
| `--store` | `EKR_STORE` | where the data lives: a directory for `file`, a database file for `sqlite` |
| `--backend` | `EKR_BACKEND` | `file` or `sqlite`, lowercase |

The store need not exist before `ekr seed`: the file provider creates the directory and any missing
parents, the SQLite provider creates the database file but not its directory. `guide`, `operations`,
`example`, `mint`, `hash` and `schema` open no store, need none of the settings and ignore the variables.

```console
export EKR_HOST=host.json EKR_STORE=./store EKR_BACKEND=file
```

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
| `authority.validation_profile` | the deterministic P1 validation profile. Copy it from `ekr example ekr.cli-host/1` unchanged except `validator`, which must equal `context.validator` |

Unknown, missing or duplicated fields are refused. A host document that parses but whose profile or
agent registry is not the P1 one is refused when the store is opened, by any store verb including
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
| 1 | a fault: provider, verification, unreadable input, host configuration, a store that is not seeded | a message on stderr |
| 2 | a named refusal or a usage error. Nothing was recorded | `ekr: ekr.kernel.<Name>: <reason>` on stderr, or clap's usage message |

`guide`, `operations` and `example` print text; every other verb prints one JSON document. In JSON
output a tagged value is an object with one key — `{"Node": "<id>"}` — where a YAML input writes the
tag `!Node <id>`. A proposal record's `document_bytes` prints as one standard base64 string.

## Verbs

| verb | store | input | prints |
|---|---|---|---|
| `ekr seed` | writes | an `ekr-seed/2` file, or `-` for stdin | the seed result: `result.revision` is `0` |
| `ekr propose` | writes | an `ekr.transaction-document/1` file, or `-` | the proposal record: `transaction_id` |
| `ekr validate` | writes | a transaction id; `--against <revision>` | the validation outcome: `kind` is `Validated` or `Rejected` (with `issues`) |
| `ekr commit` | writes | a transaction id | the commit outcome: `kind` is `Committed` (with `result.revision`) or `Stale` |
| `ekr snapshot` | reads | `--at <revision>`, `--valid-at <ms or YYYY-MM-DD>` | the whole graph at one revision |
| `ekr explain` | reads | an assertion id | the assertion, where it came from, what later changed it, and its evidence |
| `ekr head` | reads | none | the head `revision` and its `root` |
| `ekr transactions` | reads | `--state <State>` | every retained transaction: id, state, proposer |
| `ekr ontology` | reads | none | node types, edge types and properties with names and ids |
| `ekr guide` | none | none | the workflow, as text |
| `ekr operations` | none | an operation kind, optionally | the kinds, or one kind's fields and example |
| `ekr example` | none | `ekr.transaction-document/1`, `ekr-seed/2` or `ekr.cli-host/1` (aliases `transaction`, `seed`, `host`) | a complete example document |
| `ekr mint` | none | an id kind | `{"id", "kind"}`: a fresh id |
| `ekr hash` | none | a payload file, or `-` | the payload's `content_hash` and its `payload_yaml` |
| `ekr schema` | none | `ekr.transaction-document/1`, `ekr-seed/2` or `ekr.cli-host/1` (aliases `transaction`, `seed`, `host`) | the format's JSON Schema (draft 2020-12) |

Every verb has `--help`.

### `ekr seed`

Writes revision 0 from an `ekr-seed/2` document: the schema, the initial graph and the evidence
payloads. The seed is validated like a transaction first; a seed that does not validate is refused as
`ekr.kernel.InvalidSeed` with the reason. Seeding the same document again returns the original result
(exit 0); a different document on a seeded store is refused as `ekr.kernel.AlreadySeeded`.

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
`nodes`, `edges`, `assertions` and `evidence`, each a map keyed by id. A plain snapshot returns every
assertion, including retracted and superseded ones, with `matching_assertions: null`. With
`--valid-at` (milliseconds since the Unix epoch, or `YYYY-MM-DD` for midnight UTC),
`matching_assertions` lists the ids of the assertions believed at that instant: accepted, not
retracted, with a valid time that contains it. Valid time is half-open: `from` is included, `to` is
not.

### `ekr explain`

Explains one assertion at the head. `links` is a list of objects, each with a `kind`:

| `kind` | what it is | when it appears |
|---|---|---|
| `Assertion` | an assertion as it stands at the head | first, for the requested assertion; again for each assertion that superseded it, followed by that one's own origin and lifecycle links |
| `Seed` | the seed the assertion came from | for an assertion written in the seed, in place of `Proposal`, `Validation` and `Commit` |
| `Proposal` | the retained proposal of the transaction that added it | for an assertion added by a transaction |
| `Validation` | that transaction's validation | with `Proposal` |
| `Commit` | that transaction's commit receipt | with `Proposal` |
| `Lifecycle` | a later committed retraction or supersession of it | once per such change |
| `Evidence` | an evidence entry cited, with `payload` (the retained bytes, base64) and `text` (the same bytes as a string, when they are UTF-8) | last, once per evidence id cited by an assertion in the chain or listed in the `evidence` of a transaction that added or changed one |

Read links by their `kind` and, for `Assertion` and `Lifecycle`, by the assertion id they carry:
`id` on an `Assertion` link, `assertion_id` on a `Lifecycle` link. Do not read them by position, because the number and order of links depend on the assertion's
history. An unknown id is refused as `ekr.kernel.AssertionNotFound`.

### `ekr head`

Prints the newest revision number and its root hashes. Use it for `validate --against` and to see
that a commit landed.

### `ekr transactions`

Lists every retained transaction with `transaction_id`, `state`, `proposer`, `operation_count` and
`submitted_at`. `--state` filters by one of `Proposed`, `Validated`, `Committed`, `Rejected` or
`Stale`.

### `ekr ontology`

Prints the schema in force: `node_types` and `edge_types` with their ids, names, parents, endpoint
types and properties. These are the ids a transaction document uses for `type_id`,
`predicate: !Relation` and property keys.

### `ekr guide`

Prints the workflow for an agent: roles, propose → validate → commit, exit codes, where ids come from
and how to add evidence to a seed.

### `ekr operations`

Without an argument, lists the twelve operation kinds, one per line, marking the four P1 does not
apply. With a kind (`ekr operations AddAssertion`), prints its fields and an example operation.

### `ekr example`

Prints a complete document of one format. The three examples fit together: seed a store from the
example seed under the example host, and the example transaction commits.

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
| a transaction document over 262144 bytes, nested deeper than 32, with more than 32768 values and keys, or more than 1048576 bytes of text | refuses | cannot see it |

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
ekr explain <assertion_id>                      # why
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

A seed is a YAML document with four top-level fields. It is the only place a schema is declared:
**in P1 the schema cannot change after seeding**, because the operations that would change it are
refused (see [operation kinds](#operation-kinds)). Design the schema before you seed.

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
| `aliases` | other names, a list of strings. Seed only: transactions do not set aliases |
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

Every assertion cites evidence, and in P1 evidence enters only through the seed. An evidence entry
records where a statement came from; its payload is the statement's exact bytes.

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
`ekr hash file`, and paste `content_hash` and `payload_yaml` into the seed. The hash is over the
file's exact bytes, so a trailing newline changes it.

## Transaction documents (`ekr.transaction-document/1`)

```yaml ekr.transaction-document/1
format: ekr.transaction-document/1
transaction:
  id: 00000000-0000-4000-a000-000000000799        # a fresh transaction id: ekr mint transaction
  proposer: 00000000-0000-4000-a000-000000000011  # the host operator
  operations:                                     # one or more, each tagged with its kind
  - !DeleteEdge 00000000-0000-4000-a000-000000000601
  evidence: []                                    # the evidence the AddAssertions cite
```

`operations` is a non-empty list applied in order, all or nothing. `evidence` is exactly the set of
evidence ids cited by the transaction's `!AddAssertion` operations — no more, no fewer
(`evidence-set-mismatch`) — and `[]` when it adds no assertion. Every cited evidence id must already
be retained, which in P1 means seeded. `ekr example ekr.transaction-document/1` prints a complete
document, and `ekr operations <Kind>` prints each kind's fields.

### Operation kinds

There are twelve kinds. Eight are applied in P1. Four parse but are **refused**: validation rejects
every proposal containing them with the issue code `unsupported-operation`. That is why the schema is
fixed at seeding.

| kind | P1 | what it does |
|---|---|---|
| `CreateNode` | applied | creates a node: `id`, `root_id`, `type_id`, `canonical_name`, `properties` |
| `UpdateProperty` | applied | sets all values of one property of one node: `node`, `property`, `values` (`[]` clears it) |
| `CreateEdge` | applied | creates an edge: `id`, `root_id`, `type_id`, `source`, `target`, `properties` |
| `DeleteEdge` | applied | removes an edge: `!DeleteEdge <edge id>` |
| `AddAssertion` | applied | adds an assertion that cites evidence ([below](#assertions)) |
| `RetractAssertion` | applied | withdraws an accepted, active assertion with a reason: `assertion`, `reason`. It is kept, marked retracted |
| `Invoke` | applied | calls an operation of the node's type: `node`, `operation` (the operation's key under the type's `operations`, not its `name` field), `arguments` (map name → value) |
| `SupersedeAssertion` | applied | replaces an accepted, active assertion from an instant on: `assertion`, `by` (the replacement, which may be added in the same transaction), `effective_from`. [Rules below](#supersession) |
| `DefineNodeType` | refused | would declare a node type |
| `DefineEdgeType` | refused | would declare an edge type |
| `ModifyProperty` | refused | would redeclare a property definition |
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

### Relations: assertion, edge or both

A relation can be recorded two ways, and often both are wanted. The assertion is the claim, with
evidence and valid time. `!CreateEdge` is the structural record: no evidence, no valid time, held to
the edge type's endpoint types and cardinality, and visible to a reader of the graph's edges.

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
        aliases: []
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
value.

```console
ekr seed seed.yaml       # -> "result": {"revision": 0, ...}
ekr ontology             # the four types and thirteen properties, by name and id
```

### 4. Record who wrote the book

One transaction adds the claim, citing the first statement, and the structural edge:

```yaml ekr.transaction-document/1 file=wrote.yaml outcome=Committed
format: ekr.transaction-document/1
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
ekr explain 00000000-0000-4000-a000-000000000501
```

The snapshot's `matching_assertions` contains `00000000-0000-4000-a000-000000000501`: the claim is
believed on 2020-01-01, because its valid time starts on 2019-04-01. For this assertion, which a
transaction added and nothing has retracted or superseded, `ekr explain` prints five links:
`Assertion` (now `Accepted` by the validator), `Proposal`, `Validation`, `Commit` and `Evidence`,
and the evidence link's `text` is the statement in `wrote.txt`.

### 6. Publish the book and add a translation

The second transaction uses the other applied kinds: a new node with a `NodeRef`, a property
update, the `publish` operation, and a property assertion.

```yaml ekr.transaction-document/1 file=publish.yaml outcome=Committed
format: ekr.transaction-document/1
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

```yaml ekr.transaction-document/1 file=refused.yaml outcome=Rejected:inadmissible-value,unsupported-operation
format: ekr.transaction-document/1
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
`Decimal` property — which, in P1, means a new seed in a new store.

## Designing a schema

Because a P1 schema is fixed at seeding, most of the work is in the seed. What holds up:

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
  narrowest source and target types that are true. Use `cardinality: One` only when a second edge
  from the same source would be an error. Record the claim as a `!Relation` assertion, and add an
  edge when readers of the graph's edges should see it.
- **Pick value kinds that can be committed.** No `Float` values; use `Integer` in a stated unit
  (`weight_grams`, `reading_time_ms`) or `Decimal`. Use `Timestamp` for instants, `NodeRef` for
  references to other nodes (not a string holding a name), `Enum` for closed sets, `List` for an
  ordered sequence that is one value, `Record` for a fixed group of fields, and `cardinality: Many`
  for several independent values.
- **Keep reserved fields empty.** `constraints`, `preconditions` and `emits` must be `[]` for writes
  and invocations to be accepted in P1.
- **Declare lifecycles only for real state machines.** A lifecycle's state changes only through a
  named operation with a declared transition; there is no other way to move it.
- **Put the evidence you will need in the seed.** In P1 evidence enters only through the seed, and
  every assertion must cite retained evidence. Seed the statements your first assertions rest on.

## Common refusals

There are three forms, and the `exit` column says which one each refusal takes:

- **A named refusal, exit 2.** Nothing was recorded. stderr is `ekr: ekr.kernel.<Name>: <reason>`.
  A refused seed is `ekr.kernel.InvalidSeed: <code>`, followed by `: <detail>` for most codes.
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
validator in `crates/ekr-kernel/src/validate` raises; it does not run those rows. The worked example
itself produces `inadmissible-value` and `unsupported-operation`.

| refusal | where | exit | what it means | what to fix |
|---|---|---|---|---|
| `seed-decode` | seed | 2 | the YAML does not have the expected shape: an unknown or missing field, a wrong type, a duplicate key, an empty property value list | the field and line it names |
| `seed-ontology` | seed | 2 | the ontology does not cohere | the rule it names ([the ontology section](#the-ontology-section)) |
| `seed-ontology-lineage` | seed | 2 | the ontology's `version` is not a first version: `number` is not `0` or `parent` is not `null` | `number: 0`, `parent: null` |
| `seed-root-lineage` | seed | 2 | the graph is not revision 0: `revision` is not `0` or `root.parent` is not `null` | `revision: 0`, `parent: null` |
| `seed-schema-version` | seed | 2 | the graph root's `schema_version_id` is not the ontology's `version.id` | make them equal |
| `seed-initial-lifecycle` | seed | 2 | a node's `type_state` is not its type's `initial` state, or is set for a type without a lifecycle | write `initial`, or `null` |
| `seed-attribution-mismatch` | seed | 2 | an assertion's `proposed_by` or an evidence entry's `extracted_by` is not the host operator | use `context.operator` |
| `seed-misfiled-entity` | seed | 2 | a node, edge or assertion is filed under a key that is not its own `id` | make the key equal the id |
| `seed-misrooted-entity` | seed | 2 | an entity's `root_id` is not the graph root's `id` | use the root id |
| `seed-unsupported-source` | seed | 2 | an evidence entry's `source` is not `!HumanStatement` | P1 seeds accept only `!HumanStatement` |
| `seed-evidence-payload-missing` | seed | 2 | an evidence entry's `content_hash` is not a key of `evidence_payloads` | paste the hash `ekr hash` prints as the key |
| `seed-evidence-payload-mismatch` | seed | 2 | a payload's bytes do not hash to its key | re-run `ekr hash` on the exact bytes |
| `ekr.kernel.AlreadySeeded` | seed | 2 | the store already holds a different seed | use a new store or tenant |
| `seed-authority-profile` | any store verb | 1 | the host's `validation_profile` or agent registry is not the P1 profile for these agents; reported as `opening the provider: invalid seed: seed-authority-profile` | copy the profile from the example; set `validator` to `context.validator` |
| `bootstrap-authority-mismatch` | any store verb | 1 | the store was seeded under a host document whose authority differs from this one | use the host document the store was seeded with |
| `ekr.kernel.ProposalAttribution` | propose | 2 | the document's `proposer` is not the host operator | use `context.operator` |
| `ekr.kernel.StructurallyInvalid` | propose | 2 | the transaction document does not parse, for example a bare `assessment: Accepted` | the field it names; compare with `ekr operations <Kind>` |
| `ekr.kernel.TransactionNotFound` | validate, commit | 2 | no transaction has that id | take the id `ekr propose` printed, or `ekr transactions` |
| `ekr.kernel.TransactionStateConflict` | validate, commit | 2 | the transaction is not in the state the verb needs: validating one that is already validated, committing one that is only proposed or was rejected | propose a corrected document under a new id |
| `ekr.kernel.AssertionNotFound` | explain | 2 | no assertion has that id at the head | take the id from `ekr snapshot` |
| `unsupported-operation` | validation issue | 0 | a `DefineNodeType`, `DefineEdgeType`, `ModifyProperty` or `MergeEntity` operation | none in P1: the schema is the seed's |
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
| `unresolved-evidence` | validation issue | 0 | an assertion cites evidence that is not retained | cite seeded evidence |
| `unresolved-edge` | validation issue | 0 | an edge id (`!DeleteEdge`, an `!Edge` subject) that does not exist | take ids from `ekr snapshot` |
| `unresolved-assertion` | validation issue | 0 | a retraction or supersession names an assertion (or a `by`) that does not exist | take ids from `ekr snapshot`, or add the replacement in the same transaction |
| `unresolved-graph-root` | validation issue | 0 | an entity's `root_id` is not the graph root | use the root id from `ekr snapshot` (`root_id` of any node) |
| `assertion-lifecycle-state` | validation issue | 0 | a retraction or supersession of an assertion that is not accepted and active, for example one already retracted or superseded | act only on current assertions; check `lifecycle` in `ekr snapshot` |
| `conflicting-assertion-lifecycle` | validation issue | 0 | one transaction retracts or supersedes the same assertion twice | one lifecycle change per assertion per transaction |
| `invalid-supersession` | validation issue | 0 | a [supersession rule](#supersession) fails, most often a replacement `valid_time.from` that is not exactly `effective_from` | set the replacement's `from` to `effective_from` |
| `supersession-cycle` | validation issue | 0 | supersessions would lead back to the assertion they started from | supersede toward a new assertion |
| `evidence-set-mismatch` | validation issue | 0 | `transaction.evidence` is not exactly the evidence the assertions cite | list exactly those ids |
| `assertion-without-evidence` | validation issue | 0 | an assertion cites no evidence | cite at least one evidence id |
| `assertion-states-its-own-verdict` | validation issue | 0 | an assertion written with a complete assessment other than `Proposed`, such as `!Accepted {validators: [...]}` (a bare `Accepted` is refused earlier, as `ekr.kernel.StructurallyInvalid`) | write `assessment: Proposed` |
| `identity-already-exists` | validation issue | 0 | a create reuses an id that already exists | `ekr mint` a fresh id |
| `duplicate-identity` | validation issue | 0 | one transaction creates the same id twice | `ekr mint` one id per created thing |
| `conflicting-write` | validation issue | 0 | one transaction writes the same property of a node twice with different values, or moves one node's lifecycle twice | one write per property and one state move per node per transaction |
| `operation-not-declared` | validation issue | 0 | `!Invoke` names no operation **key** of the node's type (an operation's `name` field is not consulted) | use the key under `operations` |
| `transition-refused` | validation issue | 0 | the node is not in the operation's `from` state | check the node's `type_state` |
| `missing-argument` | validation issue | 0 | `!Invoke` omits a declared argument | pass every declared argument |
| `undeclared-argument` | validation issue | 0 | `!Invoke` passes an argument the operation does not declare | remove it |
| `unsupported-constraint` | validation issue | 0 | the affected type has property `constraints`, or the operation has `preconditions` or `emits` | none in P1: those must be `[]` in the seed |
