---
format: aep.planning-md/3
id: task:conformance-cannot-observe-command-responses
kind: task
status: draft
title: Adopt conformance assertions for command responses
relations:
- serves: vision:o2
- derived_from: story:ess-conformance-kernel
- decomposes: epic:p6-maintenance-observability
revision: 3
---
## What is wrong

Measured by adversary pass 1 on wave p1-14 unit p1-14-conformance (`review-result:p1-14-conformance-adversary-r1`, finding 3): setting the Propose and Validate command responses to `None` in the conformance target leaves all 36 generated scenarios passing. The unit reports, from reading the ESS 0.29.0 source, that the authored-scenario format cannot check a response, and a payload can map only from a top-level response field. So the lossless projection of `ProposalRecordV1` and `ValidationCommandResult` is not held by the suite.

## What holds it today

`crates/ekr/tests/adversary_p1_14_conformance.rs::proposal_responses_carry_the_exact_document_bytes`: an EKR test, outside the suite.

## What closes this

An ESS capability for observing command responses in generated or authored scenarios, adopted by pinning that ESS version. This is an upstream question for the ESS repository.

## Current capability and remaining adoption

The original ESS 0.29.0 capability limitation is historical. The pinned ESS 0.52.0 supports authored ess-scenario/4 response assertions, introduced in 0.39.0. The current EKR suite has not adopted those assertions for Propose and Validate, so removing the responses still leaves the relevant scenario statuses unchanged. The corrected adversary controls require the relevant untampered scenarios to pass and compare full status maps; unrelated unsupported cases no longer count as response failures.

Adopt explicit response assertions and expose any missing actual response projections, then remove this gap only after a response-removal mutation makes those named scenarios fail. A generator upgrade alone does not complete this task. The existing native response-byte test remains useful independent coverage.
