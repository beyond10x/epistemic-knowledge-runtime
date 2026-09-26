---
format: aep.planning-md/2
id: architecture-decision-record:0005-float-is-not-canonical
kind: architecture-decision-record
status: accepted
title: ADR 0005 — A Float is not admissible in canonical state
relations:
- decides: story:eventlog-store
- decides: story:transaction-and-validators
revision: 3
---
## Status

Proposed 2026-09-21, before wave p1-05 dispatched, and accepted the same day. Taken by the
coordinator because both units of that wave need an answer and neither story owns the surface.

## The question

`crates/ekr-core/src/canonical.rs` rule 4 admits no float: `NaN` is not equal to itself, `0.0 ==
-0.0` holds for two different bit patterns, and no encoding of either is both total and faithful to
equality. `ekr_ontology::Value` has a `Float(f64)` variant, so `Value` has no `Canonical`
implementation, so `ekr_graph::Assertion` has none either — it carries `Object::Value(Value)`, and
`Node.properties` and `Edge.properties` carry `Value` as well.

Wave p1-04's implementor recorded this as a real constraint that "the kernel's knowledge-root
hashing will meet in a later wave". That wave is p1-05, and it meets it in both units at once:

- `story:eventlog-store`'s acceptance is that a store reopened folds to the same head `Root` hash.
  `Root.knowledge_root` and `Root.evidence_root` are content addresses of graph and evidence state
  (`crates/ekr-graph/src/root.rs:57-59`), which means hashing assertions, which means hashing
  `Value`.
- `story:transaction-and-validators` ships a case that a valid transaction produces the same
  `validation_hash` on two runs. A transaction's operations carry values.

## Decision

**A `Float` is not admissible in canonical state.** Rule 4 already says what to do with a quantity
that must be hashed — carry it as an integer or a decimal string — and says that is what
`ekr.ontology.ValueKind` distinguishes `Float` from `Decimal` for. This record makes that
enforceable rather than advisory.

- `ekr-ontology` answers whether a value is admissible, recursively: a `List` or a `Record`
  containing a `Float` is not admissible either.
- `ekr-graph` gains a newtype that only an admissible value can inhabit, with a `TryFrom<Value>`
  that refuses the rest and a **total** `Canonical` implementation. `Object::Value`,
  `Node.properties` and `Edge.properties` carry the newtype; `Assertion` then has a `Canonical`
  implementation and `knowledge_root` becomes computable.
- `ekr-kernel`'s type validator refuses an operation carrying an inadmissible value, with an issue
  naming it. That is the layer design § 20 puts type checking at.
- `Float` stays legal in `ekr_ontology::Value` and in the transient graph. Nothing there is
  content-addressed, and an approximate measurement is a perfectly good thing to hold before it
  becomes canonical.

**Unrepresentable rather than refused**, which is the shape this repository has taken twice already
— `AGENTS.md` invariant 2 for the canonical/transient membrane, and wave p1-04's `TransactionTime`
for a belief the runtime never formed. A fallible `Assertion::content_hash` returning an error that
validation makes unreachable is the alternative, and an unreachable error is a claim nobody can
check.

## What this does not settle

- **No decimal arithmetic.** `ValueKind::Decimal` is a string today and stays one. What a valid
  decimal string is, and whether two spellings of one number are one value, is not decided here.
- **Nothing about `f64` inside evidence payloads or observations.** `ObservationContent::Blob`
  (amendment 86) is bytes and hashes as bytes.
- **No migration.** Nothing is stored yet.

## Cost of deciding now against later

Now: `Object::Value`, two property maps, and the fixtures in `ekr-graph`'s test files. Nothing is
persisted, so no address moves.

Later: every address the store has written and every `validation_hash` the kernel has recorded,
plus a migration for both. `story:commit-and-revision-lineage` in wave p1-06 writes the first root
hash anybody keeps.


## Amendment, 2026-09-21, after adversary pass 1 on unit 0

The decision said "`Float` stays legal in `ekr_ontology::Value` and in the transient graph. Nothing
there is content-addressed." The first implementation made that sentence false, and the adversary
measured it: `TransientGraph` is composed of the same `Node`, `Edge` and `Assertion` that canonical
state holds (`crates/ekr-graph/src/transient.rs:124-128`), so moving those three to the admissible
newtype made a float unholdable anywhere, incubation forest included.

That is the wrong answer. The incubation forest exists to hold candidate knowledge that is not yet
canonical, and an imported approximate measurement is exactly such a candidate. Refusing it at the
boundary would mean refusing an import for carrying a number the runtime cannot yet address, which
inverts what the forest is for.

**`Node`, `Edge` and `Assertion` are generic over the value they carry**, defaulting to the
admissible newtype. `CanonicalGraph` holds the default. `TransientGraph` holds the same three types
over `ekr_ontology::Value`, so a candidate may carry a float.

`Canonical` is implemented only where the value parameter is itself `Canonical`. That is the part
worth stating plainly: **the address exists exactly where canonical state does**, so "only canonical
state can be content-addressed" stops being a convention and becomes a property of the type system —
the same shape `AGENTS.md` invariant 2 already asks for on the reference direction. The generic was
proposed by the adversary as the more expensive of its two options; it is taken because the cheaper
one, amending this sentence away, would have made the membrane weaker rather than described it.

The other option was to say the constraint is global. It is rejected: nothing about transient state
needs it, and the sentence this amendment is fixing was right about why.
