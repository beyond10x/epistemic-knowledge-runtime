---
format: aep.planning-md/3
id: story:claims-are-submitted-and-decided
kind: story
status: draft
title: A reader submits a claim, a reviewer decides it, and an approved claim becomes an assertion
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
revision: 1
---
## Context

A consumer wants a signed-in reader to submit a claim — a statement about nodes, with optional
evidence — that a reviewer then approves or rejects. On approval the claim becomes an assertion
whose provenance names the submitter; on rejection it stays recorded with the reason. Until it is
decided, a pending claim is visible but never counted or read as an assertion.

Nothing in the store builds this. `epic:p4-operator-surface` already says that a person's answer
becomes `HumanStatement` evidence and a transaction (design § 81, amendment A1), and the validation
an extraction delta passes through `ekr apply-extraction` (`docs/cli.md`, `ekr apply-extraction`) is
the check a claim would reuse. The requested interface: `submit_claim(subject, predicate, object,
evidence?) -> claim_id` and `decide_claim(claim_id, approve | reject, reason)`, as MCP tools under a
write scope and as CLI verbs.

## Invariants this story holds

- Invariant 1: submitting writes only a proposal. Approval is a `ValidatedTransaction` the kernel
  builds after the same deterministic validation an extraction delta passes; neither the MCP server
  nor the CLI writes canonical state. Agents and readers propose; they never mutate.
- Invariant 4: "a reader said so" is not sufficient on its own. The submitter's statement is
  `HumanStatement` evidence, and the reviewer's decision is recorded beside it; policy decides
  whether that provenance is sufficient for `Accepted`.
- Invariant 7: ontology, reference and cardinality checks run without a model.
- Invariant 9: the claim's text and evidence are untrusted record text (as `epic:p4-operator-surface` says of record text, A14) and pass the
  observation layer's redaction before any model reads them.

## Build (spec first)

1. `systems/ekr/domains/kernel.yaml` first, green under `ess specify validate --path systems/ekr` and
   its conformance suite before any code: a `Claim` entity (id from `Id::mint()`, subject, predicate,
   object, optional evidence, submitter, submitted-at) with the lifecycle pending → approved |
   rejected, the decision record (decider, reason, decided-at, and on approval the revision it
   produced), and the submit and decide commands with their refusals (unknown subject or
   predicate, an object of the wrong type, a decision on a claim that is not pending, a decider who
   is the submitter where policy forbids it).
2. `systems/ekr/domains/views.yaml`: pending claims as their own read, never inside assertion reads
   or counts; MCP tools `submit_claim` and `decide_claim` available only on a server started with a
   write scope, which stays off by default.
3. `systems/ekr/domains/cli.yaml` and `docs/cli.md`: `ekr claim submit` and `ekr claim decide`.

## Acceptance

- `submitted_claim_is_pending_and_not_an_assertion`: after `submit_claim`, the claim reads as
  pending, and assertion reads, counts and the head revision are unchanged.
- `approved_claim_becomes_an_assertion_with_submitter_provenance`: `decide_claim(approve)` produces
  one revision whose new assertion cites the submitter's `HumanStatement` evidence and the decision.
- `rejected_claim_stays_recorded`: `decide_claim(reject, reason)` changes no canonical state and the
  claim reads as rejected with its reason.
- `invalid_claim_is_refused_as_an_extraction_delta_would_be`: a claim naming an unknown type or a
  property of the wrong kind is refused at submission with the same named refusal the extraction
  validation gives.
- `decision_on_a_decided_claim_is_refused`: a second decision is refused by name.
- `write_tools_absent_without_write_scope`: `tools/list` on a server without the write scope names
  neither tool, and a call to either is refused.
- `cli_and_mcp_agree`: the same submission and decision through the CLI and through MCP leave
  byte-identical stores, on both providers.

## Out of scope

Authentication and who is "signed in" (the runtime receives an identity from its caller and does not
verify one); a review queue user interface; research runs that write claims; bulk import of claims.
