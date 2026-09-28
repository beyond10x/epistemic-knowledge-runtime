---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-02-integrate-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit T (typed reference resolver)
relations:
- reviews: story:typed-reference-resolver
revision: 1
---
unit: story:typed-reference-resolver (unit T), worktree ekr-wave-a-t at head 7039386e plus one untracked adversary test file
verdict: red (3 red cases; strongest finding confirmed, medium)
cases: executed 16→21, red 3
origin: introduced 4, pre-existing 1, undecided 0
wrote-outside-worktree: <wave-root>/t/scratch/mutant/ (created, then deleted), <wave-root>/t/scratch/adversary-pass-1.md, <wave-root>/t/target (the assigned build dir)
needs-coordinator: F1 and F2 need a spec decision in systems/ekr/domains/integrate.yaml (you own that file); F3 needs an aliases field on ekr-kernel NodeDraft (another crate)

## 1. git --no-pager diff --stat

`git diff --stat` prints nothing: the only change is one untracked file. `git status --short`:

```
?? crates/ekr-integrate/tests/adversary_resolve.rs
```

It is a test file. No implementation file and no file written by the implementor was changed.

## 2. Cases added (crates/ekr-integrate/tests/adversary_resolve.rs)

Each case was run on its own (`cargo test -p ekr-integrate --locked --test adversary_resolve -- --exact <name>`) before the suite ran.

| case | asserts | now |
|---|---|---|
| `a_reference_whose_only_alias_is_empty_is_refused_as_without_identity` | a reference with aliases `[""]` is refused with `reference-without-identity` | red |
| `an_empty_alias_does_not_make_an_unrelated_node_a_candidate` | `["ada", ""]` resolves to the one node that holds `ada`, not to an ambiguity across every node that holds `""` | red |
| `a_reference_to_an_undeclared_type_is_not_proposed_as_a_new_node` | a type the ontology does not declare does not produce `ProposeNew` | red |
| `a_repeated_alias_is_carried_once_in_a_proposal_and_a_refusal` | `["b","a","b","a"]` produces `ProposeNew` with `["a","b"]`; this catches the dedup mutant | green (red against the mutant) |
| `every_outcome_round_trips_through_its_wire_form` | each of the 4 outcomes deserialises back to itself | green |

Red output from each case run on its own, verbatim (line numbers are from before `cargo fmt`, which later moved the second assert to :96):

```
---- a_reference_whose_only_alias_is_empty_is_refused_as_without_identity stdout ----
panicked at crates/ekr-integrate/tests/adversary_resolve.rs:79:5:
assertion `left == right` failed
  left: Resolved(ResolvedReference { node_id: NodeId(1737840089068066293415937) })
 right: Refused(ResolutionRefusal { code: ReferenceWithoutIdentity, reference: TypedReference { type_id: TypeId(2946765908682695468122113), aliases: [""] } })
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
exit=101

---- an_empty_alias_does_not_make_an_unrelated_node_a_candidate stdout ----
panicked at crates/ekr-integrate/tests/adversary_resolve.rs:93:5:
assertion `left != right` failed: the empty alias matched every node that carries one
  left: Ambiguous(AmbiguousReference { candidates: [NodeId(1737840089068066293415937), NodeId(1737840089068066293415938), NodeId(1737840089068066293415939)] })
 right: Ambiguous(AmbiguousReference { candidates: [NodeId(1737840089068066293415937), NodeId(1737840089068066293415938), NodeId(1737840089068066293415939)] })
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
exit=101

---- a_reference_to_an_undeclared_type_is_not_proposed_as_a_new_node stdout ----
panicked at crates/ekr-integrate/tests/adversary_resolve.rs:117:5:
an undeclared type was proposed as a new node: ProposeNew(TypedReference { type_id: TypeId(2946765908682695468122211), aliases: ["ada"] })
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out
exit=101
```

## 3. Suite run (after the cases existed)

`CARGO_TARGET_DIR=<wave-root>/t/target nice -n 19 cargo test -p ekr-integrate --locked --no-fail-fast`, exit 101:

```
Running tests/adversary_resolve.rs   test result: FAILED. 2 passed; 3 failed
Running tests/domain_projection.rs   test result: ok. 3 passed; 0 failed
Running tests/manifest.rs            test result: ok. 1 passed; 0 failed
Running tests/resolve.rs             test result: ok. 11 passed; 0 failed
Doc-tests ekr_integrate              test result: ok. 1 passed; 0 failed
```

The count before my cases is 16. Two sources agree: the implementor's commit message says 16, and 21 minus my 5 is 16.

Other checks: `cargo fmt -p ekr-integrate -- --check` exit 0 after formatting my file. `cargo clippy -p ekr-integrate --all-targets --locked -- -D warnings` exit 0.

Mutation probes. Each ran in a `git archive HEAD` copy under scratch, which I deleted afterwards:

| mutant | implementor suite | adversary file |
|---|---|---|
| remove `aliases.dedup()` (lib.rs:176) | **green** (11/3/1/1): the mutant survives | `a_repeated_alias_…` red |
| remove the `node.root_id == graph.root.id` filter (lib.rs:209) | red: `a_node_outside_the_canonical_root_is_not_a_candidate` | — |
| rewrite `serde.workspace = true` as a `[dependencies.serde]` table in Cargo.toml | red: the scan no longer sees `serde`, so a table-form dependency is invisible to the guard (F4) | — |

## 4. Findings (they cover head 7039386e)

| id | file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr-integrate/src/lib.rs:189 | The resolver treats an empty-string alias as an identity. `[""]` resolves to any node that holds `""`. Adding `""` to a real alias turns a unique match into `Ambiguous` across every node that holds `""`. Fix: drop empty strings before the `is_empty()` refusal (this needs a spec line in integrate.yaml). | confirmed | introduced | Any caller of the public `resolve`; no caller exists yet. Nodes get aliases only through a seed document (kernel seed.rs:394), and I found no check there that refuses `""`. |
| F2 | crates/ekr-integrate/src/lib.rs:218 | A type the ontology does not declare falls through to `ProposeNew`. That proposes a node the kernel will not admit (`Ontology::conforms_to`: an undeclared type conforms to nothing). The implementor stated this choice. integrate.yaml has no refusal code for it. | confirmed | introduced | Any caller holding a stale or foreign `TypeId`; no caller yet. |
| F3 | crates/ekr-kernel/src/transaction.rs:62 (with apply.rs:30) | `NodeDraft` has no `aliases` field, and `CreateNode` always builds a node with no aliases. So a `ProposeNew` committed through the only write path produces a node the resolver can never match. The next identical reference gets `ProposeNew` again, which yields duplicate nodes: the exact defect the story sets out to prevent. I did not write a case: that needs an ekr-kernel dev-dependency, and Cargo.toml is not a test file. | plausible | pre-existing | The story's own documented workflow: the caller mints the id and commits through a transaction. The base f5e01f7a transaction.rs contains 0 occurrences of `aliases`. |
| F4 | crates/ekr-integrate/tests/manifest.rs:24 | The dependency scan only recognises section headers that end in `dependencies`. `[dependencies.strsim]` passes both the allow-list and the similarity list. Transitive similarity crates are not checked at all. | confirmed (scratch probe) | introduced | Whoever adds a dependency in table form. |
| F5 | crates/ekr-integrate/src/lib.rs:176 | Removing `dedup()` leaves the implementor's suite green. The property test's `wanted` is a subsequence, so it never repeats an alias. My green case now catches the mutant. | confirmed | introduced | Any reference with a repeated alias. |

## 5. Attacked and not broken

- The acceptance statement: same name with different type never cross-resolves.
- Ambiguous lists candidates in id order.
- Case and whitespace variants do not match.
- `canonical_name` is never compared.
- Refusal for abstract types and for types with descendants. A direct-parent check is enough to detect a descendant.
- Wire round-trip of all 4 outcomes.
- Refusal codes and field names match integrate.yaml.
- Sort-order mutant: caught by the property test.
- Root-filter mutant: caught.
- A Transient root inside a CanonicalGraph: the kernel seed refuses it (`seed-space`), so nobody reaches it.
- Merge: `MergeEntity` is `unsupported-operation` (apply.rs:147), so absorbed nodes cannot produce spurious ambiguity yet.

## 6. Paths written outside the worktree

- <wave-root>/t/scratch/mutant/ (a `git archive` copy for the mutants; deleted)
- <wave-root>/t/scratch/adversary-pass-1.md (this report)
- <wave-root>/t/target (the assigned build dir; the mutant copy also built into it, and the worktree was rebuilt afterwards)

## Scope confirmation

| scope line | status | touched |
|---|---|---|
| crates/ekr-integrate | confirmed | crates/ekr-integrate/tests/adversary_resolve.rs only |
| Cargo.toml, Cargo.lock, README.md, crates/ekr/tests/adversary_docs_contract.rs | not touched | — |


## Coordinator routing (2026-09-27)

- F1 → back to the implementor: an empty-string alias is no identity. Empty strings are dropped
  before the no-alias check, so `[""]` is refused as `reference-without-identity` and `["ada", ""]`
  resolves as `["ada"]` would.
- F2 → back to the implementor, with a spec change granted to the unit: `integrate.yaml`
  `ekr.integrate.ResolutionRefusalCode` gains `reference-type-undeclared`, and a reference whose
  type the ontology does not declare is refused with it.
- F3 → pre-existing, filed as its own task; it does not block the unit. `NodeDraft` has no aliases
  field (`crates/ekr-kernel/src/transaction.rs:62`, reported by the adversary, not re-read by the
  coordinator), so what a caller commits after `ProposeNew` decides whether the reference matches
  again.
- F4 → back to the implementor: the manifest guard reads the manifest as TOML, table-form
  dependencies included.
- F5 → no-op: the adversary's case kills the mutant and stays.

```findings
[
{"file":"crates/ekr-integrate/src/lib.rs","line":189,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"an empty-string alias counts as an identity, so [\"\"] resolves and \"\" turns a unique match into an ambiguity over every node holding \"\""},
{"file":"crates/ekr-integrate/src/lib.rs","line":218,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a reference to a type the ontology does not declare yields ProposeNew for a node the kernel cannot admit"},
{"file":"crates/ekr-kernel/src/transaction.rs","line":62,"category":"correctness","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"NodeDraft carries no aliases, so a committed ProposeNew is never matched again and repeated references mint duplicate nodes"},
{"file":"crates/ekr-integrate/tests/manifest.rs","line":24,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the no-similarity-dependency guard does not see table-form dependencies such as [dependencies.strsim]"},
{"file":"crates/ekr-integrate/src/lib.rs","line":176,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"removing dedup() leaves the implementor's suite green; only the adversary case catches it"}
]
```
