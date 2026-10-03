---
format: aep.planning-md/3
id: executable-system-specification:ekr-v1
kind: executable-system-specification
status: validated
title: systems/ekr — the runtime's executable system specification, v1
relations:
- specifies: epic:p1-kernel-ontology-core
model_digest: 7e8583c5dd6b1d7e52589c443b3c8b6c5233df495af62c0e960428a3a47ece11
revision: 7
transitions:
- {from: "draft", to: "validated", at: "2026-09-21T01:59:07Z", actor: "agent:claude", revision: 2, imported: true}
- {from: "validated", to: "conforming", at: "2026-09-23T20:38:55Z", actor: "agent:claude-lazy-sutton-p1-14", revision: 3, decided_on: {"recorded":{"ess_conformance":1}}, imported: true}
- {from: "conforming", to: "validated", at: "2026-10-03T17:26:54Z", actor: "agent:codex-ekr-knowledge", revision: 4, decided_on: {"recorded":{"ess_conformance":0}}}
---
## Current contract

systems/ekr is the runtime contract, maintained with the verified ESS 0.52.0 generator. It now includes observation retention, incubation, authority transitions, attention, human decisions and schema proposals alongside the original canonical kernel domains. Generated artifacts and all committed suites are held by regeneration checks.

## Current validation and conformance

Current compiled specification digest: 9b1977c0ec3cbd55738865efb90771a8bac2f9b36ef8a6034c666be950b34345.

Fresh contract generation/comparison/compilation, specification validation and suite regeneration pass; retained logs are in .engineering/reviews/knowledge-integrated-conformance/.

The complete kernel component inventory passes on file and SQLite; the imported reports preserve exact counts and suite bytes. Authored knowledge cases exercise retention, disputes, human answers, upgrades and schema evidence with reopen and full replay. The integration component still contains unsupported scenarios and E/F are unfinished. The specification is therefore validated, not globally conforming; the full delivery gate remains open.

## Historical record

The imported 2026-09-21 validation used ESS 0.26.0 and the original four-domain specification. The imported 2026-09-23 conforming transition was based on the kernel evidence available then. Those transitions and their evidence remain unchanged; they do not establish conformance of the expanded current model.

## Preserved decisions

General constraint-language and broader migration decisions remain outside this delivery. Named acceptance scenarios and source digests belong to the release plan and individual AEP stories; specification validation alone does not close them.
