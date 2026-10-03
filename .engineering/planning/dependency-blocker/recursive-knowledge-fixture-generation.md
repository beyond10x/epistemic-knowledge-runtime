---
format: aep.planning-md/3
id: dependency-blocker:recursive-knowledge-fixture-generation
kind: dependency-blocker
status: open
title: Released ESS cannot admit recursive knowledge fixture contracts
relations:
- blocks: story:propose-better-vocabulary
- blocks: story:apply-approved-refinement
withholds: test_result
revision: 2
---
## Missing capability

The pinned released ESS 0.52 generator refuses typed fixture contracts that reach a named recursive declaration, including recursion behind Optional and List. InterpretationDocument and SchemaProposalDocument contain finite valid values but their command fixtures are refused during conformance synthesis. The exact refusal is ESS-SYNTH-001, `recursive response type cannot be finitely admitted` under fixture inputs.

## Observed evidence

The full synthesis attempt refused ImportInterpretation/answered, ImportInterpretation/refused, SubmitSchemaProposal/answered and SubmitSchemaProposal/refused. The retained diagnostic and bounded alternative probes are under .engineering/reviews/knowledge-schema-review-admission/fixture-generation/. Removing the two recursive fixture bindings restores synthesis, but the synthesized empty proposal has no supporting citations and correctly cannot pass runtime admission. A finite authored proposal with a Bytes payload fixture compiles; that is separate authored coverage and does not discharge the generated execution floor.

## Completion condition

A supported upstream release must admit finite values under these recursive typed fixture contracts, or provide an explicit finite structured witness facility that preserves command types and actual supplied input. Regenerate with an exact verified pin and execute the unchanged required scenarios against file and SQLite with observable state and replay. No input substitution, invented supporting citation, Json widening, generated-model hand transcription, unavailable-ceiling increase or lowered execution floor is authorized.

## Ownership and continuing work

ESS source, releases and integration remain owned by the existing release coordinator. The message bridge refused delivery of the capability report, so no upstream acknowledgement is claimed. No new ESS pull request or source mutation was made. EKR can continue adapter compilation, truthful authored checks and F contract/runtime work while generated conformance remains incomplete. Specification validation and passing kernel tests are not substitutes for this missing conformance evidence.

## Upstream issue

A minimal two-file reproduction was validated and synthesized with released ESS 0.52.0. Validation exits 0; synthesis exits 0 but writes zero scenarios and one ESS-SYNTH-001 refusal for the finite recursive List input fixture. The complete reproduction and exact diagnostic are published in https://github.com/beyond10x/ess/issues/416, created through the bot-authenticated Gates API as b10x-bot[bot]. No ESS source, ref, release or held bundle was changed. The blocker remains open until a supported released capability closes it and EKR executes the generated obligations.
