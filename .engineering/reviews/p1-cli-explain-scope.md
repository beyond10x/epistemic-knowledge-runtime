# CLI and explain story scope — 2026-09-22

Read-only preparation at coordinator `7cee20e1e01e9bb8d96dafd54aac9e1c4f6713d7`.
Read the reconciled `story:ekr-cli` revision 5 and `story:seed-and-explain` revision 6,
activated DESIGN §§88–92, kernel ESS, the shared durable brief/acceptance and
public interfaces pinned with `git show 7cee20e:<path>`. No moving durable-unit
implementation was inspected or assessed for completion. No source/ESS/AEP
writes, provider calls, builds or original-parser regression edits occurred.

**Dispatch follows an agreed durable public facade and resolution of three
contract seams: host identity/time transport, Snapshot valid-time selection,
and Explain's typed chain result.** The story acceptances remain appropriate;
none can close on argument parsing or seed-only roots.

Packaging deviation: installed `aep-plan:planning` 0.9.3 §8 names `story-scoper`,
but its package has no such agent file. This assigned read-only worker applied
§8's cited/inferred scope distinction directly, for these two explicitly coupled
stories; no substitute critic or additional worker was dispatched.

Paths/citations below are relative to the pinned coordinator tree. `DESIGN`
means `docs/epistemic-knowledge-runtime-design.md`.

## Scope

### story:ekr-cli

- **Cited:** `crates/ekr/src/main.rs` — clap composition, provider selection and
  host inputs; story lines 75–77 and pinned entrypoint lines 7–15.
- **Cited:** `crates/ekr/src/cli/{seed,propose,validate,commit,snapshot,explain}.rs`
  — exact six modules named by story lines 76–77. Their absence at the pinned
  base does not change that the planned paths are explicitly cited.
- **Cited:** `crates/ekr/src/exit.rs` — outcome/refusal/fault mapping; story 63–65,78.
- **Cited:** `crates/ekr/tests/retraction_example.rs` and
  `crates/ekr/tests/fixtures/` — fresh-process §65 and retained-result controls;
  story 39–42,79–80. Fixtures must use final graph/2, seed/2 and shared operation
  shapes, with declared ontology, real host authority and retained evidence.
- **Inferred, covered by existing directory scope:** `crates/ekr/src/cli/mod.rs`
  for dispatch; host-input/result presentation helpers can remain beneath
  `crates/ekr/src/cli/` if selected. No new top-level host module is yet justified.
- **Cited read dependency, not permission to edit:** `crates/ekr/Cargo.toml:14–24`
  already supplies clap, core/graph/kernel/ontology, serde_json, assert_cmd and
  tempfile. It deliberately has no ekr-store dependency. New dependencies must
  be measured and scoped before editing, not added speculatively.

### story:seed-and-explain

- **Cited:** `crates/ekr-kernel/src/explain.rs` — typed chain over retained records;
  story 63–64.
- **Cited:** `crates/ekr-kernel/tests/seed.rs` and
  `crates/ekr-kernel/tests/fixtures/seed-minimal.yaml` — seed chain and unknown
  assertion control; story 18–21,65–66. Preserve existing admission regressions.
- **Inferred, necessary addition:** `crates/ekr-kernel/src/lib.rs` — expose the
  new explain module/result through the kernel boundary. The existing public
  module/reexport list is at 93–115; no Explain export appears there.
- **Inferred, proposed narrow addition:** `crates/ekr-kernel/tests/explain.rs`
  — ordinary/lifecycle chain, missing/corrupt record, and fresh-process reopen
  tests, as story 67–68 expressly requires recording an additional test file.
  Coordinator may instead place those tests in an already scoped file; choose
  before dispatch. Do not infer a tests-wide grant.
- **Cited stale ownership entry:** machine scope still contains
  `crates/ekr-kernel/src/seed.rs` (story 16–17), while body 27–30,56–59,75 assigns
  seed admission and retained-result behavior to the durable unit. Remove this
  from the remaining explain write scope unless a concrete later accessor edit
  is measured; seed is a dependency, not a second writer.

## Required public seams, without invented signatures

At the pinned base, `Commit::over` and `over_with_bootstrap` require callers to
construct the store through a closure (`commit.rs:194–217`); `seed` returns Root
and `commit` consumes an in-process ValidatedTransaction (`:277,339`). Those are
baseline signatures, not final durable API promises. `head`, `snapshot`, `replay`
and `content` are existing read seams (`:225–249`). `TransactionDocument::read`,
`read_with_upload_limit`, `bytes`, `transaction` and `hash` are the actual shared
parser interface (`document.rs:162–174,224–235`).

The durable owner must hand off a kernel-owned way to open the selected SQLite
or file provider with verified retained authority, plus shared document handlers,
transaction-ID lookup/validation/commit, own-result retries, selected-revision
reads and verified retained-record/content access for explain. Exact public
symbols/result types remain **unestablished** by this report. The CLI must not
add ekr-store to get around the missing factory: AGENTS invariant 1 permits
only the kernel's production store dependency/authority implementation.

`GraphSnapshot::of` and `valid_at(Timestamp)` exist as public read seams
(`graph/src/snapshot.rs:26,75`), but the durable owner is changing lifecycle read
semantics under §88. Consume its final result; do not recreate lifecycle filtering
in CLI. Preserve the synchronous runtime-context boundary.

## Operation, result and exit requirements

| ESS wire command and inputs | Actual outcome the CLI must distinguish |
|---|---|
| `seed(seed_document)` | seeded or retained-seed returns original SeedResultV1/Root0, even after later head movement; differing logical input/context/anchor is AlreadySeeded, invalid admission is InvalidSeed. |
| `propose(transaction_document)` | proposed retains original YAML bytes and trusted submitter; malformed/structurally invalid input is StructurallyInvalid with no Proposed fact. Structurally valid semantic invalidity may persist for Validate. |
| `validate(transaction_id, against)` | validated **or recorded rejected**; TransactionNotFound first, then applicable state/basis rules; absent committed basis is RevisionNotFound, leaving Proposed without a decision record; wrong state is TransactionStateConflict. |
| `commit(transaction_id)` | committed **or recorded stale**; Committed retry returns original CommitReceiptV1 before staleness/time allocation; absent ID is TransactionNotFound; Proposed/Rejected/Stale are TransactionStateConflict. |
| `snapshot(at: optional RevisionNumber)` | taken at selected committed revision, default latest; absent revision is RevisionNotFound. Valid-time filter/result projection needs the choice below. |
| `explain(assertion_id)` | full verified chain or AssertionNotFound with requested ID; required-history corruption is not absence and not successful partial explanation. |

Citations: `systems/ekr/domains/kernel.yaml:833–1045`; DESIGN §92
(`:3992–4005`) fixes missing-transaction lookup priority. Story CLI 63–65 requires
exit 0 for successful declared outcomes, 2 for named ESS refusals on stderr,
1 for operational faults. Recorded rejection/staleness are declared outcomes,
not thrown errors; render their exact outcome/state so success does not falsely
claim acceptance/commit. Define usage/invalid-ID and verification-fault rendering
explicitly; do not relabel corruption as TransactionNotFound/AssertionNotFound.

Only Seed and Commit currently declare `response.result` in ESS. Propose and
Validate declare events; Snapshot/Explain only emit summary fields. Agree the
JSON presentation of actual handler outcomes before dispatch, including rejected
issues and stale provenance; a global CommitReceipt response must not cause
the stale path to fabricate a successful receipt. Preserve typed integer values
and original document bytes; no JSON transaction round trip or float conversion
(DESIGN:3761–3771,3932–3946).

## Model/interface decisions still required

1. **Trusted host transport and roles.** Decide the concrete host configuration
   source and authentication assertion boundary across fresh processes, separate
   from seed/proposal bytes. AuthorityStateV1, RegisteredAgent, BootstrapContext
   and the fixed profile already have ESS homes (`kernel.yaml:175–245`); do not
   invent a registry or new authority entity. Bootstrap operator/validator are
   registered and distinct; validator matches the exact fixed profile. Current
   actor maps to Proposer/Validator/Committer/Operator command roles (`:757–782`).
   A freely supplied ID is not authentication, and capability strings remain
   metadata, not a new P5 authorization language. Reopen checks the host anchor
   against retained authority; startup options cannot replace it (DESIGN:3637–3667).

2. **Trusted time must be supplied lazily enough.** Host context supplies the
   actor and time once per logical decision; proposer and every AddAssertion
   attribution match it. Enforce submitted_at <= validated_at <= committed_at,
   nondecreasing lineage times and assertion bounds; equal times are permitted
   (DESIGN:3501–3508,3767–3771,3879–3902). Retained Seed/Commit success lookup
   precedes clock sampling and new occurrence allocation. A CLI that samples
   `now` before every handler cannot meet that requirement merely by ignoring
   the new value. Agree a clock/context interface that permits the required
   ordering and a controlled trusted host fixture for fresh-process tests;
   do not offer proposal-supplied recorded times as authority. Replay never
   substitutes today's clock.

3. **Snapshot valid-time input and output.** CLI acceptance requires two valid
   times and an earlier revision, while ESS Snapshot currently accepts only
   optional `at` (`kernel.yaml:1008–1010`). Decide whether to add a typed valid-time
   command selector in the coordinator-owned model or explicitly define a CLI
   projection over the returned immutable snapshot using the shared graph read.
   Either path needs a declared result and no guessed wall-clock default.
   `Timestamp` currently parses canonical decimal milliseconds, not ISO dates
   (`core/src/time.rs:53–59,80–93`); choose/document CLI date syntax and conversions
   before fixture commands are fixed. The §65 replacement must use explicit
   supersession: retraction excludes Alice at **all** valid times at the newer
   revision, whereas supersession preserves her pre-boundary interval
   (DESIGN:3483–3514).

4. **Explain chain response/projection.** ESS presently supplies only assertion
   identity and integer link count in Explained (`kernel.yaml:1026–1045,1134–1139`).
   The story requires a typed full chain. Model the smallest response using
   references to already modeled assertion, seed/admission, proposal, validation,
   commit and evidence records; avoid new durable identities/store entities.
   Decide deterministic ordering/branching and lifecycle-change linkage before
   exposing it. Seed chains terminate in actual retained seed evidence/admission;
   ordinary chains include the exact proposal, basis/profile and commit; lifecycle
   changes add provenance without erasing original acceptance. HumanStatement
   terminates directly. Do not invent an ordinary seed proposal, observation,
   interpretation or integration plan to fill the general §62 sketch. Observation
   extension remains P2 (story seed-and-explain:42–54). Required bytes/addresses
   must verify; missing/corrupt history cannot yield a persuasive partial chain.

5. **I/O and result conventions.** Fix provider/path selection, host-document
   transport, stdout JSON/error form and stdin `-` behavior. For Propose, call the
   existing bounded reader directly over the stream; an unbounded read followed
   by `parse` violates DESIGN:3742–3746. No new grammar or frozen-profile changes.
   A stricter host ingress limit cannot change historical replay validity. Define
   Seed/2 loading through the final shared seed handler rather than copying the
   old seed/1 decoder. No migration, ingest, maintain or seventh transaction-list
   CLI verb is introduced.

## Machine-readable scope and scheduling handoff

Necessary next scope additions: **inferred** `crates/ekr-kernel/src/lib.rs` for
the explain export; **inferred** `crates/ekr-kernel/tests/explain.rs` if the proposed
separate ordinary/lifecycle suite is selected. Existing CLI directory scope
covers its module/host/presentation helpers. Existing paths explicitly named by
each revised story can have their confidence reconciled to **cited**. Remove or
explain the stale seed.rs write scope rather than relying on its broad grant.

Coordinator-owned model work is **cited** in
`systems/ekr/domains/kernel.yaml` for the missing result/valid-time choices;
add that file to the owning planning scope if that story is assigned the model
change. DESIGN amendment scope is conditional on the chosen semantic change;
`systems/ekr/components.yaml` needs no change if the same six commands/events
remain. No Cargo/lock, store, parser, P2, or repository-wide scope addition is
currently justified. A new public factory/record accessor should stay with the
durable owner already scoped over kernel source, not become a parallel CLI
worker's edit.

The active durable/format stories both cover kernel source/tests and
`crates/ekr/tests` (`commit-and-revision-lineage:33–67`,
`version-persisted-contracts:50–68,104–110`). Thus explain's lib.rs/seed tests/
fixtures and CLI integration tests overlap even if the new CLI modules do not.
Do not schedule these as disjoint based only on their titles. Freeze/hand off
the durable interface and fixture ownership, then implement explain, then bind
CLI to the same shared handlers. Alternatively a coordinator can explicitly
partition test paths/ownership after measuring the final durable handback.
Existing depends_on edges already encode explain → writer/seed/store and
CLI → explain/store; a duplicate direct writer edge is not needed for correctness.

Before dispatch: settle the five choices, record exact scoped additions and
shared model decisions, and hand over final public symbols/results and fixture
owners. No completion, test pass or model validation is asserted by this report.

