---
format: aep.planning-md/3
id: architecture-decision-record:0015-evidence-erasure-keeps-history
kind: architecture-decision-record
status: accepted
title: ADR 0015 — Evidence is erased in place with an audit stub, and its dependants are marked, not deleted
relations:
- decides: story:evidence-erasure
revision: 3
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-17T16:51:19Z", actor: "agent:ekr-controller", revision: 3}
---
## Status

Accepted by the operator on 2026-10-17, with option A. Erasure changes what the store's
append-only history holds, so the decision was the operator's. `story:evidence-erasure` builds
it, spec first, after `story:merge-entity-applies` and `story:retype-node`.

## Context

A consumer must remove evidence bytes from a store for a privacy cleanup: a whole evidence record,
or a span within one. Today `RetractAssertion` ends an assertion's activity (design § 35.1, § 36)
and keeps the evidence it cited stored and served; the only way to remove the bytes is to rebuild
the store from a filtered head, which replaces its revision history.

Design § 35.3 allows physical reclamation "only after reachability, policy, retention, and audit
requirements permit it"; § 60 asks for policy-aware deletion of evidence independently of
epistemic history, and lists what may happen to dependent assertions. Invariant 5 says committed
revisions are immutable and physical reclamation happens only through the maintenance path.
Invariant 4 says every canonical assertion has sufficient provenance according to policy.
`story:evidence-purge-keeps-history` (draft) proposes a purge selected by a predicate that also
deletes every assertion citing only the purged evidence.

## Decision

Option A below: an `EraseEvidence` operation, committed as its own revision through the kernel,
naming one evidence record, or a byte span within it, and a stated reason.

1. The erasure revision records an audit stub per erased item: the evidence id, the span (or the
   whole record), the actor, the time and the reason; never the erased bytes or a hash an attacker
   could test guesses against.
2. The maintenance path then removes the bytes from every place the store keeps them (the payload,
   retained preparations, receipt copies, replay checkpoints), and every read surface answers the
   stub instead of the bytes, at the head and at every earlier revision.
3. Earlier revisions keep their identity and order. Their roots are recomputed over the stub, and a
   replay checkpoint records that the history it verified contains an erasure.
4. No dependent assertion is deleted. Each assertion citing erased evidence is marked as resting on
   erased evidence; policy then decides its state (keep with the mark, Disputed, or revalidated from
   other evidence), as § 60 lists. An assertion with other admissible evidence keeps it.

## Options

- **A. Erase in place with an audit stub; dependants marked (accepted).** History keeps every
  revision; only the bytes go. Cost: a revision's roots change after the fact, so a root recorded
  outside the store before the erasure no longer verifies, and replay must accept a stub in place of
  a payload.
- **B. Purge by predicate, sole dependants deleted** (`story:evidence-purge-keeps-history`). Removes
  assertions resting only on the purged evidence and tombstones them. Cost: it deletes knowledge the
  consumer may want to keep marked rather than lose, and selects by predicate rather than by record.
- **C. Rebuild the store from a filtered head** (today's workaround). No new operation. Cost: all
  revision history before the rebuild is gone, and every consumer's pinned revision breaks.
- **D. Encrypt each evidence payload under its own key and erase by destroying the key.** History
  bytes stay unchanged, so roots still verify. Cost: a key store becomes part of the store's
  durability; existing stores hold plaintext and need a migration; a span cannot be erased alone.

## Consequences

Accepted with A: `story:evidence-erasure` builds it, spec first; `story:evidence-purge-keeps-history`
is narrowed to a predicate front end over erasure or archived. Roots published outside the store
before an erasure are documented as not verifying after it. The erasure is never outward-facing
(invariant 6 does not apply).
