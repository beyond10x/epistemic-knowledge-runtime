---
format: aep.planning-md/3
id: release-plan:next-three-waves-2026-10-02
kind: release-plan
status: active
title: Input safety, historical paging and historical ontology after the released baseline
relations:
- supersedes: release-plan:next-waves-2026-10-01
- serves: vision:o5
revision: 5
transitions:
- {from: "draft", to: "active", at: "2026-10-02T18:02:02Z", actor: "agent:codex-ekr-next-three-root", revision: 2}
---
## Baseline and authorization

The operator requested: "$aep:planning review+revise+refine planning . then dispatch the next 3 waves with $aep:wave".
This preapproves selection, dispatch and integration after review and gates. It authorizes no
new tag, version bump or external dependency release. Baseline is the released main recorded by
git rev-parse 0.0.27^{} below. AEP planning and wave procedures are skill version 0.19.1.

The store review found stale epic scope, missing recognition of released race diagnostics and an
obsolete migration baseline. The affected artifacts are revised; diagnostics do not close the
unexecuted stress acceptance. Open product decisions remain open.

## Ordered waves

| Wave | Existing governed work | Scope and boundary |
|---|---|---|
| input-08 | task:every-yaml-reader-is-bounded; task:identity-scan-reads-a-hand-kept-domain-list | Outside-input YAML limits and dynamic ESS identity coverage; no new domain entity |
| history-09 | task:selected-revision-loads-read-one-event-per-call | Bounded selected-history pages; preserve the selected integrity boundary |
| ontology-10 | task:ontology-at-reads-only-the-schema | Verified historical ontology mapping without selected data replay; preserve opening integrity |

These are existing tasks, not a new epic decomposition. Each wave is rechecked against actual
merged state before dispatch. Builds are serialized because free space is near the repository's
floor. Each unit receives its own managed tree and scratch; the inactive EKR target may be
reused exclusively in sequence under AGENTS.md. No concurrent build shares it.

Cited source scope comes from the read-only correctness and storage scopers and root verification.
The input units may overlap core/lib.rs if YAML needs a new module export: the identity unit owns
identity exports, YAML owns only a decoder export. Integrate YAML before identity and inspect
merge-tree before integration. Subsequent waves overlap store history and kernel read/replay and
are serialized. Planning, changelog, ESS coordination and shared docs belong to the coordinator.

The story scheduler output below is complete, not a task schedule. The pinned CLI accepts typed
scope only on stories. These legacy tasks decompose epics directly; their cited/inferred body
scope and this explicit ordering govern them. The proposed reclamation story appears ready by
graph but its body records unresolved receipt, migration and benchmark design. It is not admitted
to dispatch merely because graph dependencies are terminal.

## Deferred work and limits

Preparation reclamation needs a crash-safe attempt-chain and migration design, receipt compatibility
and a reproducible baseline before dispatch. Store-open verification and persisted-read-model work
must profile the already shipped memo/checkpoint path first. W remains active under its unchanged
validate and commit bounds; its latest recorded commit-scaling acceptance failed. This plan
neither restarts its exhausted adversary loop nor claims the target is implemented.

Native Eventlog changes, receipt format changes, checkpoint cadence, durability changes and the
open P2/P3/type-pack/constraint decisions remain out of scope. Migration-marker identity needs an
explicit compatible specification before a fix; evidence can contain arbitrary marker bytes.
The seed-authority mismatch needs paired CLI/kernel/conformance evidence; it is not documentation-only.
Those items remain governed in their existing artifacts.

## Review and delivery procedure

Generic collaboration agents adapt the plugin scoper, implementor and adversary procedures because
named plugin agent types are unavailable. Critic model sonnet is unavailable; no sonnet execution
will be claimed. The available slots also prevent simultaneous independent critics in every lane:
record the actual adapter, sequence and limitations with each review.

Every implementation starts with a witnessed red regression, then package tests, format and clippy.
Each unit receives adversarial testing, at most the procedure's allowed passes; a red or inconclusive
claim is held with its evidence. Root verifies baseline versus treatment and records every result.
Each combined wave runs task check and required source CI before base integration. No release is
part of wave completion. After publication, exact managed ids are finished and garbage-collected;
retained evidence is recorded first.

## Baseline and complete scheduler evidence

79de37c21f3a9beafc99a8c52c4bd7e053ec6437

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:preparation-blobs-are-reclaimed",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/ekr-kernel/src/checkpoint.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/ekr-kernel/src/commands.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-kernel/src/commit.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-kernel/src/explain.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-kernel/src/migrate.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/ekr-kernel/src/records.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/ekr-kernel/src/replay.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-store/src/eventlog.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-store/src/inventory.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-store/src/log.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/ekr-store/src/preparation.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr-views/src/changes.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/ekr/src/conformance.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/epistemic-knowledge-runtime-design.md"
            },
            {
              "confidence": "cited",
              "path": "systems/ekr/domains/kernel.yaml"
            },
            {
              "confidence": "inferred",
              "path": "systems/ekr/domains/store.yaml"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [],
  "unassessed": [],
  "cycles": []
}

```

## Resource update during input wave

The input wave's resource assignment section supersedes the initial serial-only choice after
free storage recovered. Its recorded storage observation supports a separate identity adversary
target alongside the YAML worker's target. The repository concurrency ceiling still holds and no
concurrent tree shares a target. Subsequent resource changes and the exact ownership handoffs are
recorded in each wave page before dispatch.

## Nearest release checkpoint

The operator asked to find the next checkpoint where a release can be cut. The nearest coherent
checkpoint is input-08 integrated into main: bounded ontology YAML plus dynamic ESS identity
coverage. Proposed patch: 0.0.28, following the released baseline. No version or tag has changed.

The coordinator asked an optional pause-versus-continue question, received no answer, and stated
its assumption to stop at this checkpoint. History paging and historical ontology reads stay
planned and undispatched; neither is a dependency of these input fixes. Resume them only after
reconciling this checkpoint and the operator's next instruction.

Both exact reviewed units are integrated in the candidate. The recovered YAML inventory
correction passed its final allowed adversary; the focused baseline/treatment claims are
VERIFIED and both tasks are implemented. Combined local gates and affected checks after the
current-main documentation merge passed; evidence is recorded in the wave page and task bodies.
Remaining checkpoint conditions are required source CI, exact main integration verification and
managed-tree cleanup. A release still requires its separate version/tag procedure and successful
release checks and artifacts. W's unmet performance acceptance stays active and unchanged.

Current source candidate before these planning-only closing edits: 401a56d43e4b617a078a0dda7b384c4f7a1ef908.
The shared Cargo target is retained because its prior missing-marker cleanup refusal remains;
no alternative deletion or marker fabrication is authorized by this checkpoint.
