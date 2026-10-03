# Evidence read cache correction

AEP implementing skill 0.19.1. Single corrective unit under the operator's standing authorization to implement and publish required hosted-serving corrections. No additional approval is inferred from silence. The separate corrective release remains subject to the full repository gate and exact-head CI. One-story scope does not need a decomposition critic panel.

## Selection and scope

Selected `story:viewer-evidence-reuses-admitted-revision`, serving O2. `story:preparation-blobs-are-reclaimed` is deliberately not selected: provider reclamation is outside this serving correction. Scope is the viewer handler/tests and optional existing integration tests/documentation. The read-only scoper confirmed the existing index API; no kernel/provider change is authorized. Cold index loading is not claimed to become constant work.

The proposal-stage verb returned these lists (before the selected story moved active):

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
        },
        {
          "id": "story:viewer-evidence-reuses-admitted-revision",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/ekr/src/cli/view.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/cli.md"
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

## Ownership and checkpoints

- Coordinator integration: managed `ekr-evidence-cache`, branch `agent/evidence-read-cache`, path `<worktrees>/epistemic-knowledge-runtime/ekr-evidence-cache`.
- Implementor unit: managed `ekr-evidence-cache-unit`, branch `impl/evidence-read-cache`, path `<worktrees>/epistemic-knowledge-runtime/ekr-evidence-cache-unit`, created after this opening commit.
- Build: `<cache>/b10x-target/epistemic-knowledge-runtime`, used by only one checkout at a time as repository AGENTS.md permits; no concurrent compiler, tests or documentation readers share it.
- Scratch: `<cache>/ekr-evidence-cache/unit`; coordinator/full-gate evidence: `<cache>/ekr-evidence-cache/gate`.
- Native agent runs the `aep:implementor` procedure, then a separate native agent runs `aep:adversary`; plugin agent types are not exposed by this harness. Root owns all planning writes and release actions.
- Stage: unit `557937e5b9a40f847b75205f494233473901a237` passed its focused gate; independent adversary in managed `ekr-evidence-cache-review` is running. Base: `20c31fd1af508ed00477f4308e00d3b09ab88fb3`.
- Estimated focused build storage from the scoper: 2–3 GiB, not a measurement. Prior full-gate build cost was measured separately; monitor actual growth and retain the 10 GiB free-space floor. Debug info and incremental compilation disabled, two build jobs.
- Authorization covers the unit commit, integration/closing commits, verified base publication and a separately gated corrective release. No consumer data, paid extraction or unrelated work enters this wave.

Preflight disk observation:

```text
Filesystem     1024-blocks      Used Available Capacity Mounted on
/dev/nvme0n1p2   888795864 817104084  26469756      97% /
```

## Required evidence

Actual viewer request seam fails before the fix and passes after it by seed-replay count, exact payload and refusal checks. Existing package suite and formatter/lint pass. Independent adversary checks cache invalidation, historical membership, unavailable payload and response compatibility. Full combined gate plus required CI precede upstream release. Each step retains its own exit; ignored/skipped cases are explicit. Native harness token/tool accounting is reported only if available, never estimated.

## Unit handoff

The implementor report at `<cache>/ekr-evidence-cache/unit/report.md` records the deterministic replay regression red then green, exact retained bytes, historical membership and replacement refusal. Its affected gate reports 230 passed, zero failed and one existing manual screenshot ignore. The broader package run was deliberately interrupted with observed exit 143; it is not a gate pass. Root will run the full workspace gate after independent review. A pre-existing raw File payload overwrite discrepancy remains separately tracked as `task:held-file-payload-overwrite`; this patch does not change provider trust semantics.

The native agent thread limit refused a new adversary thread. The existing independent scoping/documentation worker now runs the adversary procedure in its own review tree; it did not implement the viewer change. Shared build ownership is exclusive to that review until explicit handoff. Corrective release candidate version is 0.0.29; neither this version preparation nor unit success is release acceptance.
