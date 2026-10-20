---
format: aep.planning-md/3
id: story:supersede-node-review-fixes
kind: story
status: active
title: The 0.0.39 review's defects in SupersedeNode are fixed, its test gaps closed and its documents corrected
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersede-node
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/ekr-graph/src/canonical.rs
- confidence: cited
  path: crates/ekr-graph/src/supersession.rs
- confidence: inferred
  path: crates/ekr-integrate/src/extraction.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/supersession.rs
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_validate_once.rs
- confidence: cited
  path: crates/ekr-kernel/tests/commit_applies_once.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/supersede_node.rs
- confidence: cited
  path: crates/ekr-sdk/src/document/transaction.rs
- confidence: cited
  path: crates/ekr-sdk/src/extraction.rs
- confidence: cited
  path: crates/ekr-sdk/src/resolve.rs
- confidence: cited
  path: crates/ekr-store/tests/borrowed_graph.rs
- confidence: cited
  path: crates/ekr-views/src/index.rs
- confidence: cited
  path: crates/ekr-views/src/sample.rs
- confidence: cited
  path: crates/ekr-views/tests/supersession.rs
- confidence: cited
  path: crates/ekr/src/cli/extraction.rs
- confidence: cited
  path: crates/ekr/tests/store_comparison.rs
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: docs/sdk.md
- confidence: cited
  path: systems/ekr/domains/graph.yaml
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/views.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-20T01:30:35Z", actor: "agent:claude-ekr-session", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-20T01:30:36Z", actor: "agent:claude-ekr-session", revision: 6}
---
## Context

`review-result:haiku-panel-0-0-39-supersede-node` holds a read-only review of release 0.0.39
(`story:supersede-node`, ADR 0017) by 20 reviewers, one slice each: 54 findings, merged by the
coordinator into 10 defects (D), 7 test gaps (T), 7 document drifts (R) and 7 design questions (Q).
The coordinator read the cited code for D1, D2, D3 and D10 and confirmed each; the rest are the
reviewers' traces. The operator chose on 2026-10-20 to fix them as one wave and release 0.0.40.
This story fixes D1–D10, closes T1–T7, corrects R1–R7, and takes the coordinator's decisions on
Q1–Q7 below.

## Build

1. **D1** `crates/ekr-views/src/index.rs:228`: the `referencing` index skips an assertion whose subject
   is a superseded node or an edge retired with one, as `about_node`/`about_edge` already do.
2. **D2** `crates/ekr-sdk/src/resolve.rs:209`: `Resolver::observe` drops every cached answer naming a
   node a committed `SupersedeNode` retires (and any cached `by`-less answer for that node's aliases).
3. **D3** `crates/ekr-views/src/index.rs:556`: `roles()` dates nothing from an assertion whose subject is
   superseded or retired; overview timeline and roles agree.
4. **D4** decided as documentation (Q-style): a NodeRef value inside `Object::Value` keeps naming the
   superseded id; a reader that follows it gets `node-superseded` naming `by`. ADR 0017 gains a dated
   clarification; `views.yaml` and `kernel.yaml` already narrow the pointer to a `!Node` object.
5. **D5** decided as documentation: `ekr.node-detail/2` omits relations to and from a superseded node,
   as every read omits it; design § 110.5 says so instead of "both node-detail formats".
6. **D6** `crates/ekr-sdk/src/extraction.rs:396`, `:936`: a fact whose key resolves through a
   supersession to a node of another type is checked against the kept node's type before anything is
   written, so no node is committed for a fact that is then refused; `name_supersessions` names the
   supersession only on a refusal whose cause is that key's end (the key's type or a property or edge
   end the kept type does not declare).
7. **D7** `crates/ekr-kernel/src/validate/supersession.rs:218`, `:140`: the same-transaction retiring
   set holds only supersessions Reference admits; `SupersedeNode {X by X}` on an absent X is refused
   once for absence and once as `supersede-into-itself`, not twice for absence.
8. **D8** refusal messages: `reference.rs:425`, `:283` name the `by` the head will hold after this
   transaction (following a same-transaction supersession of `by`); `reference.rs:458` says to create
   the edge on `by` (edges are retired, not moved); `structural.rs:246`, `:294` name the superseded
   holder as superseded, name its `by`, and say the alias already resolves there.
9. **D9** `crates/ekr-graph/src/canonical.rs:484`: one retired-edge → record index per validation and
   per read instead of a scan over every record per lookup (`reference.rs:331`, `structural.rs:836`,
   `:920`, `sample.rs:216`); counted, not timed.
10. **D10** `crates/ekr-kernel/src/validate/structural.rs:912`: `CreateNode` with the id of an edge
    retired with a superseded node is `identity-already-exists` under every profile, as `kernel.yaml:741`
    says.
11. **T1–T7**: a test for each surviving mutant and gap the panel named: the standalone alias scan
    (`structural.rs:415`), `named()`'s CreateEdge target and AddAlias arms (`supersession.rs:92`, `:120`),
    a later CreateEdge into a superseded node (`reference.rs:133`), a supersession of a node created in
    the same transaction (`reference.rs:272`), a pinned knowledge root over a non-empty record
    (`ekr-graph/src/supersession.rs:80`), the cost counters with a supersession
    (`replay.rs:2247`, `:2396`, `commit_applies_once.rs:231`, `adversary_validate_once.rs:727`),
    borrowed vs owned graph bytes with records (`borrowed_graph.rs:134`), the conditional seventh compare
    category (`store_comparison.rs:283`), and the referencing leak (`ekr-views/tests/supersession.rs`).
12. **R1–R7** documents: § 110.2, `graph.yaml:511` and `ekr-graph/src/supersession.rs:47` say an edge
    between two nodes superseded together is held once, in its source's record (the code's rule, pinned
    by a test); ADR 0017 decision 4 gains a dated clarification that `by` may be created by the same
    transaction (the code's rule, pinned by a test); § 110.6 counts three generated `node-superseded`
    outcomes; `docs/cli.md:2621` says 404 for HTTP and a named refusal for MCP and the session;
    `graph.yaml:522` declares the record's evidence and retired edges; the doc comments at
    `structural.rs:200`, `:694` and `canonical.rs:505` are made true; `views.yaml` states that overview
    revision totals count every held assertion while projections and the timeline omit those about
    superseded or retired subjects.
13. **Q7** `crates/ekr-kernel/src/transaction.rs:248` and the SDK mirror: duplicate evidence ids in
    `!SupersedeNode` are refused as assertion evidence is (`unique_set`).
14. `docs/cli.md`, the design § 110 changes, `CHANGELOG.md` under Unreleased.

## Decisions on the design questions (coordinator, 2026-10-20)

- **Q1** no relatedness check between `node` and `by`: kept. ADR 0017 admits unrelated types on
  purpose; the reason, the evidence and the change row are the audit trail.
- **Q2** the change row's reason and retired edges: not in this story; a later read story if a
  consumer asks.
- **Q3** `ekr.transaction-document/1` accepting later operations: pre-existing, not this story.
- **Q4** facts typed by an old key's type are checked against the kept node's type (D6); a property the
  kept type does not declare stays refused.
- **Q5** subtype refusal through a kept node's own type: pre-existing rule of typed references; not
  changed.
- **Q6** `SupersededNode` deserialisation: unchanged; the knowledge-root compare refuses any record
  canonical state could not hold.
- **Q7** in this story (Build 13).

## Acceptance

One named test per Build item 1–3, 6–10, 13 (both providers where a store is involved), each run red
before its fix; the tests named in Build 11; `task check` exit 0 on the integrated head; the documents
in Build 12 match the code (`docs_cli` and `story_contract` green).

## Out of scope

Q1–Q3, Q5, Q6; merges (`story:merge-entity-applies`).

## Narrowed

On 2026-10-20 the coordinator took Build items 7 (D7), 8 (D8), 10 (D10) and 13 (Q7) out of this
story: each changes a kernel validation verdict, and replay compares retained rejections byte for
byte (`crates/ekr-kernel/src/replay.rs:1203-1213`) against stores that ran the released 0.0.39. They
move to `story:supersession-verdict-fixes-replay-safe`, with the implementation saved as a patch.
D9 stays: it changes how often records are read, not what any validation answers. The Acceptance
items for D7, D8, D10 and Q7 move with them. `crates/ekr/src/cli/extraction.rs` and `docs/sdk.md`
join the scope (D6's reader and D2's SDK note).
