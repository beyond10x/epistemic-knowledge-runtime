---
format: aep.planning-md/3
id: review-result:adversary-ingest-01-e-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit E: the seed envelope decoded once'
relations:
- reviews: story:seed-envelope-decoded-once
revision: 1
---
unit: E, story:seed-envelope-decoded-once, commit 0b48bc05 on impl/envelope-once (worktree ekr-ingest-e) plus one untracked test file
verdict: NEEDS-CHANGE (the unit breaks invariant 1)
cases: executed 362→364, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (under /dev/shm/ekr-ingest-e)
needs-coordinator: none

## Cases added

`crates/ekr-kernel/tests/adversary_envelope_once_p1.rs` (untracked; no implementation file touched). Both red; both pass on the base tree (`0b48bc05^`, extracted and built separately: `test result: ok. 2 passed; 0 failed`), so both defects are introduced.

| case | asserts | red line |
|---|---|---|
| `a_seed_whose_envelope_bytes_do_not_decode_is_never_published` | if `seed()` returns Ok, a fresh open and a full replay must both admit the store | `seed=Ok(SeedResultV1 {…}) fresh-open=Err(InvalidSeed("seed-decode: 5 precedes 10: a range whose end precedes its start contains no instant …")) full-replay=Err(…same…) seeding-handle=Ok(RevisionNumber(0))` |
| `a_checkpoint_over_an_envelope_with_non_byte_payloads_answers_as_a_full_replay` | a checkpointed open's `snapshot()` equals the full replay's answer | `left: Ok(GraphRoot {…}) right: Err(InvalidSeed("seed-decode: invalid type: string \"these are not the payload's bytes\", expected a sequence …"))` |

Suite: `cargo test -p ekr-kernel --no-fail-fast` exited 101, passed 362, failed 2, only in `adversary_envelope_once_p1`.

## Findings

| # | file:line | verdict / origin | what breaks | what reaches it |
|---|---|---|---|---|
| F1 | `crates/ekr-kernel/src/commit.rs:363-365` | NEEDS-CHANGE / introduced, blocker | `seed()` caches its in-memory envelope before publishing, so the pre-publication replay never decodes the staged bytes (invariant 1, "replaying the staged candidate"). A seed whose bytes do not decode is published; every later process gets `seed-decode`, and the store is permanently unreadable. `seed_envelope_once.rs:238` asserted this behaviour. | the public Rust API (`Runtime::seed` with an in-memory document; an inverted range by struct literal). Not reachable from the `ekr` CLI, whose YAML parser refuses it. |
| F2 | `crates/ekr-kernel/src/checkpoint.rs:285-289`, `seed.rs:215` | CONFIRMED / introduced, warning | the light view accepts a forged checkpoint over an envelope whose payload values are not bytes; `snapshot()` returns the graph where a full replay and the old restore give `seed-decode`. Only the verified read still refuses. | a forged store only |

## What held

- Cross-store and cross-history serving: the cache is keyed by `seed_hash`, and `history.content()` re-hashes before every lookup; a tampered or missing blob still fails first with `required-object-integrity` or `required-object-missing`.
- Typed-first decode against Value-first: `seed-decode` messages, `SeedMigrationRequired` and `unsupported-seed-envelope` unchanged.
- Light view against full decode: same `deny_unknown_fields` sets and field types apart from payloads, so `checkpoint-graph-root-identity`, `checkpoint-seed-payloads` and `checkpoint-held-identities` are unchanged.
- Full replay still refuses the F2 store with `seed-decode` and still compares every retained payload's bytes.

```findings
- file: crates/ekr-kernel/src/commit.rs
  line: 363
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: seed() caches its in-memory envelope before publishing, so the pre-publication check never decodes the staged bytes and a seed whose bytes do not decode is published, leaving a store no later process can open (test a_seed_whose_envelope_bytes_do_not_decode_is_never_published)
- file: crates/ekr-kernel/src/checkpoint.rs
  line: 288
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the light view accepts a forged checkpoint over an envelope whose payload values are not bytes, so snapshot() returns the graph where a full replay refuses with seed-decode (test a_checkpoint_over_an_envelope_with_non_byte_payloads_answers_as_a_full_replay)
```
