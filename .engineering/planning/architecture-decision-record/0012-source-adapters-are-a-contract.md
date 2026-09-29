---
format: aep.planning-md/3
id: architecture-decision-record:0012-source-adapters-are-a-contract
kind: architecture-decision-record
status: accepted
title: ADR 0012 — Source adapters are a contract the engine specifies; third-party connectors stay out of the engine
relations:
- decides: epic:p2-observation-layer
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-09-28T23:00:22Z", actor: "agent:claude-coordinator", revision: 2}
---
## Status

Accepted on 2026-09-29 by the operator, in the coordinator session: "i think it must for now just
be specified via trait so consumers of this can implement their own ... maybe we should keep 3rd
party targeting connectors out of this engine".

## Context

Roadmap P2 (`docs/roadmap.md` § 3 and § 4) placed source adapters for Slack, GitLab, Jira,
Confluence and GitHub in the engine, in an `ekr-adapters` crate lifted from the v2 instance. The
first consumer instance already reads its own sources and asks for GitLab, then Jira, then
Confluence after chat (`release-plan:roadmap-2026-09-28`). Four P2 decision-blockers ask how
observations, source units, checkpoints and adapter unit ownership fit together.

## Decision

The engine specifies the source-adapter contract and does not ship third-party connectors.

- The engine owns: the adapter trait (what an adapter yields, how a unit and its checkpoint are
  named, what a poll reports), observations and their idempotency, checkpoint storage, poll health
  and coverage, and redaction before any model input (invariant 9). The trait is declared in ESS
  (`ekr.observe`) and in Rust; one adapter over fixture records ships with the engine's tests.
- Consumers own: connectors to named third-party systems (chat, issue trackers, wikis, source
  hosting) and their credentials, implemented against the trait.
- `ekr-adapters` leaves the crate map. The raw importer of the v1 predecessor's chat cache is a
  consumer concern too.

## Consequences

- `decision-blocker:adapter-unit-ownership` is answered: the consumer's adapter owns its units,
  and the trait says how it declares them to the engine for coverage.
- `decision-blocker:observation-retention-path`, `source-unit-granularity` and
  `checkpoint-unit-cardinality` stay open; they are questions about the engine's side of the trait
  and are settled in `story:source-adapter-contract`.
- `story:v1-chat-raw-becomes-observations` is archived: it targeted one source's cache.
- `docs/roadmap.md` § 3 and § 4 carry a dated note pointing here.
