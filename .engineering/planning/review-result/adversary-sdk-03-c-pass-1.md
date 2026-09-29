---
format: aep.planning-md/3
id: review-result:adversary-sdk-03-c-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-03 unit C, pass 1
relations:
- reviews: task:sdk-types-the-check-reads
revision: 1
---
## Verdict

CONFIRMED on `27912edb`, two notes, both fixed in `09943aec`: a code-name kind a newer `ekr` adds now
reads as `CodeNameKind::Other` instead of failing the whole read (the engine-side exact read still
reports the drift), and the coverage list requires a quality document without `constrained_share`.
Held under attack: argv for every documented argument shape (paths starting with `-`, `--`, spaces,
unicode, repeats, `u64::MAX`), session, one-shot and CLI bytes equal on both providers, refusals typed
as `Refused` or `Fault` as `docs/cli.md` states.

```findings
- file: crates/ekr-sdk/src/read/checks.rs
  line: 199
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "CodeNameKind was closed, so a kind a newer ekr adds failed the whole code-names read, against the module doc"
- file: crates/ekr-sdk/tests/check_reads.rs
  line: 642
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the required-coverage list omitted a document without constrained_share"
```
