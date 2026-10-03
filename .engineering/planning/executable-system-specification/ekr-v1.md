---
format: aep.planning-md/3
id: executable-system-specification:ekr-v1
kind: executable-system-specification
status: validated
title: systems/ekr — the runtime's executable system specification, v1
relations:
- specifies: epic:p1-kernel-ontology-core
model_digest: f356a296c15cb806b72a218ce953f41b3edb6194bcefd41f350d20df79d80073
revision: 9
transitions:
- {from: "draft", to: "validated", at: "2026-09-21T01:59:07Z", actor: "agent:claude", revision: 2, imported: true}
- {from: "validated", to: "conforming", at: "2026-09-23T20:38:55Z", actor: "agent:claude-lazy-sutton-p1-14", revision: 3, decided_on: {"recorded":{"ess_conformance":1}}, imported: true}
- {from: "conforming", to: "validated", at: "2026-10-03T17:26:54Z", actor: "agent:codex-ekr-knowledge", revision: 4, decided_on: {"recorded":{"ess_conformance":0}}}
---
## Current contract

systems/ekr is the runtime contract, maintained with the verified ESS 0.52.0 generator. It now includes observation retention, incubation, authority transitions, attention, human decisions and schema proposals alongside the original canonical kernel domains. Generated artifacts and all committed suites are held by regeneration checks.

## Current validation and conformance

Current E compiled specification digest: f356a296c15cb806b72a218ce953f41b3edb6194bcefd41f350d20df79d80073.

The released ESS 0.52.0 validates all nine specification files and regenerates both contract trees and all seven suites. The integration inventory remains 19 generated obligations with answered floor 19 and unavailable ceiling zero. Finite typed fixture resolution is explicit, including exact payloads for unchanged generated document literals; unsupported recursive fixture declarations were not retained.

Actual integration report/2 documents currently show, on each of file and SQLite, 16 passed, one failed, zero errors/skips and two unsupported. SubmitSchemaProposal/answered has a generated literal without admissible supporting evidence; the released generator cannot currently supply the needed recursive typed fixture. Both ApplySchemaProposal outcomes are still unimplemented F work. The report completion timestamps are fixed scenario-clock values, not wall-clock execution evidence. Reports and observed test logs are retained separately.

Earlier kernel/authored conformance evidence remains historical at its own source digest. Current full task check is not green, and specification validation does not establish global conformance. The F design refinement is independently approved but its runtime implementation and conformance remain outstanding.

## Historical record

The imported 2026-09-21 validation used ESS 0.26.0 and the original four-domain specification. The imported 2026-09-23 conforming transition was based on the kernel evidence available then. Those transitions and their evidence remain unchanged; they do not establish conformance of the expanded current model.

## Preserved decisions

General constraint-language and broader migration decisions remain outside this delivery. Named acceptance scenarios and source digests belong to the release plan and individual AEP stories; specification validation alone does not close them.
