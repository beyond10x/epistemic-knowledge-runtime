---
format: aep.planning-md/3
id: story:ekr-store-integrity-fixes
kind: story
status: active
title: Store integrity fixes
summary: Store comparison, migration and the release gate report only what they checked
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T04:08:30Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T04:08:31Z", actor: "human:timo", revision: 3}
---
## Outcome

Store comparison, migration and the release gate report only what they checked.

## Work

Review of EKR 0.0.42, findings 15 16 36. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

15. Store comparison overlooks the evidence cited by the schema. Two stores held the same revision, type definitions, facts and evidence records. One schema change cited evidence ending in 0401, the other 0402. ekr ontology returned the distinct citations. ekr compare said equal=true and counted zero differences in all six categories. Include the schema version’s evidence in semantic equality and add a comparison fixture differing only in that citation.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/compare.rs#L255
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/ontology.rs#L78
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L3823


16. A store migration can report success with an open stage left behind. The base was at revision 1. A Begun stage had committed revision 2 inside its tenant. SQLite-to-SQLite migrate exited 0 and the two base stores compared equal. The destination listed zero stages and refused the original stage ID. The source still held the staged work. After publishing on the source, a fresh copy carried canonical revision 2 but again had no stage records. Warn and identify active stages before cutover, or carry their state. Document which audit records move.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L1995
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2114
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-store/src/inventory.rs#L126
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-kernel/src/migrate.rs#L257


36. The release gate counted a PostgreSQL migration as passed without performing it. The SQLite-to-PostgreSQL migration test returns immediately when the PostgreSQL fixture configuration is absent. The release job reported it as passing and the workflow provided no fixture or flag requiring one. Give the hosted migration an integration job that executes against a disposable PostgreSQL instance and reports executed and skipped cases separately.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/postgres_cli.rs#L29
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/postgres_cli.rs#L78
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/.github/workflows/correctness.yml#L87
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/epistemic-knowledge-runtime-design.md#L5895
https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/44971928696/job/137053388213


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
