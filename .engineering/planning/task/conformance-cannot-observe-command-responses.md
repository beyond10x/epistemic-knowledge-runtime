---
format: aep.planning-md/2
id: task:conformance-cannot-observe-command-responses
kind: task
status: draft
title: The conformance suite cannot observe Propose and Validate responses
relations:
- serves: vision:o2
- derived_from: story:ess-conformance-kernel
revision: 1
---
## What is wrong

Measured by adversary pass 1 on wave p1-14 unit p1-14-conformance (`review-result:p1-14-conformance-adversary-r1`, finding 3): setting the Propose and Validate command responses to `None` in the conformance target leaves all 36 generated scenarios passing. The unit reports, from reading the ESS 0.29.0 source, that the authored-scenario format cannot check a response, and a payload can map only from a top-level response field. So the lossless projection of `ProposalRecordV1` and `ValidationCommandResult` is not held by the suite.

## What holds it today

`crates/ekr/tests/adversary_p1_14_conformance.rs::proposal_responses_carry_the_exact_document_bytes`: an EKR test, outside the suite.

## What closes this

An ESS capability for observing command responses in generated or authored scenarios, adopted by pinning that ESS version. This is an upstream question for the ESS repository.
