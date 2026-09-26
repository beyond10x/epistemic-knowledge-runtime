---
format: aep.planning-md/2
id: task:the-proposer-field-is-unauthenticated
kind: task
status: implemented
title: An agent that names a different proposer can validate its own transaction
relations:
- serves: vision:o2
revision: 4
---
## What is wrong

Design § 6.10 says the proposer is never the sole basis of validation. `ekr-kernel`'s authorization
validator checks it as `actor != tx.proposer`, where `actor` comes from the pipeline and
`tx.proposer` is a `pub` field the proposal carries.

So the check has one operand from the runtime and one from the thing being checked. An agent that
names a **different** agent as the proposer then validates its own transaction as itself, and the
check passes.

Found by the adversary of wave p1-05, unit 2, pass 2, which graded it `INFEASIBLE` and was right to:
the kernel receives no submitter identity, so it cannot be written as a case and cannot be fixed
there.

## Why the kernel should not grow the fix

The validator's own doc argues the actor belongs to the pipeline "rather than part of the proposal —
a proposer who could name their own validator would be answering the question the check exists to
ask." That argument is correct and it is the same argument one level up: a proposer who can name
their own *proposer* is answering the same question from the other side.

Authenticating a submitter is a transport concern. `AGENTS.md` invariant 7 wants the deterministic
validators plain and rerunnable, and a validator that authenticates is neither.

## What closes this

Whatever accepts a proposal from outside the process binds `tx.proposer` to the authenticated
submitter, or refuses the proposal. Then the kernel's check has two operands it can trust and the
validator does not change at all.

Until then the obligation is unowned, and the one thing to do now is say so where a reader of the
validator will find it: `crates/ekr-kernel/src/validate/authorization.rs` names this task, so the
story that builds a submission path inherits it rather than rediscovering it.

The candidates for that story are `story:ekr-cli` — where the submitter is the person at the
terminal and the binding is trivial — and whatever P4 gives the MCP surface, where it is not.
