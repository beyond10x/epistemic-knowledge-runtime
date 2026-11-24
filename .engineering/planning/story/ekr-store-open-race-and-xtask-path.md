---
format: aep.planning-md/3
id: story:ekr-store-open-race-and-xtask-path
kind: story
status: active
title: Store-open race answers documented states; xtask builds with a symlinked CARGO_HOME
summary: ekr head racing the first seed answers only documented states, and cargo xtask search-web builds when CARGO_HOME is a symlink
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T09:14:47Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T09:14:47Z", actor: "human:timo", revision: 3}
---
## Outcome

`ekr head` racing the first seed of a SQLite store answers only documented states on every round,
and `cargo xtask search-web` builds on a host whose `CARGO_HOME` is a symlink.

## Work

Two defects found on 2026-11-24 while fixing the EKR 0.0.42 review findings.

1. Flaky race. `adversary2_p5_01_store_open::sqlite_head_racing_the_first_seed_answers_only_documented_states`
   failed once on CI (PR #124, head 37a3ab2, run on the "Repository correctness" job): in round 19
   `ekr head` answered `store-read-only` because it could not write the store's `-wal` file while
   the first seed was creating the store. It passed on the rerun. Find out whether
   `store-read-only` is a documented state for that moment (docs/cli.md) or a wrong answer from the
   store-open code; make the code answer a documented state (or the documented list name this one,
   if the contract allows it), and make the test deterministic enough to catch a regression.

2. `xtask search-web` canonicalises `CARGO_HOME`. On the build server `~/.cargo` is a symlink to
   another directory, so the registry paths are not remapped and the build stops with "personal
   build path". Workaround used: `CARGO_HOME=$(readlink -f "$HOME/.cargo") cargo xtask search-web`.
   Remap both the given and the canonical path, so the check holds with or without a symlink.

## Acceptance

For 1: a test that reproduces the race deterministically (or with a bound on rounds that failed on
main) and passes after the change, on SQLite; no assertion weakened. For 2: a unit test of the
remap with a symlinked `CARGO_HOME`, red before and green after.
