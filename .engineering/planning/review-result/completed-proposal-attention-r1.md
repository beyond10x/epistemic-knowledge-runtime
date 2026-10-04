---
format: aep.planning-md/3
id: review-result:completed-proposal-attention-r1
kind: review-result
status: active
title: Completed proposal attention independent review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new coordinator/kernel implementor findings; 0 delegated implementor findings.

Bounded source/log review of the attention completion predicate, approved-question wording, CLI documentation and focused regression over `e55fdd655559d052273cda4669b4437da00b5b8b`.

`crates/ekr-kernel/src/attention.rs:181-190` removes a proposal question only when that proposal's private captured application prefix contains its schema commit and no original mapping or correction remains. Mapping completion uses the full qualified mapping key, including mapping digest; selected corrections remain pending until the authenticated final correction commit. Elections, preparations, mutable receipt claims and another proposal's progress do not satisfy the new predicate. The prefix already filters verified commits to the captured revision. An empty mapping list alone cannot hide a proposal before its schema commits, and an explicit Unresolved correction prevents completion after schema application.

The change only omits the current question; proposal, approval, application and receipt history remain retained. Incomplete approved proposals now ask about integration or renewed review, while unreviewed proposals retain the approval question. Existing rejection filtering is unchanged. Documentation accurately describes those boundaries and the existing ListAttention obligation to exclude settled questions needs no model change.

Inspected implementor evidence under `<retained-evidence>/schema-application-kernel/`:

- `completed-proposal-attention-red.log`: the schema-only regression fails because the completed proposal remains alongside the other pending proposal; one failed test. The run stops in its first provider branch.
- `completed-proposal-attention-green.log`: the initial schema-only both-provider regression passes; one passed, zero failed.
- `completed-proposal-attention-final.log`: now terminal. The expanded attention test passes in 11.38 seconds, looping File/SQLite with and without explicit Unresolved correction. It checks exact remaining proposal identities, retirement/preservation of the old question handle, unchanged retained approval/application/receipts, no read-side events and approved-question wording. The existing schema_proposal_reviews target also finishes with eight passed, zero failed.

Inspected SHA-256 fingerprints:

- `crates/ekr-kernel/src/attention.rs`: `8fa0df43dcd3cbae121d87c20cf555c20fa71bbdb12d39f8ef0f2bb62ceb23fa`
- `crates/ekr-kernel/tests/schema_proposal_attention.rs`: `0ea00a525542aa055df3c1f251ae5bcc5724ae1a497c07bdce2dec01476eacfe`
- `docs/cli.md`: `71dbd5f50face0051028b44dc2b48ca7adc6aa1a5b299160511037f76e2364ea`

Limitations: source/log review only; all execution belongs to the coordinator. This reviewer ran no builds, tests or mutations and made no source/AEP/publication changes. The new attention fixture does not itself execute a multi-mapping partial application; that qualification was source-inspected through the existing remaining-work helper. No new concrete defect was established, and no whole-F or full-gate acceptance is claimed.

```findings
[]
```
