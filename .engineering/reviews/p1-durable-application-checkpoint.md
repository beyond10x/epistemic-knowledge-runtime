unit: durable-kernel application checkpoint — version-persisted-contracts + commit-and-revision-lineage
verdict: red — intentionally incomplete checkpoint; scoped tests pass, recovery qualification and workspace migration remain
cases: kernel+ontology 275 passed, 0 failed, 0 ignored, 0 filtered (first complete invocation); selected seed/application lane 49 passed+1 failed ->56 passed
origin: n/a
wrote-outside-worktree: assigned durable-activation evidence/TMPDIR and exclusive assigned compiler target; exact paths in private inventory
needs-coordinator: integrate checkpoint for disjoint CLI/Explain work; retain active durability stories and remaining acceptance

This checkpoint implements actual retained Propose, Validate and Commit responses, real application and reconstruction, and the initial private preparation/retry protocol. It does not establish complete recovery, full-workspace compatibility, migration, CLI or phase completion. Source opening for this checkpoint is afa7ee471b3eaf9cd3015c00daffa2dea5868dea; that includes coordinator-owned shared contracts and the separate host source. The source commit and complete file hashes accompany this report.

## Delivered behavior and public seam

Runtime::file/sqlite opens with provider coordinates, BootstrapContext and AuthorityStateV1. It derives untrusted retained ontology input from the seed and admits it under the actual host anchor; callers do not supply duplicate ontology. Typed AuthorityMismatch is preserved for state admission; Seed alone maps a valid different host anchor to AlreadySeeded. No default authority or clock is invented.

Runtime::propose_reader uses the frozen bounded exact-byte ingress. Malformed input remains CommitError::Document; actor binding refusal remains ProposalAttribution. Missing transaction/revision and terminal state are distinct typed errors. The kernel exports the error-only PersistenceError alias for consumers forbidden a raw-store dependency. Propose returns ProposalRecordV1, Validate returns actual Validated/Rejected records, and Commit returns Committed/Stale records. Commit result variants box their large records without changing their tagged wire shape.

All ordinary handlers use retained records and real kernel authority. Replay reparses proposals, reruns deterministic validation against the complete basis, applies accepted operations and verifies root/record/occurrence identities. Lifecycle application retains attribution and assessment. Historical loading stops at the selected committed revision, so missing later payloads do not poison a valid earlier read. Runtime::read returns one VerifiedRead containing graph/root, original seed input/result, context/authority/profile, exact canonical coordinates, retained transaction decisions and already verified bytes. Explain may use this capture without reopening a provider.

The new preparation module implements strict ekr.publication-preparation/1 carriers and private Eventlog-backed CAS election. It retains complete ordered native requests and fingerprints, verifies their binding to the elected kernel-admitted decision, and resumes the same request after UnknownCommit. A definite conflict permits a linked attempt; provider-only conflict preserves the domain occurrence/time while canonical loss may elect a distinct Stale occurrence with the original sampled time. Initial executable coverage establishes selected paths, not the complete §94 failure matrix.

Object retention upgrades now use the exact stream version captured with the retained class; the earlier Expected::Any prevented exact preparation authorization. The existing real seed contention case exposed this and the corrected source reaches the actual pending/published outcome distinction.

## Retained red evidence

- application-red-ontology-duplicates-behavior: behavior failure, 22 accepted malformed JSON/YAML inputs across eleven nested collection boundaries. The strict decoder test later passes with positive controls. This closes the checkpoint's explicitly reported inherited collection gap.
- application-red-commands: behavior failure against the old ordinary command path. Later both-provider fresh-process execution shows changed knowledge, exact Many versus inner List values and real retained results.
- application-red-lifecycle: 0 passed, 1 failed; fourteen malformed lifecycle candidates were accepted across both providers. Common structural checks now refuse them; valid retraction/supersession controls apply and preserve temporal/provenance semantics.
- application-red-unknown: 0 passed, 1 failed. A second unresolved Commit created a new EventId/revision identity and sampled time. The original assertion body remains unchanged and now passes in the unfiltered durable command target.
- application-checkpoint-tests: 3 passed, 1 failed, actual preparation-object-expectation on cached seed contention. After exact retention expectation correction, preparation-cached-seed-correction reaches the old AlreadySeeded-only race assertion and still fails.
- application-checkpoint-independent-lanes: 49 passed, 1 failed. The second identical outdated race assertion in seed.rs fails with PublicationInputConflict.
- application-kernel-tests: earlier incomplete regression run has one filtered Unknown witness and a failing source-ownership guard; retained as red, not called a full green. The guard now checks the actual ontology encoder owner. The final package run is unfiltered and passes.
- application-checkpoint-clippy and -2: two test-only lint failures (interceptor type complexity and a one-element loop). A named alias and direct block close them; -3 passes.

The coordinator explicitly approved only the two inherently racing seed loser assertions to accept AlreadySeeded or PublicationInputConflict. Exactly-one-success, absent losing object, and unchanged losing Cache retention assertions remain intact. The sequential published changed-seed test remains AlreadySeeded-only. New deterministic pending and published controls separately require their exact distinct error on both providers, a clock panic if sampled, and identical physical event/public-binding observations before and after refusal. These added controls were run after protocol implementation and are not claimed as new preimplementation reds.

## Executed scoped checks

Exact shell-escaped commands, complete stdout/stderr and individual statuses are retained in adjacent named .command/.log/.status files, with per-run resource observations.

| Lane | Command | Runner result | Exit |
|---|---|---|---|
| application-checkpoint-reconciled | cargo test --locked --offline -p ekr-kernel --test durable_commands --test seed --test adversary_p1_08_seed --test durable_seed --test commit_path | 4+5+13+9+25=56 passed, none failed/ignored/filtered | 0 |
| application-checkpoint-package | cargo test --locked --offline -p ekr-kernel -p ekr-ontology | 275 passed across30 summaries, including2 rustdoc cases; none failed/ignored/filtered | 0 |
| application-checkpoint-clippy-3 | cargo clippy --locked --offline -p ekr-kernel -p ekr-ontology --all-targets -- -D warnings | all selected targets | 0 |
| application-checkpoint-store-clippy | cargo clippy --locked --offline -p ekr-store --lib -- -D warnings | store library only; old store test compilation not implied | 0 |
| application-checkpoint/fmt | rustfmt --check --edition 2021 --config skip_children=true on the exact changed/untracked Rust manifest | empty output | 0 |

The selected predecessor invocation omitted the adversary target and contained eleven durable cases: 49 pass+1 fail. The final selected invocation adds that four-case target and two deterministic controls:56 pass. This is not an unexplained count increase on an identical command. The final combined package command had no identical pre-change complete run; its before count is unmeasured, not invented. Separate failing compile/regression attempts remain available.

The durable command target covers thirteen actual cases, including one fresh-process branch inside its parent test rather than a separate no-op helper. One case executes all six occurrence kinds with semantic, unknown-field, duplicate-field and missing-payload tampering on both providers (48 refusal combinations plus complete-history controls); that matrix still counts as one Rust test. Publication fault injection wraps the real store port under real kernel authority; it does not claim native disk crash injection.

## Changes and ownership

Only kernel, ontology and store source/test paths listed in source-final.sha256 belong to this checkpoint. No AEP, DESIGN, ESS, manifest, lock, gate, vendor or frozen parser changes. Frozen legacy source/tests/vectors have an empty diff against the reviewed opening checkpoint. Root's host source and graph projection/event migrations are ancestors, not implementor authorship.

The private publication carriers and authorization live in store/preparation.rs; the kernel owns commands, replay, application, lifecycle validation and read capture. Existing kernel test migrations retain their behavior under the active format. The old compile-time source guard now reads ontology canonical.rs for ontology fields and transaction.rs for kernel fields; no assertion was replaced by an empty source mention.

An initial runner invocation tried to execute the non-executable scratch run.sh directly and exited126 before Cargo; the same script was then invoked through bash. No source effect. Earlier changed-file rustfmt briefly used edition2024 then was rerun with the workspace's actual2021 edition; the final exact changed-file check is2021. No blanket workspace formatter or legacy regeneration ran.

## Remaining acceptance

See remaining.md for the concrete unfinished list. It includes all-six-decision fresh-process/native uncertainty controls, forged/missing private preparations and no-losing-private-binding contention, old graph/store fixtures, complete current-format/root sensitivity, final guards/mutations, independent review and the full repository gate. This checkpoint deliberately stops before those controls so the disjoint CLI/Explain worker can use the real stable public facade. No acceptance is narrowed or declared complete by integration.

Final post-lint affected-case reruns: application-checkpoint-final-commands executes13/13 durable command cases with no filters, exit0; application-checkpoint-final-ontology executes3/3 decoder cases, exit0. No source edits follow these checks.
