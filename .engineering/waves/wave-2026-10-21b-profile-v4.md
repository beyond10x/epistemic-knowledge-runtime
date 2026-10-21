# Wave 2026-10-21b — profile v4 and the recorded profile move (ADR 0102)

Skill: aep:implementing 0.23.2 (wave mode). Coordinator: the Claude session of waves 2026-10-19a to
2026-10-21a.
Status: **open**; U1 after two adversary passes, its full gate running.

Source: Atlas ADR 0102, "A kernel verdict change is a new validation profile; a retained verdict
replays under the profile that decided it" (accepted 2026-10-21, atlas `d5d1dcd6`).

Approval: `b10x-chef-orchestrator` on 2026-10-21, option A of the coordinator's proposal: U1, then
U2; one implementor at a time; two adversary passes per unit; a full gate per unit; a pull request
per unit landed by a bot fast-forward of its green head; then release 0.0.42. The CHANGELOG's first
line under Changed is a `**Breaking:**` bullet (a store moved to v4 cannot be read by 0.0.41 or
older; the move happens only on an operator's recorded event), and one bullet says the ess pin
stays 0.36.0 (held on https://github.com/beyond10x/ess/issues/623).

Base: `main` at `c16413d9` plus the two plan commits of the held wave 2026-10-21a. Integration branch
`wave/20261021b`.

Width: 2 units, sequential (both touch `authority.rs` and `replay.rs`).

## Units

| unit | story | main surface | dispatch |
|---|---|---|---|
| U1 | `story:supersession-verdict-fixes-replay-safe` | profile v4 in `authority.rs`; D7, D8 in the supersession validators, Q7 at parse (kernel and SDK), each only under v4; spec, design, CLI help | `aep:implementor` |
| U2 | `story:validation-profile-move` | a profile-move log event, replay and the reopening anchor under the profile in force, both providers, a CLI verb, the 0.0.39 fixture | `aep:implementor`, after U1 lands |

## Trees

| unit | branch | worktree id | build dir | scratch | state |
|---|---|---|---|---|---|
| integration | `wave/20261021b` | `ekr-w20261020b` (reused) | none | none | store writes |
| U1 | `unit/20261021b-u1` | `ekr-w21b-u1` | `<tree>/target` | `<home>/.cache/ekr-w21b-scratch` | implementor `bb5b5b05`, correction 1 `d11b3acd`, corrections 2 and 3 `a0544302`; branch pushed; tree finished and archived at the pause |

## Reviews

| pass | record | findings | outcome |
|---|---|---|---|
| adversary 1 on `bb5b5b05` | `review-result:adversary-w20261021b-u1-pass-1` | F1-F3 red, F4 guard, F5 acceptance | F1-F3 fixed in `d11b3acd`; F4 kept; F5 no change (story decision 2); MergeEntity duplicate evidence is H13 of `story:merge-entity-review-follow-ups` |
| adversary 2 on `d11b3acd` | `review-result:adversary-w20261021b-u1-pass-2` | 1 warning, 4 notes, 4 red | 1-3 fixed in correction 2 (v4 messages follow the transaction's supersessions to the end of the chain, with a cycle message); 4 kept; 5 doc claim narrowed, its SDK decoding case kept `#[ignore]` as H14 |

Coordinator changes to the adversary files in correction 2: `cargo fmt` and one
`#[allow(clippy::type_complexity)]` (kernel pass-2 file), and the `#[ignore]` above.

## Next (state for a session that resumes here)

State at the pause (2026-10-21, session `ekr`, on `b10x-chef-orchestrator`'s pause of that day;
the operator's words: "pause ... on a good snapshot"): corrections 2 and 3 are commit `a0544302`
on `unit/20261021b-u1`, pushed. Correction 3 answered the first full gate's 4 failed meta-tests
(public surface of `profiled_set`, `with_strict_sets`, `read_under`; a direct `serde_yaml_ng`
reader in the SDK). It adds `DocumentError::Read(String)` to the SDK's exhaustive `DocumentError`;
no local checkout of the organization's repositories depends on `ekr-sdk` (searched 2026-10-21), the CHANGELOG names the variant, and
the orchestrator accepted it on 2026-10-21. The full gate on corrections 2 and 3 was stopped by the
pause: fmt, clippy, `task spec-check`, `task conform-fresh` and `task doc-check` exited 0, and the
workspace test step had reported 168 suites ok and none failed. That run is not a passing gate.
The three stories of this wave and wave 2026-10-21c are scoped (store commit `987aa90d`), with the
scoper's open points decided in each `## Scope`. No tree is held; the U1 tree is archived.

First step on resume: a new managed tree from `unit/20261021b-u1` at `a0544302`, then the full gate
(`sh <home>/.cache/ekr-w21b-scratch/u1-gate.sh <log>`, every step 0), then step 1 below from the
merge onward.

1. U1: when the rerun gate shows every step 0, merge `unit/20261021b-u1` into `wave/20261021b` in
   a tree of `wave/20261021b`; commit the store (review outcomes,
   this page, the story's verification evidence); push `wave/20261021b`; open a PR; on green CI,
   bot fast-forward of `main` to the head; finish `ekr-w21b-u1` with `--discard-cache --archive`.
2. Send `b10x-chef-orchestrator` "ready for respawn" at that idle point (its request of
   2026-10-21).
3. U2 `story:validation-profile-move`: scope, move to active, new tree from `main`, one
   implementor, two adversary passes, full gate, PR, fast-forward.
4. Release 0.0.42 (CHANGELOG: first Changed bullet `**Breaking:**` a store moved to v4 cannot be
   read by 0.0.41 or older and moves only on an operator's recorded event; one bullet: the ess pin
   stays 0.36.0, held on ess#623); tag sha to the orchestrator.
5. Then the review-record redaction with aep 0.75.0 and the CI aep pin 0.64.0 -> 0.75.0 in one PR
   (orchestrator's approval of 2026-10-21; reference https://github.com/beyond10x/cortex/pull/81).
6. Then triage `<home>/.cache/b10x-layers/review-2026-10-21/findings-ekr.md` (orchestrator, 2026-10-21):
   real findings as draft stories, "false" with a reason for the rest; finding 1 is already gone;
   skip the shared-gates.yml `pull_request_target` finding; reply in one line (real, false, ids).

## Pre-flight

| check | reading |
|---|---|
| builds | every cargo command through `b10x-build-slot` (3 machine-wide, none under 20G free); disk and accounts are `b10x-chef-orchestrator`'s |
| build settings | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, this repository's sccache on port 4302 |
| ess, aep | ess 0.36.0 and aep 0.64.0, the repository's pins |
| free disk at dispatch | 57G (`df -h /`, 2026-10-21) |
| held tree | `ekr-w21a-u1` keeps wave 2026-10-21a's uncommitted changes; not touched |
