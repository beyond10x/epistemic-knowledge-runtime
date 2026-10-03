# Schema transaction evidence

The operator's execution request authorizes story D, `story:schema-transaction-cites-evidence`, under the existing knowledge-inbox release plan and sole PR64. AEP implementing skill 0.19.1. This is an interactive continuation of that authorization, not a new release or independent PR. Objective: vision:o2.

The prerequisite authority/evidence foundations are published on the integration branch; their final acceptance remains open. The operator's plan sequences D after those foundations, so this unit proceeds without falsely marking B complete. Shared kernel, CLI, specification and gate changes remain serialized. E and F remain separate later units within the same carrier. Unrelated draft stories are not selected.

Integration: managed tree `ekr-knowledge-inbox-20261003`, branch `feat/knowledge-inbox-schema`. Unit: managed tree `ekr-schema-evidence-20261003`, branch `ekr/schema-evidence-20261003`, base `0333f526ea7b607c1477fddcd4758d26de0d1694`. Owner: `codex-ekr-knowledge-resume-20261003`. Sequential build directory: `<tmp>/ekr-knowledge-disputes-check.jlypm3/target`; scratch/evidence: `<cache>/ekr-knowledge-prereq-20261003/schema-evidence`. No concurrent build may use this target. Existing owned trees remain recovery/handoff trees, not parallel implementors. Resource preflight observed 19 GiB persistent and 13 GiB tmpfs available; one compiler job, no incremental compilation or debug symbols.

Stage: specification and red-first tests. The coordinator implements locally; this is not independent review. Independent adversary and full integrated gate remain required before final acceptance. Source and evidence commits, bot integration into PR64 and planning evidence updates are authorized by the original implementation/publication request. No ESS refs, tags or unrelated hosted-store work are changed.

## Contract and compatibility

`systems/ekr/domains/kernel.yaml` already declares SchemaTransactionEvidence and its relations to GraphTransaction and retained Evidence. Supporting evidence is the immutable transaction manifest, exposed per schema revision; do not change historical ontology encodings or roots. New rules admit schema declarations plus AddEvidence only; other data operations remain mixed, and supporting IDs must resolve to retained or in-transaction admissible evidence. Existing ordinary transaction manifests retain their exact rule.

Investigation found that knowledge authority currently permits only one transition and has the same schema-evidence refusals as the historical identity profile. New admission therefore requires a new explicit knowledge authority version. Preserve knowledge/1 replay and seed profiles byte for byte; permit reviewed knowledge/1 to knowledge/2 transitions, bind each preview to the active predecessor, invalidate old pending validations, and refuse stale or unsupported targets before mutation. New installs still start with their original seed anchor.

## Verification required

Red-first provider tests must become green for retained evidence and inline evidence schema transactions, reopen/full replay and exact history citations. Old-profile rejection records must continue replaying. Unknown support, uncited additions, every other mixed data operation, stale transitions, forged approval and backward transitions must refuse without canonical mutation. The named `schema_change_exposes_supporting_evidence` scenario must run real CLI/SDK-backed behavior against both providers with a red-capable control. The typed SDK, ontology CLI and read-only viewer must show the same retained support.

The full generated component scenario floors and original A–F acceptance remain unchanged. A focused checkpoint does not close this story or undraft PR64.
