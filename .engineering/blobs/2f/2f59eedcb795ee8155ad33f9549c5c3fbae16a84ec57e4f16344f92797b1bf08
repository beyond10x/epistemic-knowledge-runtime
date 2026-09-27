---
format: aep.planning-md/1
id: review-result:p1-13-cli-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-13 CLI unit
relations:
- reviews: story:ekr-cli
revision: 1
---
Adversary pass 2 on unit p1-13-cli (wave p1-13), `aep-drive:adversary`, fresh binary processes on
both providers.

Owners: 5 findings, 5 implementor (introduced).

Verdict: CONFIRMED. cases: executed 106→114, red 1.

Cases in `crates/ekr/tests/adversary_p1_13_cli_r2_exit_map.rs` (every kernel error maps to its
declared exit and name, green; the library seam keeps the binary's help and version exit, red) and
`crates/ekr/tests/adversary_p1_13_cli_r2_reads.rs` (negative `--valid-at` never takes a following
flag; snapshot equals the kernel carrier at every revision and selector and § 65 holds; explain of a
seed-only assertion superseded later; explain of a replacement accepted before its supersession;
every outcome, refusal and fault writes one result or nothing; an unreadable document is a fault
and records nothing — all green).

Mutation probe on `exit.rs`: four arms survived the 106 earlier cases and are killed only by these
files. The coordinator fixed finding 5 (`run` now returns clap's help and version text as success)
and verified it: 114 passed, clippy exit 0.

```findings
- file: crates/ekr/src/exit.rs
  line: 101
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: turning the DocumentError::Io fault into a StructurallyInvalid refusal leaves all 106 existing cases green, although `ekr propose <directory>` reaches that arm
- file: crates/ekr/src/exit.rs
  line: 92
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: no existing case pins SeedError::Store(other) to exit 1, and reaching it needs a provider write failure after a successful open, which I did not construct
- file: crates/ekr/src/exit.rs
  line: 118
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: no existing case separates CommitError::Store from NotSeeded, so mapping store faults to a refusal survives the suite
- file: crates/ekr/src/exit.rs
  line: 131
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: ProjectionError::Unverified to exit 2 survives the suite, and nothing reaches it through the binary because the store rejects corruption on open and seed refuses non-human evidence sources
- file: crates/ekr/src/cli/mod.rs
  line: 121
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the uncalled library seam `run` reports clap's --help and --version as usage failures with exit 2 while the binary exits 0
```
