---
format: aep.planning-md/2
id: review-result:adversary-ontology-pass-2
kind: review-result
status: active
title: Adversary, story:ontology-types-and-values, pass 2
tags:
- adversary:aep-drive:adversary
relations:
- reviews: story:ontology-types-and-values
revision: 1
---
```
unit: story:ontology-types-and-values — uncommitted working tree at home-path:sha256:6635bceff979fd018d3bee69dc58778211b33c540f98996b434a7afc9f9b8f7d, base 7595dca
verdict: red
cases: executed 138→142, red 4
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (all in the assigned scratch, listed in part 6)
needs-coordinator: yes — finding 4's second fix route amends `systems/ekr/domains/ontology.yaml`, outside the unit's file assignment
```

## 1. What I touched

`git --no-pager diff --stat` — the tracked diff is the implementor's, unchanged by me:

```
 crates/ekr-ontology/src/lib.rs | 63 ++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 63 insertions(+)
```

Most of the unit is untracked. The two paths in `git status --porcelain -uall` that are mine:

```
?? crates/ekr-ontology/tests/domain_projection.rs
?? crates/ekr-ontology/tests/hierarchy_specificity.rs
```

Both are test paths. No implementation file was edited, copied or mutated; no existing case was deleted, skipped or weakened. `cargo fmt --all` ran before reporting and reformatted nothing under `src/`.

## 2. The cases I added, and their red output when written

Four cases in two new files. All four are red **now**. These are the runs of those targets alone, before any suite run.

`cargo test -p ekr-ontology --locked --test hierarchy_specificity -- --test-threads=1`

```
running 3 tests
test a_parent_that_specialises_another_parent_is_not_an_ambiguous_declaration ... FAILED
test a_redeclaration_is_never_overridden_by_the_declaration_it_redeclares ... FAILED
test the_reason_a_two_fault_document_is_refused_does_not_depend_on_id_order ... FAILED

---- a_parent_that_specialises_another_parent_is_not_an_ambiguous_declaration stdout ----
panicked at crates/ekr-ontology/tests/hierarchy_specificity.rs:110:13:
a type naming both a base and a refinement of that base is a well-formed document, and the hierarchy chooses between the two declarations without any id order: 018f2a00-0000-7000-8000-0000000001b0 is a strict descendant of 018f2a00-0000-7000-8000-0000000001a0. Refused with: 018f2a00-0000-7000-8000-0000000001c0 inherits two different declarations of property 018f2a00-0000-7000-8000-0000000001e0 at the same distance, and nothing but an id order could choose between them

---- a_redeclaration_is_never_overridden_by_the_declaration_it_redeclares stdout ----
assertion `left == right` failed: 018f2a00-...-01b0 redeclares p and is a strict descendant of 018f2a00-...-01a0, so 018f2a00-...-01b0's declaration is the more specific one and must win for 018f2a00-...-01d0, whatever breadth-first distance each sits at
  left: Some(String)
 right: Some(Integer)

---- the_reason_a_two_fault_document_is_refused_does_not_depend_on_id_order stdout ----
assertion `left == right` failed: one document, two true faults, and the reason it is refused changed when the ids were swapped: ["CyclicParents", "UnknownType"]
  left: "CyclicParents"
 right: "UnknownType"

test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out
EXIT=101
```

`cargo test -p ekr-ontology --locked --test domain_projection -- --test-threads=1`

```
running 1 test
test every_compound_kind_the_domain_says_property_definition_carries_it_actually_carries ... FAILED

---- every_compound_kind_the_domain_says_property_definition_carries_it_actually_carries stdout ----
panicked at crates/ekr-ontology/tests/domain_projection.rs:92:5:
ekr.ontology.PropertyDefinition declares {"cardinality", "constraints", "enum_variants", "name", "owner_type_id", "ref_allowed_types", "required", "schema_version_id", "value_kind"}, and carries the parameters of no such kind as ["List", "Record"] — while the document's comment at its lines 26-28, quoted by crates/ekr-ontology/src/value.rs, says the parameters of all four compound kinds are carried there

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
EXIT=101
```

Order note: the first `domain_projection` run panicked at the fixture (`the domain declares ekr.ontology.PropertyDefinition`) because the domain puts that declaration under `entities:` and not `types:`. That is a typo, not a finding; I fixed the lookup and re-ran before any suite run. The output above is the fixed form, and it is the first output that is about the implementation.

All three hierarchy cases quantify over all six permutations of their fixed ids, so neither the red nor a future green is an accident of which id sorted first.

## 3. The suite, after the cases existed

`export CARGO_TARGET_DIR=home-path:sha256:b75f9a061b971f23993e7f85d8bf63ed6eff15c82c05ab13d3f33e64656bc3fe && task check` — final lines verbatim:

```
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-ontology --test domain_projection`
task: Failed to run task "check": task: Failed to run task "test": exit status 101
EXIT=201
```

`fmt-check` (`cargo fmt --all -- --check`) and `clippy` (`cargo clippy --workspace --all-targets --locked -- -D warnings`) both passed; `test` is the step that fell over. Counts from `cargo test --workspace --locked --no-fail-fast`: **passed=138 failed=4 total=142**. The 138 passing is exactly the implementor's declared `138`, so `<before>` is confirmed from two sources.

## 4. Findings

Base read with `git show 7595dca:<path>`; at the base `crates/ekr-ontology` held only `Cargo.toml` and a 5-line `lib.rs`, and `git diff --stat 7595dca -- systems/` is empty.

| # | file:line | carried / new | What is wrong | What was measured | What reaches it | Verdict / origin |
|---|---|---|---|---|---|---|
| 1 | `crates/ekr-ontology/src/schema.rs:350` | new | The new `AmbiguousProperty` refusal fires on a document that is not ambiguous. `resolve_properties` compares two declarations by **breadth-first distance from the starting type**, and calls it ambiguous when they tie. But two ancestors at one distance are only ambiguous when neither is an ancestor of the other. `child(parents = {parent, grand})` with `parent(parent = grand)` — a type naming both a base and a refinement of that base — puts both declarations at distance 1, and the document is refused at load. The refusal's own text says "nothing but an id order could choose between them", which is untrue of this document: `parent` is a strict descendant of `grand`, and the hierarchy the document declares chooses without any id. | `Ontology::load` returns `Err(AmbiguousProperty { type_id: child, property })` for all six id permutations. `hierarchy_specificity.rs:110`, exit 101. | `Ontology::load` / `from_yaml`. `NodeType::parents` is a public `BTreeSet<TypeId>` (design § 11.1) with no rule against a redundant superclass assertion, and property redeclaration is the feature this correction round is about. A document of that shape is now unloadable, deterministically, with a diagnostic that names the wrong cause. Fix: order the merge by the specialisation partial order — a declaration in `V` beats one in `W` when `conforms_to(V, W)` — and reserve `AmbiguousProperty` for the case where neither declaring type is an ancestor of the other. One change closes this and finding 2. | `CONFIRMED` / `introduced` |
| 2 | `crates/ekr-ontology/src/schema.rs:349` | new | The same disagreement without a refusal: the nearer-wins arm silently prefers a declaration that a strict descendant of it has already redeclared. `grand` declares `p: String`; `mid(parent = grand)` redeclares `p: Integer`; `parent(parent = mid)` declares nothing; `child(parents = {grand, parent})`. `grand` sits at distance 1 and `mid` at distance 2, so `properties_of(child)` returns `grand`'s `String` and never looks at `mid`'s redeclaration — although `child` conforms to `mid` and reaches `grand` only through it. No error, no diagnostic. `properties_of`'s doc comment does say "'Nearest' is distance in the hierarchy and nothing else", so the code matches its own prose; the prose is the thing that is wrong, because a redeclaration that a descendant never sees is not a redeclaration. | `properties_of(child)[p].value_type` is `String`, expected `Integer`, for all six id permutations; `check_node(child, {p: Integer(1)})` is refused. `hierarchy_specificity.rs:166`, exit 101. | `Ontology::load` accepts the document, so every later `properties_of` and `check_node` on `child` carries the wrong declaration. Same entry points and the same public feature as finding 1. Same fix. | `CONFIRMED` / `introduced` |
| 3 | `crates/ekr-ontology/src/schema.rs:108` | new | `check_declarations` walks `node_types` in `TypeId` order and returns the first fault, so a document carrying **two** faults reports whichever belongs to the lower-sorting type. A document with a self-looping type and a type naming an undeclared parent reports `CyclicParents` or `UnknownType` depending only on which id sorts first. The implementor reserved first-error-wins deliberately and argued both reasons are true — they are. What is not deliberate is that the class case `inheritance_and_declaration_coherence.rs:552` is named `the_reason_a_malformed_hierarchy_is_refused_never_depends_on_id_order` and every document it enumerates carries exactly one fault, so its name asserts a rule the crate does not hold, in the exact place a later reader will look for it. | Two ids swapped over one document shape gives `["CyclicParents", "UnknownType"]`. `hierarchy_specificity.rs:222`, exit 101. | `Ontology::load` / `from_yaml` on any document with more than one declaration fault — which is the normal state of a document being written. Fix, cheapest route: narrow the existing case's name and doc comment to the single-fault rule it actually proves, and record multi-fault ordering as design § 20's return-contract question. I did not weaken or rename that case; the rename is the implementor's to make. | `CONFIRMED` / `introduced` |
| 4 | `crates/ekr-ontology/src/value.rs:121` | new (supersedes pass-1 #4) | The round-1 correction did not remove the false attribution to the ESS domain; it replaced it with a **quotation** of a domain sentence that is itself false. `value.rs` now justifies the shape difference by saying "the domain says at its own lines 26-28 that the compound kinds' parameters `are carried by PropertyDefinition below`". The document does say that, naming four kinds — "the allowed types of a NodeRef, the variants of an Enum, the element type of a List, the fields of a Record". `ekr.ontology.PropertyDefinition` carries `ref_allowed_types` and `enum_variants` and nothing for `List` or `Record`. The consequence is not cosmetic: `ValueType::List(..)` and `ValueType::Record(..)` are constructible, `Ontology::load` accepts them, and the domain the story `implements` has nowhere to put them — so the P3 store cannot round-trip a property type this crate declares loadable, and the one sentence that would have warned it says the opposite. | `ekr.ontology.PropertyDefinition` declares `{cardinality, constraints, enum_variants, name, owner_type_id, ref_allowed_types, required, schema_version_id, value_kind}`; no carrier for `List` or `Record`. `domain_projection.rs:92`, exit 101. | Any reader of `value.rs`, and specifically the P3 store implementor. Nothing compares the two halves: `ess specify validate` checks the document against itself and the suite checks the crate against itself. `git diff --stat 7595dca -- systems/` is empty, so the **document** half reproduces at the base; the sentence that cites it is the unit's. Two fixes, and the choice is the coordinator's: (a) in scope — rewrite `value.rs:117-123` to say the projection covers `NodeRef` and `Enum` only and that a `List` or `Record` property has no domain representation yet; (b) out of scope — amend `ontology.yaml`'s comment at 26-28 to stop claiming four. | `CONFIRMED` / `introduced` |
| 5 | `crates/ekr-ontology/src/schema.rs:356` | new | The comment `// The same declaration reached by two paths is one declaration.` misdescribes the arm below it. A type reached by two paths is dropped by the `seen` set at `schema.rs:340` and never reaches this match at all; the arm is reached only when two *different* types at one distance declare equal `PropertyDefinition`s. The fixture that exercises it (`inheritance_and_declaration_coherence.rs:509`, "one declaration reached by two paths is one declaration") is likewise two independent equal declarations and not a diamond, so no case in the suite constructs the diamond the comment names. Residue: a reader maintaining the specificity fix findings 1 and 2 ask for will reason about diamonds from this comment and be reasoning about the wrong code. | Read, not run. `seen.insert(at)` at `schema.rs:340` is unconditional and precedes the property loop. | Nobody at runtime — it is a comment. It reaches the next person who edits `resolve_properties`, which findings 1 and 2 guarantee is soon. | `CONFIRMED` / `introduced` |

### My four pass-1 findings

| pass-1 | now |
|---|---|
| 1, `schema.rs:283` merge by `TypeId` sort order | **resolved as filed** — no answer depends on mint order; all six permutations agree in every case I ran. The replacement rule introduces findings 1 and 2 above, which are a different defect at a different line, not the same one carried. |
| 2, `ancestors` conflating absent and cyclic | **resolved** — `ancestors` returns `Result` and names each itself; my two-fault case confirms it reports `UnknownType` for the absent type and `CyclicParents` for the real cycle, each correctly. Finding 3 is about *which of two true faults* is chosen, not about a false one. |
| 3, the walk missing operation arguments | **resolved** — `check_value_type` carries the walk and all three declaration positions call it (`schema.rs:157`, `schema.rs:230`); `EmptyValueType` carries a `DeclarationSite`; `ontology_load.rs:61` and `:104` were updated to the new shape and still assert the exact site, not `..`. Not weakened. |
| 4, the false citation in `value.rs` | **resolved as filed, superseded** — the `parameters` attribution is gone. Finding 4 above is the replacement sentence. |

On the re-aimed case (`value_rs_does_not_attribute_its_serde_shape_to_the_ess_domain`, `inheritance_and_declaration_coherence.rs:200`): the re-aim is honest, not a weakening. It pins the coordinator's decision from both sides, keeps the original name and reason in its doc comment, and would go red if anyone "reconciled" the two by amending the document. I do not raise it.

## 5. Attacked and could not break

- **The BFS at the shapes the brief named** — a diamond reaching one ancestor at two different distances, an ancestor at distance 2 and 3, a redeclaration at distance 1 against two identical ones at distance 2: all resolve correctly, because `seen` dedupes at the type level before the property loop.
- **`AmbiguousProperty` firing where only one of two same-distance ancestors declares the property** — does not fire; the `None` arm inserts and there is nothing to compare.
- **`AmbiguousProperty` masked by a nearer declaration** — `child`'s own declaration at distance 0 correctly suppresses an ambiguity at distance 1. Right answer.
- **`AmbiguousProperty` naming** — names the type whose set cannot be resolved and the property, and both reach the `Display` string.
- **`ancestors` with both faults present, within one type's walk** — reports `UnknownType` for the absent ancestor regardless of DFS pop order for every shape I could build; the order dependence I found is in `check_declarations`' outer loop (finding 3), not here.
- **`UnknownType { type_id }` naming the absent type** — it does, not the type that pointed at it (`declared(next)?` carries `next`).
- **`the_crate_declares_no_value_type_field_the_walk_does_not_reach`** — its stated bound is real (a new container of `PropertyDefinition` escapes it), but the implementor states it in the case's own doc comment, which is the honest form. I did not raise it as a finding; it is the bound, written down.
- **`every_domain_name_this_crate_cites_is_declared_by_the_domain`** — I extracted every `ekr.<domain>.<Name>` occurrence from the crate's `src/` mechanically and every one is declared by the document. The hand-kept `cited` list is a bound, admitted in the doc comment; I could not make it pass on a false citation that a mechanical extraction would have caught.
- **`ValueKind` variant names and `Cardinality`'s `Display`** — match the domain's declared variants exactly, as `value.rs:92` claims.
- **The acceptance oracle** — unchanged since pass 1 and still independent; the property suite declares no hierarchy, so findings 1 and 2 are invisible to it by construction rather than by a gap in it.
- **`check_value_type`'s `reachable` walk** — `List` and `Record` are the only `ValueType`-containing variants and both recurse; an empty `Record` and a non-empty `List` are correctly *not* refused.
- **`EmptyValueType`'s reshaping** — `ontology_load.rs:61`, `:104` and `:404` were updated, not weakened; two of the three still assert the full `DeclarationSite`.

## 6. Paths written outside the worktree

All four in the assigned scratch directory `home-path:sha256:6d2a91e475a794c4b6120c0290acb817ec4a0815ce735f935d2e4e81fcfe351b`, none elsewhere, nothing under `/tmp`:

- `adversary2-red-cases.log` — the `domain_projection` run quoted in part 2
- `adversary2-red-hierarchy.log` — the `hierarchy_specificity` run quoted in part 2
- `adversary2-suite.log` — the `--no-fail-fast` run the 138/142 counts come from
- `adversary2-gate.log` — the `task check` run quoted in part 3

Build output went to the assigned `home-path:sha256:b75f9a061b971f23993e7f85d8bf63ed6eff15c82c05ab13d3f33e64656bc3fe`. Lease `ekr-adversary-ontology-2` taken at start and released before this report. No `aep plan artifact` command was run; no commit, no branch operation, no worktree operation.

```findings
- file: crates/ekr-ontology/src/schema.rs
  line: 350
  category: acceptance
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "resolve_properties refuses a well-formed document with AmbiguousProperty when a type names both a base and a refinement of that base, because it compares declarations by breadth-first distance rather than by the specialisation order the document itself declares"
- file: crates/ekr-ontology/src/schema.rs
  line: 349
  category: property
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "the nearer-wins arm silently prefers a declaration that a strict descendant of it has already redeclared, so a type inheriting through a longer branch never sees the redeclaration on that branch"
- file: crates/ekr-ontology/src/schema.rs
  line: 108
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "check_declarations returns the first fault in TypeId order, so a document with two faults reports a reason decided by id order, while the case at inheritance_and_declaration_coherence.rs:552 is named for the general rule and enumerates only single-fault documents"
- file: crates/ekr-ontology/src/value.rs
  line: 121
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the corrected citation quotes a domain sentence that claims PropertyDefinition carries the parameters of all four compound kinds, and it carries NodeRef's and Enum's only, so a List or Record property this crate loads has no representation in the domain the story implements"
- file: crates/ekr-ontology/src/schema.rs
  line: 356
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the comment 'the same declaration reached by two paths is one declaration' describes a case the seen set at schema.rs:340 has already dropped, and the arm it sits on is reached only by two distinct types declaring equal definitions at one distance"
```
