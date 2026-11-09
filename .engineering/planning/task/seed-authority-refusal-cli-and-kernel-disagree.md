---
format: aep.planning-md/3
id: task:seed-authority-refusal-cli-and-kernel-disagree
kind: task
status: active
title: The CLI and the kernel refuse a seed under another host authority differently
relations:
- derived_from: story:store-open-semantics
- serves: vision:o5
- decomposes: epic:p6-maintenance-observability
- decomposes: epic:zero-open-issues-2026-11
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:54:59Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-09T21:55:00Z", actor: "agent:claude-ekr-zero-issues-coordinator", revision: 4}
---
## Context

Wave p5-01, `story:store-open-semantics` correction round 2 (commit `14d1493`): `ekr seed` on an
existing store now checks the host's authority against the retained one first and refuses
`bootstrap-authority-mismatch`, exit 1, as `docs/cli.md` § The host document says for every store
verb. The kernel's `Runtime::seed` and the conformance target still answer `AlreadySeeded`, which is
what `systems/ekr/domains/kernel.yaml` declares (implementor report, round 2).

## Acceptance

- The kernel contract and the CLI agree on what a seed under a different host authority is refused
  as: either `kernel.yaml` and `Runtime::seed` move to the authority mismatch, or `docs/cli.md`
  names the kernel's answer. One case holds the chosen answer on both providers, through the CLI
  and through the conformance target.

## Scope

Derived 2026-11-09 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr` seed open path and conformance target, against the `ekr.kernel.Seed` contract — cited
- **Files:** `crates/ekr/src/cli/mod.rs:1552-1570` (`open_to_seed` returns `bootstrap-authority-mismatch` before admission) — cited
- **Files:** `crates/ekr-kernel/src/commit.rs:498` (`retained_seed` maps `AuthorityMismatch` to `AlreadySeeded`) — cited
- **Files:** `systems/ekr/domains/kernel.yaml:1473,1561` (`already-seeded` declared for a differing host authority anchor) — cited
- **Files:** `crates/ekr/src/conformance.rs:281-283` (says the target differs from `ekr seed` "by design"), `:687-695` — cited
- **Symbols:** `Runtime::seed`, `StoreError::AuthorityMismatch`, `StoreError::AlreadySeeded`, `ekr.kernel.AlreadySeeded` — cited
- **Documents:** `docs/cli.md:131,203-206,4004,4008` — cited
- **Also likely:** `crates/ekr/tests/adversary_p1_13_cli_exit_contract.rs:204-226` and `crates/ekr/tests/adversary2_p5_01_store_open.rs:76-110`, which pin the CLI's current answer — cited
- **Also likely:** `crates/ekr/tests/conformance.rs` as home of the new both-provider case — inferred, no differing-authority case exists there today
- **Also likely (kernel-moves answer only):** `crates/ekr/src/exit.rs:175-182`, `crates/ekr-kernel/src/runtime.rs:503`, `crates/ekr-kernel/tests/durable_commands.rs:518-522`, `systems/ekr/conformance/suite.json` and `baseline.json`, `docs/epistemic-knowledge-runtime-design.md:3709,4089,4191` — inferred
- **Confidence:** high — the story names the commit, `Runtime::seed`, `kernel.yaml` and `docs/cli.md`, and the tree shows both answers at the cited lines
- **Would collide with:** any unit touching `open_to_seed` in `crates/ekr/src/cli/mod.rs`, the `ekr.kernel.Seed` outcomes in `kernel.yaml`, the synthesized `suite.json`, or the refusal table in `docs/cli.md` — cited
- **Safety fact:** the differing-authority seed refusal writes nothing under either name, so no retained result depends on which name is chosen — step 2 (`docs/epistemic-knowledge-runtime-design.md:3709`, `crates/ekr-kernel/tests/durable_commands.rs:524`), unproven — cited
