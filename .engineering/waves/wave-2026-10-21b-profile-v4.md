# Wave 2026-10-21b — profile v4 and the recorded profile move (ADR 0102)

Skill: aep:implementing 0.23.2 (wave mode). Coordinator: the Claude session of waves 2026-10-19a to
2026-10-21a.
Status: **open**; U1 dispatched.

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
| U1 | `unit/20261021b-u1` | `ekr-w21b-u1` | `<tree>/target` | `<home>/.cache/ekr-w21b-scratch` | dispatched |

## Pre-flight

| check | reading |
|---|---|
| builds | every cargo command through `b10x-build-slot` (3 machine-wide, none under 20G free); disk and accounts are `b10x-chef-orchestrator`'s |
| build settings | `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, this repository's sccache on port 4302 |
| ess, aep | ess 0.36.0 and aep 0.64.0, the repository's pins |
| free disk at dispatch | 57G (`df -h /`, 2026-10-21) |
| held tree | `ekr-w21a-u1` keeps wave 2026-10-21a's uncommitted changes; not touched |
