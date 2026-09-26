---
format: aep.planning-md/2
id: epic:p4-operator-surface
kind: epic
status: draft
title: P4 — Operator surface and projections
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p3-incubation-integration
- serves: vision:o5
revision: 1
---
## Context

The design stops at query scopes (§ 47) and an explain chain (§ 62). v1 and v2 both proved that the
product is the surface a person reads and answers: an attention page with numbered items, rendered
briefs, an agent-facing read surface, approvals on outward writes. Amendments 81–84 add these to the
design; this epic builds them.

## Outcome

Crates `ekr-views`, `ekr-mcp`:

- attention queue with numbered items; an answer becomes `HumanStatement` evidence and a
  transaction (A1);
- approvals: every outward write is an approved transaction (A3); obligations with an external clock
  and horizons past which a fact reads `?` (A4);
- rendered views with templates lifted from `org-brain-successor/views/*.j2` (A2); a human work
  backlog projection distinct from the frontier (A15);
- query scopes of § 47; `ekr explain` printing the § 62 chain;
- `ekr-mcp` read tools lifted from `org-brain/src/mcp.rs`; record text is untrusted evidence (A14).

## Acceptance

The attention page renders from canonical plus disputed state; an answer commits a transaction;
`ekr explain` prints the § 62 chain for a canonical assertion; two renders of the same revision are
byte-identical.

## Carries

A1, A2, A3, A4, A14, A15.
