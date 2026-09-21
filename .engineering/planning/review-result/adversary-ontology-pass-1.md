---
format: aep.planning-md/1
id: review-result:adversary-ontology-pass-1
kind: review-result
status: active
title: Adversary, story:ontology-types-and-values, pass 1
tags:
- adversary:aep-drive:adversary
relations:
- reviews: story:ontology-types-and-values
revision: 1
---
```
unit: story:ontology-types-and-values — uncommitted working tree at $HOME/.local/state/worktree/trees/b10x/epistemic-knowledge-runtime/ekr-impl-ontology, base 7595dca
verdict: red
cases: executed 125→129, red 4
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: 2 paths (both in assigned scratch, listed in part 6)
needs-coordinator: no
```

## 1. What I touched

`git --no-pager diff --stat` — the tracked diff is the implementor's, unchanged by me:

```
 crates/ekr-ontology/src/lib.rs | 63 ++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 63 insertions(+)
```

Most of the unit is untracked, so the full picture is `git status --porcelain -uall`. The one line that is mine:

```
?? crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs
```

One path, a test path. No implementation file was edited, copied or mutated. `cargo fmt --all -- --check` exits 0 with my file in the tree, and no src file was reformatted.

## 2. The cases I added, and their red output when written

All four are in `.../ekr-impl-ontology/crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs`. All four are red **now**. This is the run of that target alone, before the suite:

`cargo test -p ekr-ontology --locked --test inheritance_and_declaration_coherence -- --test-threads=1`

```
running 4 tests
test a_redeclared_property_is_resolved_by_the_hierarchy_and_not_by_type_id_order ... FAILED
test an_operation_argument_no_value_inhabits_is_refused_at_load ... FAILED
test an_undeclared_grandparent_is_refused_as_an_unknown_type_and_not_as_a_cycle ... FAILED
test the_ess_domain_names_the_serde_content_key_value_rs_cites_it_for ... FAILED

---- a_redeclared_property_is_resolved_by_the_hierarchy_and_not_by_type_id_order stdout ----
assertion `left == right` failed: two hierarchies that differ only in which TypeId sorts higher must decide the same value the same way; A gave Err(CheckError { property: Some(PropertyId(2072578307759278508210927257031540960)), reason: WrongKind { expected: String, found: Integer } }) and B gave Ok(())
  left: Some(CheckError { property: ..., reason: WrongKind { expected: String, found: Integer } })
 right: None

---- an_operation_argument_no_value_inhabits_is_refused_at_load stdout ----
an argument allowed to point at nothing leaves the checker unable to answer: Ontology { ... operations: {"attach": OperationDefinition { name: "attach", arguments: {"target": NodeRef { allowed_types: {} }}, ... }} ... }

---- an_undeclared_grandparent_is_refused_as_an_unknown_type_and_not_as_a_cycle stdout ----
assertion `left == right` failed: the document declares no cycle; 018f2a00-0000-7000-8000-0000000000c2 is simply absent
  left: CyclicParents { type_id: TypeId(2072578307759278508210927257031540898) }
 right: UnknownType { type_id: TypeId(2072578307759278508210927257031540930) }

---- the_ess_domain_names_the_serde_content_key_value_rs_cites_it_for stdout ----
value.rs attributes the `parameters` content key to this document, which declares no such name anywhere; ekr.ontology.PropertyDefinition carries the flat `value_kind`, `ref_allowed_types` and `enum_variants` and no `value_type` at all. Declared names: {... "enum_variants", ... "ref_allowed_types", ... "value_kind"}

test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
EXIT=101
```

Note on order: an earlier draft of the fourth case asserted `text.contains("parameters")` and passed on a prose occurrence in a YAML comment. I tightened it to a structural scan of every declared field/variant name before any suite run; the run above is the tightened form.

## 3. The suite, after the cases existed

`export CARGO_TARGET_DIR=$HOME/.cache/b10x-target/epistemic-knowledge-runtime && task check` — final lines verbatim:

```
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-ontology --test inheritance_and_declaration_coherence`
task: Failed to run task "check": task: Failed to run task "test": exit status 101
EXIT=201
```

`fmt-check` and `clippy --workspace --all-targets -- -D warnings` both passed; `test` is the step that fell over. Counts come from `cargo test --workspace --locked --no-fail-fast`: **passed=125 failed=4 total=129**. The 125 passing are exactly the implementor's reported `125`, so `<before>` is confirmed from two independent sources.

## 4. Findings

Base read with `git show 7595dca:...`; at the base `crates/ekr-ontology` held only `Cargo.toml` and a 5-line `lib.rs`, so every finding below is `introduced`.

| # | file:line | What is wrong | What was measured | What reaches it | Verdict / origin |
|---|---|---|---|---|---|
| 1 | `crates/ekr-ontology/src/schema.rs:283` | `properties_of` merges ancestors' declarations by walking `ancestors()` — a `BTreeSet<TypeId>` — so among ancestors the **highest TypeId wins**, not the nearest one in the hierarchy. A grandparent silently overrides the parent that redeclared the property. The doc comment two lines up claims the opposite: "rather than depending on which id sorted first". | Two ontologies identical but for id assignment: `G(String) <- P(Integer) <- C`. `check_node(C, {p: Integer(1)})` returns `Err(WrongKind{expected: String, found: Integer})` when `G > P`, and `Ok(())` when `G < P`. `inheritance_and_declaration_coherence.rs:94`, exit 101. | `Ontology::check_node` (`check.rs:154`) calls it on every node check. `NodeType.parents` and property redeclaration are both public, documented features; `TypeId::mint()` is a UUIDv7, so which answer a real ontology gets is decided by mint order. Fix: order the merge by hierarchy distance (BFS from the type outward, nearest written last), or refuse an ambiguous redeclaration at load. | `CONFIRMED` / `introduced` |
| 2 | `crates/ekr-ontology/src/schema.rs:320` | `ancestors()` returns `None` both for a cycle and for an ancestor it cannot look up (`self.node_types.get(&next)?`), and `check_declarations:115` reads every `None` as `CyclicParents`. `check_declarations:111` only checks a type's **direct** parents, so an undeclared *grand*parent is reported as a cycle that does not exist — and only when the descendant's id sorts first. | A document declaring `C(parent P)` and `P(parent G)` with `G` absent returns `CyclicParents { type_id: C }` instead of `UnknownType { type_id: G }`. `inheritance_and_declaration_coherence.rs:132`, exit 101. | `Ontology::load` / `Ontology::from_yaml` on any hierarchy deeper than two levels with a missing ancestor. The document is still refused, so nothing unsafe loads — the refusal reason is wrong, and it is wrong non-deterministically. The existing case `ontology_load.rs:158` only covers a direct parent. Fix: have `ancestors` distinguish "not declared" from "cycle", or check every reached parent for declaredness before walking. | `CONFIRMED` / `introduced` |
| 3 | `crates/ekr-ontology/src/schema.rs:120` | `check_properties` is called on a node type's `properties` and an edge type's `properties`, and nowhere else. `OperationDefinition::arguments` (amendment 87) is a `BTreeMap<String, ValueType>` that never reaches it, so an argument typed `NodeRef { allowed_types: {} }` — or naming an undeclared `TypeId`, or `Enum { variants: {} }` — loads intact. `lib.rs:16-18` states the refusal without qualification: "a declaration that would leave the checker unable to answer — a `NodeRef` allowed to point at nothing ... — is refused before any value is checked against it". | `Ontology::load` returns `Ok` for a node type whose operation `attach` takes `target: NodeRef { allowed_types: {} }`; the loaded `Ontology` is printed in the failure with that argument type still in it. `inheritance_and_declaration_coherence.rs:170`, exit 101. | `Ontology::load` / `from_yaml`; `operations` and `arguments` are public serde-read fields and the shape amendment 87 specifies. Nothing in this crate consumes `arguments` yet, so the cost lands on design § 20 validator 5, which is the declared consumer. Fix: run `check_properties`' inhabitability walk over `operation.arguments.values()` too. | `CONFIRMED` / `introduced` |
| 4 | `crates/ekr-ontology/src/value.rs:113` | The doc comment says `ValueType`'s `value_kind`/`parameters` serde shape "is the naming `systems/ekr/domains/ontology.yaml` gives — a schema document reads `value_kind: NodeRef` with `parameters: { allowed_types: [...] }`". The document gives no such naming. It declares `ekr.ontology.PropertyDefinition` with flat `value_kind`, `ref_allowed_types`, `enum_variants` and no `value_type` field at all, and says at its line 26-28 that the compound kinds' parameters "are carried by PropertyDefinition below". | Structural scan of every declared field and variant name in the domain: `parameters` is absent; `ref_allowed_types`, `enum_variants`, `value_kind` are present. `inheritance_and_declaration_coherence.rs:204`, exit 101. | Any reader of `value.rs`, and specifically the P3 store implementor who has to persist a `PropertyDefinition` — one half of the repository tells them `parameters`, the other `ref_allowed_types`. `ess specify validate` checks the document against itself, and the suite checks the code against itself; nothing compares the two. Fix is a doc line plus a decision: either correct the citation (the wire shape is this crate's choice, not the domain's), or amend `ontology.yaml` so the two agree. **`ontology.yaml` is outside the unit's file assignment**, so the decision is the coordinator's if the second route is taken. | `CONFIRMED` / `introduced` |

## 5. Attacked and could not break

- **The acceptance oracle** (`value_type_checking.rs`) — the generator never consults `check`; `Spec::value` builds from `Spec::value_type` structurally. Genuinely independent.
- **"Exactly one" break** — checked each of the five `Break`s for a second simultaneous violation. `Break::TooMany` duplicates a well-typed value, `Break::Missing` removes a whole entry, `ForeignNodeRef`/`UndeclaredVariant` are only offered when available. No double break found.
- **`ValueKind` coverage** — `scalar_value()` reaches all seven scalars, `spec()` reaches `NodeRef`, `Enum`, `List`, `Record`; `every_value_kind_mirrors_its_value_type` pins eleven. No unreached kind.
- **Nesting** — `check_value` recurses through `List` and `Record`; a bad variant three levels down is refused (`value_type_checking.rs:616`). It does *not* name the path within the compound, only the property. I judged that acceptable against the story's "names the property and the reason" and did not raise it.
- **The cycle fix** — self-loop, two-cycle, long cycle and a cycle off the start path all refuse, because `check_declarations` walks every declared type and each cycle member reaches itself. A diamond still loads. The fix is correct; finding 2 is about the *other* `None`.
- **`NodeTypes`** — no domain concept crosses the boundary; `NodeId -> Option<TypeId>` is exactly what invariant 8 permits the ontology crate to know. An unnameable target is refused (`UnresolvedNodeRef`), not assumed allowed, and that is asserted.
- **`Vec<String>` constraints** — `preconditions_are_carried_as_opaque_text_and_refuse_nothing` is non-vacuous: a precondition is present and the transition still returns `Ok("decided")`.
- **The public-surface guard** — sampled `Cardinality::permits`, `Ontology::edge_type`, `EdgeType::new`, `Lifecycle::declares`, `OperationDefinition::new`, `SchemaVersion::seed`, `Transition::new`. Every one is named by a case that asserts something about its result. No naming-only item found.
- **Duplicate type *names*** — permitted by load. Invariant 3 says names are not identities, so I did not raise it.

## 6. Paths written outside the worktree

Both in the assigned scratch directory `$HOME/.cache/ekr-wave-p1-03/unit-scratch`, none elsewhere, nothing under `/tmp`:

- `adversary-gate.log` — the `task check` run quoted in part 3
- `adversary-suite.log` — the `--no-fail-fast` run the 125/129 counts come from

Build output went to the assigned `$HOME/.cache/b10x-target/epistemic-knowledge-runtime`. Lease `ekr-adversary-ontology-1` taken at start and released before this report.

```findings
- file: crates/ekr-ontology/src/schema.rs
  line: 283
  category: property
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "properties_of merges an inherited property by TypeId sort order rather than hierarchy distance, so a grandparent's declaration overrides the parent's and check_node gives opposite answers for two structurally identical ontologies"
- file: crates/ekr-ontology/src/schema.rs
  line: 320
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "ancestors() returns None for an undeclared ancestor as well as for a cycle, so a missing grandparent is refused as CyclicParents instead of UnknownType whenever the descendant's id sorts first"
- file: crates/ekr-ontology/src/schema.rs
  line: 120
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "check_properties never runs over OperationDefinition::arguments, so an operation argument typed NodeRef with empty allowed_types loads intact despite lib.rs promising such a declaration is refused at load"
- file: crates/ekr-ontology/src/value.rs
  line: 113
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "value.rs attributes its value_kind/parameters serde shape to systems/ekr/domains/ontology.yaml, which declares no name 'parameters' anywhere and gives PropertyDefinition the flat fields value_kind, ref_allowed_types and enum_variants instead"
```
