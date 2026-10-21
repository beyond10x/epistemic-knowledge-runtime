---
format: aep.planning-md/3
id: story:validation-profile-move
kind: story
status: draft
title: An operator's recorded event moves a store forward to a later validation profile; replay uses the profile in force (ADR 0102 d6, d7)
relations:
- decomposes: epic:p3-incubation-integration
- serves: vision:o2
- depends_on: story:supersession-verdict-fixes-replay-safe
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-graph/src/events.rs
- confidence: cited
  path: crates/ekr-kernel/src/authority.rs
- confidence: cited
  path: crates/ekr-kernel/src/checkpoint.rs
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/document.rs
- confidence: inferred
  path: crates/ekr-kernel/src/migrate.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/fixtures
- confidence: inferred
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/src/preparation.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
revision: 4
---
## Context

Atlas ADR 0102 (accepted 2026-10-21, atlas `d5d1dcd6`) decision 6 = B: "an operator-authorised log
event moves a store from its profile to a later one at a revision; replay uses the profile in force
at each proposal's position; a checkpoint already hashes the profile (`replay.rs:642`) and would hash
the one in force". Today a store keeps the profile its seed named and "no mechanism moves a store
from one to the other" (`crates/ekr-kernel/src/authority.rs:39-41`; design
`docs/epistemic-knowledge-runtime-design.md:4231-4237`, "a later milestone").

Decision 7: "A store written by the 0.0.39 binary, holding a D7, a D8 and a Q7 verdict, is checked in
and reopened with and without `--full-replay` on both providers, as `base_era_v1_replay.rs` did for
v2."

Depends on `story:supersession-verdict-fixes-replay-safe` (profile v4).

## Build

1. A new log event kind: a profile move, forward only (v1 or v2 to v3 or v4, v3 to v4), recorded at
   a revision on an operator's explicit command (a CLI verb), refused otherwise.
2. Replay validates each retained proposal under the profile in force at its position: the seed
   profile, then each recorded move. The reopening compare of the anchor (`authority.rs:39-41`)
   becomes "seed profile plus recorded moves". A checkpoint hashes the profile in force.
3. Both providers (the file and SQLite stores, and PostgreSQL where the other events are).
4. The d7 fixture: a store written by the 0.0.39 binary (built once from the tag), holding a D7, a D8
   and a Q7 verdict, checked in; reopened with and without `--full-replay` on both providers; then
   moved to v4, after which a new proposal of each kind gets the v4 verdict.

## Acceptance

- The fixture store reopens and fully replays unedited before and after the move; proposals before
  the move's revision replay under their profile, after it under v4.
- A move backwards, to the same profile, or without the operator's command is refused.
- A store with no move keeps its knowledge root and checkpoint bytes.
- 0.0.41 refuses a store holding a move (stated in the CHANGELOG as Breaking).

## Out of scope

Any new verdict; automatic moves.

## Scope

Derived 2026-10-21 by `story-scoper` against the tree of wave 2026-10-21b U1 (profile v4, uncommitted corrections), read-only; nothing was run. Each line is **cited** (read in the tree) or **inferred**. Line numbers in `document.rs`, `kernel.yaml`, `CHANGELOG.md` and the two documents may move when U1 commits.

- **Primary surface:** `crates/ekr-kernel`, where the profile in force is looked up, compared and bound. cited
- **Files:**
  - `crates/ekr-kernel/src/replay.rs:603-615` `parsed_proposal` (parses under `anchor.validation_profile`), `:635-651` `basis` (`validation_profile_hash` `:651`), `:668-690` `validate` (pipeline from the anchor's profile), `:992` `replay_from`, `:1039` `keeps_identities`, `:1090` the version cursor. cited
  - `crates/ekr-kernel/src/authority.rs:22-34` profile constants, `:44` "no mechanism moves a store from one to the other", `:173` `is_supported_for`. cited
  - `crates/ekr-kernel/src/checkpoint.rs:136-150` `checkpoint_authority`, `:152-179` `checkpoint_binding`. cited
  - `crates/ekr-kernel/src/commit.rs:395-402` `seed_root`/`agent_root`, `:496-510` retained seed compare; `crates/ekr-kernel/src/seed.rs:699-700` `AuthorityMismatch`; `crates/ekr-kernel/src/apply.rs:250` `agent_root` carried forward. cited
  - `crates/ekr-kernel/src/document.rs:290-299` `parse_under`. cited
- **Also likely:**
  - `crates/ekr-graph/src/events.rs:40-94`, `:141`, `:154`, `:173` (`RevisionPayload`, `variant_index`, `name`, encode): the new event kind. inferred
  - `crates/ekr-store/src/eventlog.rs:79-83`, `:253`, `:290`, `:2010`, `:2173` (provider aliases, `HeldRevisions::accept`, `CheckpointWritten`, the single fold for every provider). inferred
  - `crates/ekr-store/src/preparation.rs:31-56`, `:866-886` (publication command kind per payload). inferred
  - `crates/ekr-kernel/src/migrate.rs:500`, `:572`, `:778`, `:798`, `:828`, `:850` (exhaustive payload matches). inferred
  - `crates/ekr/src/cli/mod.rs:148` `Command`; `docs/cli.md` and `crates/ekr/tests/docs_cli.rs` (AGENTS.md requires both with a new verb). cited
  - `systems/ekr/domains/kernel.yaml:987-988`, `:1843-1900` (payload mapping, events block). inferred
  - `crates/ekr/src/cli/agent.rs:145-149`, `crates/ekr/src/host.rs:108-117`, `docs/epistemic-knowledge-runtime-design.md:4231-4237`: text saying no mechanism moves a store. cited
  - `CHANGELOG.md`; a new store fixture under `crates/ekr-kernel/tests/fixtures/`. inferred
- **Existing fixtures:** `base-cee0cae-v1-ownerless-modify-property.sqlite` (v1) and `base-30703729-v3-no-attachments.sqlite` (v3). Neither can hold a D7, D8 or Q7 verdict: SupersedeNode arrived at `7f0deb561`, after `30703729`. Tag 0.0.39 (`67bd55554`) has no other store. cited
- **Safety fact:** every profile read in replay goes through the seed anchor (`replay.rs:615`, `:1039`, `:684-690`), so a move lookup that falls back to the anchor changes no byte of a store with no move. Read, not run.
- **Story citations corrected:** `replay.rs:642` is `graph_root_id`; the profile hash in a basis is `replay.rs:651`; the checkpoint hashes the profile at `checkpoint.rs:136-150`; `authority.rs:39-41` is a doc comment, the statement is `:44`. cited
- **Confidence:** medium.
- **Would collide with:** wave 2026-10-21b U1 (`authority.rs`, `document.rs`, `validate/`, `kernel.yaml`, `CHANGELOG.md`, design); sequential. Story `preparation-blobs-are-reclaimed` if it adds an event kind in `eventlog.rs`. inferred

**Decided by the coordinator on 2026-10-21**, inside ADR 0102 decision 6 (Atlas `architecture/adr/0102-a-kernel-verdict-change-is-a-new-validation-profile.md:75-87`):

1. Root and checkpoint basis: `agent_root` and `checkpoint_authority` stay on the seed anchor; moves are bound by the log and the checkpoint's prefix digest (`replay.rs:302`, `:324-333`). A store with no move keeps every byte.
2. A move takes effect at a revision number, the next revision after the head it was recorded at; replay advances on the version (`replay.rs:1090`).
3. A proposal is parsed and validated under the profile in force at the revision it was validated for.
4. Allowed transitions are the ADR's list and no other: v1 to v3, v1 to v4, v2 to v3, v2 to v4, v3 to v4. Backwards, same-profile and v1 to v2 are refused with a named refusal.
5. No `ekr.revision-event/2` bump: a reader that does not know the move's variant refuses it (`events.rs:141`). The 0.0.41 refusal is evidenced by opening a moved store with the 0.0.41 binary once and recording the output as verification evidence; the CHANGELOG names it under Breaking.
6. `ekr migrate` carries each recorded move into the destination.
7. The move is recorded under the seed anchor's operator, which the host already holds (`authority.rs:213`) and the checkpoint already hashes; the verb takes no other identity.
8. A new verb, not an `ekr migrate` flag: migrate writes a destination store (`migrate.rs:778`).
9. A new publication command kind for a move (`preparation.rs:866-886`).
