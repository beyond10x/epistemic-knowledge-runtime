---
format: aep.planning-md/1
id: review-result:p1-13-cli-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-13 CLI unit
relations:
- reviews: story:ekr-cli
revision: 1
---
Adversary pass 1 on unit p1-13-cli (wave p1-13), `aep-drive:adversary`, through fresh binary
processes on both providers.

Owners: 5 findings, 4 implementor (introduced), 1 undecided (store opener, routed by the
coordinator).

Verdict: CONFIRMED (1 warning), INFEASIBLE (1 warning), 3 notes. cases: executed 97→106, red 2.

Cases in `crates/ekr/tests/adversary_p1_13_cli_exit_contract.rs`: a different host anchor
(green); assertion attribution and unregistered proposers (green); a 64 MiB stdin proposal refuses
without draining (green); `--valid-at -1` in the separated form (red); explain JSON equals the
kernel carrier (green); a whitespace-only seed respelling is an exact retry (green); a non-UTF-8
seed (green); concurrent exact seed and commit (red, SQLite only, "database is locked" at open);
concurrent distinct proposals (green).

Coordinator routing: the valid-at finding went back to the implementor; the SQLite open contention
is filed as `task:sqlite-provider-open-contention` and the case now pins today's behaviour; the
usage-error exit 2, the unseeded-commit exit 1 and the unbounded seed read are contract-conformant
and left as they are.

```findings
- file: crates/ekr/src/cli/mod.rs
  line: 84
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: clap rejects the canonical negative millisecond form in `--valid-at -1` as an unknown argument, so the declared decimal selector only accepts pre-1970 instants in the `=` spelling
- file: crates/ekr-store/src/eventlog.rs
  line: 79
  category: concurrency
  severity: warning
  verdict: INFEASIBLE
  origin: undecided
  message: concurrent exact `ekr seed` invocations on one sqlite store fail with exit 1 "database is locked" when the provider opens, instead of returning the one retained result
- file: crates/ekr/src/exit.rs
  line: 56
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: clap usage errors share exit 2 with declared refusals but carry no ekr.kernel name, which contract r2 allows and which callers cannot tell apart by status
- file: crates/ekr/src/exit.rs
  line: 117
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: commit on an unseeded store exits 1 as NotSeeded while kernel.yaml declares the no-retained-transaction case as the TransactionNotFound refusal, and retraction_example.rs asserts the exit 1
- file: crates/ekr/src/cli/seed.rs
  line: 21
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: seed input is read to the end with no bound before the parser, and no seed limit is declared that the CLI could apply
```
