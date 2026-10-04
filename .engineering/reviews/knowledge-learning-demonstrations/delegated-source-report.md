unit: story:apply-approved-refinement — knowledge learning end-to-end demonstrations (rev36)
verdict: blocked
cases: not executed; source-only handoff under coordinator-owned Cargo lane
origin: n/a
wrote-outside-worktree: assigned private knowledge-learning-demonstrations evidence directory
needs-coordinator: yes, demonstrations.patch; integrate and execute the two named cases

1. Unit and acceptance

Adds two real Runtime journeys, each looping over File and SQLite. No simulated responses or fabricated canonical/replay state. Only new tests/knowledge_learning_demonstrations.rs and new tests/support/knowledge_learning_fixtures.rs are changed; existing shared signature fixtures remain read-only. Base is exact checkpoint 5e5e814457888529aa57aea3e9c12b3f1d94c6c9.

Health: two independent observed payloads and immutable interpretations remain parked because Project.health is undeclared. Discovery must group the repeated gap with both sources/observations. Full replay before proposal verifies retention. An exact externally signed proposal adds optional String health and maps each facts[0] from its fully qualified source version. Application must produce two Accepted canonical claims with exact retained bytes, mapping and Observation derivation links, close the discovered gap, survive full replay, and leave all provider event streams unchanged on imports/application retries.

Ownership: seed two contradictory One owned_by claims and perform the explicit authority upgrade. Both must become Disputed and expose an actual evidence-backed attention question. Retained supplied ownership observations/interpretations support the exact signed distinction between operationally_owned_by and business_owned_by. Two explicit relations and CopyRelation mappings must commit before the single final correction step retracts both broad ambiguous claims. Assert actual canonical endpoints, retained original claims/evidence/history, SchemaCorrection explanation, unchanged historical dispute/ontology, full replay, and no-write repeat.

2. Patch shape

Two new Rust files, 674 inserted lines as reported by git apply --stat. The patch excludes all existing fixture and production files. No ESS/AEP/generated, CLI/SDK/viewer or publication changes.

3. Red

Not run, as explicitly instructed. The source was prepared as a reproducible verifier and root owns execution and any production fix. No failure or success is claimed.

4. Verification

Exact-file rustfmt --edition 2021 --check passed (exit 0). No Cargo build/test/check/clippy and no conformance suite ran. Disk observation: 13 GiB persistent, 7.6 GiB tmpfs available. Named cases:
- recurring_parked_project_health_becomes_reviewed_vocabulary_and_integrated_knowledge
- ownership_question_becomes_explicit_relations_and_explained_correction_without_losing_history

5. Concrete unexecuted risk and boundaries

Source inspection suggests schema_gaps::schema_gap_request reads immutable import-time receipts through retained_interpretation without joining application processing receipts. After adding health, classify_gaps may therefore report integrated facts as UnresolvedReference. The health test retains the empty-discovery assertion because successfully integrated items should leave the gap request. This is an unexecuted hypothesis, not a confirmed defect. Root owns any reproduced production correction.

Test signatures use the existing externally provisioned fixture key through the real review ingress; no product signing capability is added. All graph mutations run seed/upgrade/apply through Runtime. Repeat checks compare published_events across the whole provider feed, not only canonical head. Shared fixture-only unused helper/import allowances match existing repository tests; no product checks or assertions are weakened.

6. Outside-worktree writes

Only the assigned private evidence directory: demonstrations.patch, source-sha256.txt, report.md, handoff-sha256.txt and outside-paths.txt. Full private paths are retained in outside-paths.txt; this report remains portable. Managed tree ekr-knowledge-learning-demonstrations-20261004 remains intact for coordinator integration; previous worker trees/evidence were preserved.
