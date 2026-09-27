---
format: aep.planning-md/2
id: task:proposed-node-carries-reference-aliases
kind: task
status: active
title: A node committed from ProposeNew carries the reference's aliases
relations:
- serves: vision:o5
revision: 3
---
## Context

Found by the adversary on unit T of wave p2p3p4-02
(`review-result:adversary-p2p3p4-02-integrate-pass-1`, finding F3, origin pre-existing, verdict
plausible). The adversary reports that `NodeDraft` in `crates/ekr-kernel/src/transaction.rs:62`
carries no aliases field; the coordinator has not re-read the kernel to confirm.

If that holds, a caller that acts on `ResolutionOutcome::ProposeNew` by committing a new node cannot
give the node the reference's aliases, so the resolver never matches it again and the same reference
proposes a new node on every call: duplicate nodes for one real-world entity, which invariant 3
("names are not identities") and design § 45 exist to prevent.

## What to establish first

- Whether any committed operation sets a node's `aliases` (node creation, a property update, a named
  operation) — enumerate every write path, not one keyword.
- Who calls `resolve` and then writes: today nobody (`story:ekr-resolve-verb`, wave B, is the first).

## Acceptance

A node committed from a `ProposeNew` outcome carries the reference's aliases, and resolving the same
reference against the next revision returns `Resolved` with that node.
