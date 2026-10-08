---
format: aep.planning-md/3
id: review-result:adversary-wave-20261005b-c-pass-1
kind: review-result
status: active
title: Adversary, wave 20261005b unit C, pass 1
relations:
- reviews: story:seed-if-absent
revision: 1
---
unit: C, story:seed-if-absent, the uncommitted working tree on 55ce70cd in ekr-wx-c
verdict: CONFIRMED
cases: executed 57→61, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 9 paths (part 6)
needs-coordinator: none

The story's acceptance holds on all three providers. I could not break the races. One case is red: after a crash between election and publication, `--if-absent` says the store is already seeded when it has no seed, and no `--if-absent` call ever finishes that seed.

**1. Diff stat.** Only the test files changed relative to the implementor's tree: `durable_commands.rs` went from 176 to 247 added lines, `postgres_cli.rs` from 82 to 155, and the untracked `seed_if_absent.rs` gained two cases. No non-test path is touched by me.

**2. Cases added**

| test | asserts | now |
|---|---|---|
| `crates/ekr-kernel/tests/durable_commands.rs:1703` `adversary_an_if_absent_refusal_by_an_unpublished_election_leaves_the_lineage_unseeded` | an `--if-absent` seed is cut after election and before publication. On file and SQLite, three later `seed_if_absent` calls are each refused as `AlreadySeeded`, and the test then requires `head()` to be `Some` | **red** |
| `crates/ekr/tests/seed_if_absent.rs:231` `adversary_concurrent_if_absent_processes_on_a_fresh_store_…` | 6 `--if-absent` processes, 5 rounds, file and SQLite, on a path with no store yet: exactly one exits 0, every other exits 2 with `AlreadySeeded` | green |
| `crates/ekr/tests/seed_if_absent.rs:277` `adversary_a_plain_seed_racing_an_if_absent_seed_keeps_both_contracts` | plain `seed` against `--if-absent`, 8 rounds, both start orders: plain exits 0; `--if-absent` either returns the same result or is refused `AlreadySeeded` | green |
| `crates/ekr/tests/postgres_cli.rs:429` `adversary_if_absent_processes_racing_a_plain_seed_on_one_tenant` | 3 `--if-absent` and 1 plain process, real PostgreSQL (ekr-wx-c-postgres), 4 rounds on a fresh tenant: at most one `--if-absent` exits 0, the rest exit 2 | green |

Red output from the run of that case alone:
```
test adversary_an_if_absent_refusal_by_an_unpublished_election_leaves_the_lineage_unseeded ... FAILED
thread '…' panicked at crates/ekr-kernel/tests/durable_commands.rs:1734:13:
file=false attempt=0: refused as AlreadySeeded while the lineage has no seed; only a plain `seed` of the identical document can ever publish it
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 15 filtered out
```

**3. Suite run** (log: `~/.cache/ekr-wave-8c9138d1/c/runs/adv-suite.log`)

| command | result | exit |
|---|---|---|
| `cargo test --locked -p ekr-kernel --test durable_commands --test seed` | durable_commands: 15 passed, 1 failed (mine). Cargo stopped there, so `seed` ran next | 101 |
| `cargo test --locked -p ekr-kernel --test seed` | 34 passed | 0 |
| `cargo test --locked -p ekr --test seed_if_absent` | 5 passed | 0 |
| `cargo test --locked -p ekr --test postgres_cli -- --test-threads=1` (real PostgreSQL) | 6 passed | 0 |

The before count of 57 comes from the implementor's `green1.log`.

**4. Findings**

**F1 (warning, CONFIRMED, introduced): a crash between election and publication leaves the store stuck for `--if-absent` hosts.** `crates/ekr-kernel/src/commit.rs:580`.
- **What was measured:** the red case above. Every `--if-absent` call after the crash is refused as "a seed already there" while `head()` is `None`.
- **What gets it out:** only a plain `seed` of the byte-identical document with the same host context. A different document gets `PublicationInputConflict` forever. No other command recovers it.
- **What reaches it:** a process kill or a lost PostgreSQL connection between `prepare` and `resume`, on a host that seeds only with `--if-absent`. `docs/cli.md` tells exactly that kind of host ("a store it must own") to use the flag.
- **Fix to name:** in if-absent mode, finish the pending election (`finish_seed(pending, …)`) and then refuse. The refusal is then true, and any caller unsticks the store.

**F2 (note, CONFIRMED, introduced, no case written): a caller that actually won is told it lost.** `docs/cli.md:202`.
- **What happens:** if the answer is lost after the seed was published, the same caller's retry gets `AlreadySeeded`.
- **Why there is no case:** the API has no caller-supplied attempt identity, so no implementation could pass such a test. The behaviour is documented.
- **Effect:** for the cortex use case, the creator of a tenant can conclude another host owns it, which leaves the tenant seeded but unowned. This fails on the safe side; the fix would need a caller-supplied attempt key.

**5. Attacked and could not break**
- **Races on a fresh store:** file and SQLite, 6 processes, 5 rounds, no store pre-provisioned. One exit 0 each round, the rest exit 2.
- **Losers leave nothing behind:** file provider, 3 races of 6 processes. Each race store has 9 files, the same as a single seed (scratch probe script).
- **Plain against `--if-absent`:** on file, SQLite and PostgreSQL, the plain seed always exits 0, and an `--if-absent` exit 0 always matches the plain seed's result.
- **Plain `seed` is unchanged:** `seed_with(false)` is the old body plus one unused local variable.
- **Conformance:** only digests moved. All three suites regenerate byte-identical with `ess conform synthesize`, and the baseline digest equals sha256 of `suite.json`.
- **`ekr session --create` with the flag:** the session parses requests with the same clap `Command`. The implementor's test covers this, and a refused seed leaves the session without a store.

**6. Paths written outside the worktree**
- `~/.cache/ekr-wave-8c9138d1/c/runs/adv-kernel-alone.log`
- `~/.cache/ekr-wave-8c9138d1/c/runs/adv-cli-race-alone.log`
- `~/.cache/ekr-wave-8c9138d1/c/runs/adv-cli-plain-race-alone.log`
- `~/.cache/ekr-wave-8c9138d1/c/runs/adv-pg-alone.log`
- `~/.cache/ekr-wave-8c9138d1/c/runs/adv-suite.log`
- `~/.cache/ekr-wave-8c9138d1/c/tmp/adv-conformance/`
- `~/.cache/ekr-wave-8c9138d1/c/tmp/adv-probe/`
- `~/.cache/ekr-wave-8c9138d1/c/tmp/adv-probe-file/`
- `~/.cache/ekr-wave-8c9138d1/c/tmp/adv-probe.sh`
- The build reused `~/.cache/b10x-target/ekr-wx-c`. PostgreSQL test tenants were created in ekr-wx-c-postgres.
- Disk is at 4.5 GB free, so I stopped building.

`git status --short` (my additions are inside these test files; the status lines themselves match the implementor's):
```
 M crates/ekr-kernel/tests/durable_commands.rs
 M crates/ekr/tests/postgres_cli.rs
?? crates/ekr/tests/seed_if_absent.rs
```

**7. Findings block**
```findings
[
  {"file": "crates/ekr-kernel/src/commit.rs", "line": 580, "category": "concurrency", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "After an if-absent caller crashes between election and publication, every later if-absent call is refused as AlreadySeeded while the lineage has no seed, and only a plain seed of the byte-identical document can ever publish it."},
  {"file": "docs/cli.md", "line": 202, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A caller whose if-absent seed was published but whose answer was lost is told AlreadySeeded on retry, so the tenant's creator can conclude another host owns it; there is no caller-supplied attempt identity to tell the cases apart."}
]
```