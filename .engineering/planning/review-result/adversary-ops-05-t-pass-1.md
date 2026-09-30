---
format: aep.planning-md/3
id: review-result:adversary-ops-05-t-pass-1
kind: review-result
status: active
title: Adversary, wave ops-05 unit T, pass 1
relations:
- reviews: task:two-tests-pass-under-load
revision: 1
---
## Verdict

CONFIRMED on `7f65b24b`: both reworked cases pass under load (4 of 4 at load 21–27 beside a full
`cargo test`), and the mutants the page-stream case guards stay red. Its guard case in `8317a549`
fails when the temp filesystem never hands a freed inode back (tmpfs); this repository's TMPDIR and
CI's runners are ext4.

```findings
- file: crates/ekr/tests/adversary_sdk01_h_replaced_store.rs
  line: 123
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "nothing bounds how often the deleted-and-created-again variant skips (3 of 4 under load, every run on tmpfs); the files-replaced-inside variant covers the same reopen path"
- file: crates/ekr/tests/adversary_sdk01_h_replaced_store.rs
  line: 148
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the skip message names a cause the code does not check"
- file: crates/ekr/src/cli/viewer/index.html
  line: 534
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "a page accepting a cleanly ended stream without its end line passes the cut-stream case, since its cut always arrives as a network error"
```
