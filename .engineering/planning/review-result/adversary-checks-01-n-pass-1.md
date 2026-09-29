---
format: aep.planning-md/3
id: review-result:adversary-checks-01-n-pass-1
kind: review-result
status: active
title: Adversary, wave checks-01 unit N, pass 1
relations:
- reviews: story:store-reading-code-names-no-contents
revision: 1
---
## Verdict

NEEDS-CHANGE on `2e99af36`, six findings; five fixed in `1a833814`, the sixth (a read-only store,
pre-existing) filed as `task:read-verbs-open-a-read-only-store`. The coordinator decided that a store
name that is also a runtime word is reported flagged `runtime_word` instead of dropped, which amends
the story's acceptance line "none of the runtime's own vocabulary is reported": dropping them let a
consumer hard-code names such as `status`, `Commit` or `links`. Held under attack: nothing written on
either provider, `-wal`/`-shm` included; deterministic order; paths as given; directory and non-UTF-8
arguments; raw strings and single-quoted JSON; case-sensitive matching; 5,000 aliases by 2,000 files.

```findings
- file: crates/ekr-views/src/code_names.rs
  line: 581
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a quote of another kind earlier on a line paired with the next quote of its kind and hid a quoted store name after it"
- file: crates/ekr-views/src/code_names.rs
  line: 792
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the 522-word vocabulary exemption silently allowed ontology names such as links, weight, status, owner, Commit and Document"
- file: crates/ekr-views/src/code_names.rs
  line: 811
  category: boundary
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "removing repeated paths was quadratic in the number of sources, 15.3 s for 40,000 files"
- file: crates/ekr-views/src/code_names.rs
  line: 607
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "scanning cost escaped quotes times line length on a line with no partner quote"
- file: crates/ekr-views/tests/code_names.rs
  line: 100
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the hand-kept RUNTIME_VOCABULARY lacked the fields units Q and F add, so it went red at their merge"
- file: crates/ekr/tests/adversary_code_names_cli.rs
  line: 187
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "code-names, head and ontology exit 1 on a read-only store on both providers"
```
