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

## Invariants

Each is a claim that can be checked. Breaking one is a design change, not a refactor.

1. **Only `ekr-kernel` constructs a `ValidatedTransaction`, and only a `ValidatedTransaction`
   commits.** No other crate holds a writer to canonical state. Agents propose; they never mutate.
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

Set `CARGO_TARGET_DIR=$HOME/.cache/b10x-target/epistemic-knowledge-runtime` so every worktree of
this repository shares one build directory.

## What must not happen here

- A model or agent writing canonical state directly, under any flag.
- A `gh` write against this repository; every GitHub write is the bot's through `b10x-gates api`.
- A Python script that runs as part of the product or the gate.
- A hand edit under `.engineering/planning/`.
- A capability from `docs/predecessors.md` § 2 dropped without an ADR.
- Customer or personal data copied into fixtures, tests or documentation. Fixtures use the
  runtime's own vocabulary.
