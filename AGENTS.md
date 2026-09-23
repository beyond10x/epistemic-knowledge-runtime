# AGENTS.md — epistemic-knowledge-runtime

The contract for changing **this** repository. Read it before changing anything.

Organisation-wide rules — repository naming, the language rule (anything that runs is Rust, not
Python), bot authorship of every GitHub write, the public-source provenance rule and the rule that
a change to bytes another repository verifies is a coordinated migration — live in
`atlas/AGENTS.md` and the workspace `AGENTS.md` and are not restated here.

`README.md` orients a reader. This file says what must not break.

## Serves

The objectives of the collection this repository moves, by id from `atlas/ROADMAP.md`:

- **O2 — decisions as data, with evidence.** Every canonical assertion carries evidence, every
  commit is a validated transaction, every revision is immutable and replayable.
- **O5 — the generic agent platform.** This is the memory the platform's agents read from and
  propose to: observations through Connectors, work through AEP, agents as proposers behind a
  validation membrane.
- **O6 — self-improvement, built into all of it.** Failure to integrate is information; the runtime
  proposes its own schema from accumulated evidence and measures its own epistemic health.

A change here that moves none of these is a question for the operator, not a task.

## What this repository is

The runtime described in `docs/epistemic-knowledge-runtime-design.md`, built as a Cargo workspace
in the order `docs/roadmap.md` gives. One crate per domain; the crate map and what each owns is
roadmap § 3, and is not repeated here.

The runtime supersedes `company-brain` (v1) and `org-brain` (v2). Their code is a lift source, never
a dependency; their data enters through the import policy in `docs/predecessors.md` § 9.

## Normative documents

- `docs/epistemic-knowledge-runtime-design.md` — the architecture. Sections 1–80 are the original
  design; sections 81 onward are dated amendments. An amendment adds; it does not rewrite.
- `docs/roadmap.md` — phases, exit evidence, decisions D1–D3. A decision is reopened by an ADR in
  the planning store, not by an edit.
- `docs/predecessors.md` — the carry-over list and the import policy. A capability listed there is
  not dropped without an ADR saying so.
- `systems/ekr/` — the ESS domains, one per crate, added as the crates arrive. `ess specify validate`
  is part of `task check` once the first exists.
- `.engineering/planning/` — governed through `aep plan artifact`. Never hand-edit its records or
  journal. Before an agent writes, set `AEP_ACTOR` to the agent's execution identity.
- `.engineering/waves/COORDINATOR.md` — **read before opening a wave and again before its closing
  commit.** Eight checks, each with a command, each written because it was skipped and cost
  something named. The wave pages and review records carry the individual findings; counts must
  be measured from those records rather than inferred from the number of review passes.

## Invariants

Each is a claim that can be checked. Breaking one is a design change, not a refactor.

1. **Only `ekr-kernel` constructs a `ValidatedTransaction`, and only a `ValidatedTransaction`
   commits an ordinary transaction.** Initialization has no preceding revision: only the kernel's
   crate-private seed admission (`seed::admitted_graph`) builds a Seeded publication, and the store
   publishes one only after replaying the staged candidate through the injected kernel authority.
   Every replay revalidates the full seed input, retained evidence and actual attribution through
   that same kernel authority.
   `crates/ekr-kernel/tests/seed.rs` holds this on both providers in
   `an_evidence_seed_reopens_with_identical_roots_fields_and_retained_bytes`,
   `reopen_checks_full_ontology_and_execution_context` and
   `legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted`.
   A store without seed authority refuses, held by `crates/ekr-store/tests/fold_rules.rs`'s
   `a_store_with_no_commit_authority_refuses_to_say_what_canonical_state_is`.
   For ordinary transactions the first half is a type — the constructor is `pub(crate)` and the type derives no
   `Deserialize` — and two compile-fail cases hold it. The second half is **not** a type and this
   sentence no longer implies one: `ekr-store` sits below `ekr-kernel`, so its writer cannot take a
   `ValidatedTransaction`, and a sealed trait there would exclude `ekr-kernel` along with everybody
   else (`architecture-decision-record:0007-the-commit-path-is-the-kernels`). What holds is that no
   consumer of this runtime can reach a transaction writer to canonical state without a `ValidatedTransaction`:
   `ekr-store`'s fold moves canonical state only for a validation its injected `CommitAuthority`
   stands behind, and `ekr-kernel` is the only crate that declares `ekr-store` or implements that
   trait in a `src/`. Both of those are read off this tree by `crates/ekr/tests/story_contract.rs`,
   by a case and not by the compiler. Agents propose; they never mutate.
2. **Canonical knowledge depends only on canonical knowledge or retained admissible evidence.**
   A `Canonical → Transient` reference is unrepresentable at the type level, not merely refused.
3. **Names are not identities.** Every persistent object carries a stable id; a human-readable name
   is a property. An import mints ids and keeps the predecessor's id as an alias.
4. **Every canonical assertion has sufficient provenance according to policy.** "An agent said so"
   is not sufficient. Imported records carry `origin: inherited` and are never `Accepted` until a
   live source re-observes them.
5. **Committed revisions are immutable.** A later change is a new revision, a retraction, a
   supersession or a migration. Physical reclamation happens only through the maintenance path.
6. **Every outward write is an approved transaction.** The runtime reads the world; a message sent,
   a ticket changed or a page published requires an approval record the transaction cites.
7. **Deterministic validators run without a model.** Identity, references, types, cardinality,
   transaction integrity, revision lineage and retention rules are code; a model participates only
   where interpretation is necessary.
8. **No crate below `ekr-graph` names a domain concept.** The kernel and ontology crates know
   types, properties, edges and assertions; `Person`, `Project` and `Decision` are graph state.
9. **Sensitive material is redacted before it reaches a model.** Credential-shaped content, personal
   data outside the source's authorisation, and restricted-source evidence are withheld or
   minimised at the observation layer, not at the prompt.

## The gate

`task check` — format, clippy with warnings as errors, every test, rustdoc with broken links as
errors, the planning store's validation. Land nothing until it exits zero. Prefer `cargo check -p`
and `cargo test -p` on touched crates while working; run the whole gate before claiming a phase.

Set `CARGO_TARGET_DIR=$HOME/.cache/b10x-target/epistemic-knowledge-runtime`. One tree at a time
shares that directory with every other tree of this repository, which is what keeps a second
checkout from paying for a second full build.

**A test that reads this repository's own source must not locate it with
`env!("CARGO_MANIFEST_DIR")`.** That macro is a compile-time constant: a test binary compiled in one
checkout keeps reading that checkout's path, whatever tree later runs it. Use
`std::env::var("CARGO_MANIFEST_DIR")`, which cargo sets per process for `cargo test` and `cargo run`,
or walk up from `current_dir()` to the directory holding `Cargo.lock`.

Source readers now resolve the invoking checkout at runtime, including `xtask doctor`.
`crates/ekr/tests/temporal_reads.rs::executable_source_location_macros_cannot_return` checks
product source, tests and xtask for executable uses, including alternate macro delimiters.
The implementation and correction evidence is retained under `.engineering/reviews/p1-10-guards-*`.
Historical inventories describe their recorded base revisions. Measure executable uses separately
from comments when reporting a current count; a text search includes both.

Compile-fail snapshots are part of the membrane evidence. The exact compiler is pinned in
`rust-toolchain.toml` and the correctness workflow, but a new trait implementation can still
change rustc's incidental help text. Before refreshing a snapshot, inspect the old and new
diagnostics: the same forbidden operation must fail at the same source expression with the
same error code and primary message. A successful compilation, unrelated error, changed primary
diagnostic or missing case requires investigation, not a blanket `TRYBUILD=overwrite` run.
Refresh only the affected target after that comparison, review the `.stderr` diff, and rerun
the target with overwrite disabled. A deliberate compiler upgrade must likewise review the
primary diagnostics and preserve every forbidden-operation case. The full gate remains required.

**What was observed**, on 2026-09-21 at the close of wave p1-05: after the wave's worktrees were
removed, `task check` on the primary checkout failed reading
`…/ekr-wave-p1-05/crates/ekr-core/src` — a path no tree had. `cargo clean -p ekr-core` made that
tree's gate green at 337 cases, and the probe below shows why that is not a fix: it moves the stale
path to whichever tree did not just build. **The dangerous half is not that failure.** While both trees
exist the guard passes while reading a different checkout's source, and every one of these guards
holds a document against code, so one reading the wrong tree has stopped checking anything and says
nothing.

**The mechanism, measured on 2026-09-21.** Two byte-identical crates of the same name in two
directories, one shared build directory: `deps/` holds **one** test binary for both, and the second
directory's `cargo test` ran the first's binary in 0.01s and reported the first's compile-time path.
So two checkouts of this repository **do** write the same filenames, and that clobber is exactly what
serves one tree's binary to another. `cargo clean -p <crate>` does not fix it — it moves the stale
path to whichever tree did not just build.

**Two trees that build at the same time get one directory each.** Cargo's exclusive lock makes
concurrent compilation serialise rather than corrupt, and unit artifacts are keyed by a hash that
includes the package's manifest path. **That last clause was measured false on 2026-09-21 and is
struck**: two checkouts of this repository write the same `deps/` filenames and do clobber each
other, which is what the paragraph above is about. Two further outputs are not keyed by anything:
`doc/<crate>` is a single shared path, so `task doc-check` from two trees writes the same files, and the uplifted binary is one path, `debug/ekr`. Sharing is therefore not
merely slow for concurrent work, it is unsound for `doc-check`. A wave running more than one unit
gives each its own directory and says so in its page.

## What must not happen here

- A model or agent writing canonical state directly, under any flag.
- A `gh` write against this repository; every GitHub write is the bot's through `b10x-gates api`.
- A Python script that runs as part of the product or the gate.
- A hand edit under `.engineering/planning/`.
- A capability from `docs/predecessors.md` § 2 dropped without an ADR.
- Customer or personal data copied into fixtures, tests or documentation. Fixtures use the
  runtime's own vocabulary.
