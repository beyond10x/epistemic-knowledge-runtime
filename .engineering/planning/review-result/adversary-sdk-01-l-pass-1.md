---
format: aep.planning-md/3
id: review-result:adversary-sdk-01-l-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-01 unit L, pass 1
relations:
- reviews: story:node-gains-an-alias
revision: 1
---
## Verdict

The kernel held under every attack (checkpoint restore, same-basis race, migration, sessions, a
2,000-case property); one stale doc line fixed and one untested path pinned in `939f4f19`; the SDK
drift is for the merge of unit D.

```findings
- file: docs/cli.md
  line: 291
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the ekr operations section says fourteen kinds while the binary lists fifteen since AddAlias"
- file: crates/ekr-kernel/src/validate/structural.rs
  line: 312
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "dropping the created-node type lookup admits a duplicate alias on a node created in the same transaction and the unit own tests stay green"
- file: crates/ekr-sdk/tests/document_drift.rs
  line: 85
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "unit D exhaustive match and 14-kind list will fail to compile once AddAlias merges"
```
