---
format: aep.planning-md/3
id: review-result:integrated-conformance-refusal-independent-r6
kind: review-result
status: active
title: Independent review finds unobserved writes before upgrade refusal
relations:
- reviews: story:schema-transaction-cites-evidence
revision: 1
---
needs-revision

Owners: 1 coordinator finding; 0 delegated implementor findings.

Source-and-log review against fe9efea8a found that ApplyUpgrade captured its publication boundary, but returned upgrade_refusal immediately on error. Appended publications did not reach the runner, and end_scenario checked only preservation of the original prefix and bytes.

A concrete negative probe is to append a real valid proposal during the command and then return a recognized upgrade refusal. This review identified the missing assertion from source; it did not execute that probe.

No additional findings in supplied-input use, external precondition setup, fixture initialization, inventory dispatch, shared inode scratch selection or the earlier conformance-control corrections. The coordinator's first log reports 72 passing scenarios per provider; those counts do not resolve this observation gap. No independent execution, writes or full-gate claim.

```findings
[{"file":"crates/ekr/tests/support/upgrade_target.rs","line":457,"category":"mutant","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"A refused upgrade returns before checking appended publications, allowing its generated no-event assertions to pass despite a write followed by refusal."}]
```
