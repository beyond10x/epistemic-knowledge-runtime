---
format: aep.planning-md/3
id: review-result:adversary-w20261019a-u1-pass-2
kind: review-result
status: active
title: Adversary pass 2, wave 2026-10-19a U1 (SupersedeNode)
relations:
- reviews: story:supersede-node
revision: 1
---
Adversary pass 2 (`aep:adversary`) on wave 2026-10-19a unit U1, `story:supersede-node`, tree
`ekr-w19a-u1` at `35fe08743`, recorded by the coordinator as it returned below.

Owners: 4 findings, 2 implementor (F6, N2), 2 coordinator (F5: correction brief 1 named the
pointer for subjects only, against ADR 0017 decision 3; N1: the brief set no message rule for a
cross-type landing).

---

unit: story:supersede-node, U1, tree ekr-w19a-u1 at head 35fe08743 plus two untracked adversary test files
verdict: NEEDS-CHANGE
cases: executed 1528→1531, red 2
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 24 paths, all in the assigned scratch directory <home>/.cache/ekr-w19a-u1-adversary/pass-2
needs-coordinator: F5 is ADR 0017 decision 3 against the unit's own views.yaml:2721 contract, which names a pointer only for a sampled subject. Decide whether objects and edge subjects need one too (same question as pass-1 F4). F6 (compare) has an obvious fix. I took no worktree lease.

## 1. Diff (`git --no-pager diff --stat` is empty: both files are untracked, test files only)
```
?? crates/ekr-kernel/tests/adversary2_w20261019a_value_positions.rs
?? crates/ekr/tests/adversary2_w20261019a_supersede_cli.rs
```
I changed no implementation file. Both files are rustfmt-clean, and `cargo clippy --locked -p <crate> --test <file> -- -D warnings` passes on each.

## 2. Cases added (each was run alone before the suite)
| case | asserts | now |
|---|---|---|
| `adv2_a_node_ref_in_every_value_position_and_depth_names_the_node_it_references` (ekr-kernel) | 6 positions, both providers: UpdateProperty List element, UpdateProperty Record field, CreateNode Record field, CreateEdge property, Invoke argument, AddAssertion value-object List element. Control: each validates alone. Beside a SupersedeNode of the node it names: `superseded-node-named-in-transaction`. In a later transaction: `node-superseded` | green |
| `adv2_a_read_reaching_an_assertion_whose_object_is_superseded_names_the_pointer` (ekr) | after Acme is superseded by Globex (propose/validate/commit of YAML), three reads of `Alice CEO_OF Acme` (…0510) name Globex: MCP `describe_node` full, `ekr sample`, `ekr explain` | RED |
| `adv2_two_stores_whose_duplicate_is_superseded_by_different_nodes_do_not_compare_equal` (ekr) | two stores supersede `Dup` (alias `dup`), one by Alice and one by Bob. Precondition: `ekr resolve` of `dup` answers Alice in one and Bob in the other. Then `ekr compare` must answer `equal: false` | RED |

The kernel case is green. It pins fix F1 in the positions pass 1 never reached: pass 1's cases would stay green if the Invoke, CreateEdge or Record arms of `named()` were dropped. This is inferred and not mutation-run.

Red output when each case was first run alone (verbatim, abridged to the assertion):
```
file: reads that reach Alice CEO_OF Acme name Acme without the node it is superseded by (00000000-0000-4000-8000-00000000b001):
describe_node (ekr.node-detail/1): {...,"id":"00000000-0000-4000-8000-000000000510",...,"object_kind":"Node","object_ref":"00000000-0000-4000-8000-000000000303",...}
ekr sample: {...,"object_name":"Acme","predicate_name":"CEO_OF","subject":"00000000-0000-4000-8000-000000000301","subject_kind":"Node","subject_name":"Alice",...}
ekr explain: its assertion link names object {"Node":"00000000-0000-4000-8000-000000000303"}
---
assertion `left == right` failed: file: `dup` resolves to Alice in one store and to Bob in the other, and ekr compare answers that they hold the same knowledge: {"categories":[... every category "equal":true ...],"equal":true,"format":"ekr.store-comparison/1","left":{"revision":2},"right":{"revision":2}}
```
Probes on sqlite (scratch probe1 to probe4) gave the same answers.

## 3. Suite run, after the cases existed (script `<scratch>/suite.sh`)
- Command 1: `cargo test --locked -p ekr-graph -p ekr-kernel -p ekr-store -p ekr-integrate -p ekr-views -p ekr-sdk --no-fail-fast`. Result: 1428 passed, 0 failed, 11 ignored, CRATES_EXIT=0.
- Command 2: `cargo test --locked -p ekr --test conformance --test conformance_integrate --test docs_cli --test agent_cli --test story_contract --test public_surface --test schema_cli --test adversary_w20261019a_supersede_extraction --test adversary2_w20261019a_supersede_cli --no-fail-fast`. Result: 101 passed, 2 failed, EKR_EXIT=101.
- The only failures are my two red cases.
- `before` = 1531 executed minus my 3 cases = 1528. That matches pass 1's after-count. I did not run a separate deselected suite.

## 4. Findings (cover 35fe08743 plus my files)
| # | file:line | severity | verdict | origin | finding | what reaches it |
|---|---|---|---|---|---|---|
| F5 | crates/ekr-views/src/sample.rs:346 | warning | CONFIRMED | introduced | Fix F4 only works for a superseded subject. When the superseded node is an assertion's object, `ekr sample`, `describe_node` (`ekr.node-detail/1` `object_ref`) and `ekr explain` name the retired id without its pointer. ADR 0017 decision 3 says such a read "names the retired id with its pointer". The unit's own views.yaml:2721 promises the pointer for subjects only. By the code (not run), an assertion about a retired edge also gets none: sample.rs:348 returns `None` for `Subject::Edge`. | Every supersession of a node that other nodes' relation assertions point at, e.g. Acme in the example seed. Red case above, file provider (sqlite matches by probe). |
| F6 | crates/ekr/src/cli/compare.rs:242 | warning | CONFIRMED | introduced | `ekr compare` compares six categories and never reads `graph.superseded`. Two stores that differ only in which node a duplicate is superseded by answer `equal: true`, yet `ekr resolve` of the duplicate's alias answers differently in each. docs/cli.md:672 says compare answers "whether two stores hold the same knowledge". | `ekr compare`, the documented check after a rebuild or a provider move. Fix: add a `superseded` category, keyed by node id, comparing node, edges, by, revision, reason and evidence; cli.yaml `ComparisonCategory` and docs/cli.md:710 list it. |
| N1 | crates/ekr-sdk/src/extraction.rs:820 | note | CONFIRMED | introduced | Across types, an extraction under a superseded key resolves to a kept node of another type, as ADR decision 5 intends. A property fact of the old type is then refused `undeclared-property: property … is not declared by the type`, and nothing in the message mentions the supersession. | The ADR's own consumer case (MergeRequest key, WorkItem kept). Probe5: an `Organization [bob-ltd]` superseded by Person Bob; `legal_name` fact rejected. Same-type relation and new-value facts under a superseded key land on the kept node correctly. |
| N2 | crates/ekr-views/src/timeline.rs:518 | note | CONFIRMED | introduced | Timeline `subject` set to a superseded id answers zero rows, byte-identical to an unknown id, while `describe_node` of the same id answers `NodeSuperseded` naming `by`. ADR decision 6 lists only describe_node and expand, so this is a gap, not a breach. | MCP `timeline` and session `timeline --subject` (probe9). |

## 5. Attacked, could not break
- F1: NodeRef values at every position and depth, both in the same transaction and later. Green case above. The value space has no edge reference, so a value naming a retired edge cannot be built.
- F2: every later operation on Acme or retired edge 0700 (DeleteEdge, CreateEdge, Invoke, AddAlias, MergeEntity) gets exactly one `node-superseded`. RetractAssertion of 0510 still validates.
- F3: a relation fact from a superseded key, and a new value under one, land on the kept node (Bob→Acme edge and assertions). No new node is created.
- F4 reads: MCP describe_node of the kept node, describe_node/2 of a third node (it omits the relation), changes_since (`NodeSuperseded` with type, name and target), overview, search and expand all answer correctly.
- CLI end to end on both providers: propose/validate/commit of a SupersedeNode from YAML, `ekr resolve`, MCP `resolve`, session `describe` (`ekr.views.NodeSuperseded` naming by).
- A read-only store (chmod a-w) holding a supersession: head, snapshot, `--full-replay` snapshot and resolve all answer, on file and sqlite.
- Stage begin, commit in the stage, then publish; a second stage copied from a store that holds a record; `ekr migrate` to file and sqlite keeps the record and the knowledge root.
- docs/cli.md step 9: "revision 3, before the supersession" is correct (commits are wrote=1, publish=2, duplicate=3, supersede=4).
- `docs/sdk.md` and CHANGELOG claims match the code. The SDK `SnapshotSupersededNode` carries all six fields `ekr snapshot` emits.

## 6. Paths written outside the worktree (all under <home>/.cache/ekr-w19a-u1-adversary/pass-2/)
alone-cli.log, alone-kernel.log, att.txt, sup.txt, host.json, seed.yaml, ref-file.yaml, ref-sqlite.yaml, probe1.sh to probe9.sh, suite.sh, suite-crates.log, suite-ekr.log, and the probe store directories w-file/, w-sqlite/, c-sqlite/, x-file/, s-sqlite/ (holds m-file/ and m-sqlite/). Permissions changed in probe4 were restored. The tests' tempfile directories under $TMPDIR are removed on drop.

## 7. Findings block
```findings
- file: crates/ekr-views/src/sample.rs
  line: 346
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a superseded node named as an assertion's object (and by code a retired edge as subject) is rendered by ekr sample, describe_node/1 and ekr explain without its pointer, against ADR 0017 decision 3
- file: crates/ekr/src/cli/compare.rs
  line: 242
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: ekr compare ignores supersession records, so two stores whose duplicate resolves to different kept nodes compare equal
- file: crates/ekr-sdk/src/extraction.rs
  line: 820
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an extraction under a key superseded across types lands on the kept node and is refused undeclared-property with no mention of the supersession
- file: crates/ekr-views/src/timeline.rs
  line: 518
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a timeline subject that is a superseded id answers zero rows like an unknown id instead of naming the node it is superseded by
```
