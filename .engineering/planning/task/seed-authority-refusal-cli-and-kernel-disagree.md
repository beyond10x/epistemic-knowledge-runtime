---
format: aep.planning-md/1
id: task:seed-authority-refusal-cli-and-kernel-disagree
kind: task
status: draft
title: The CLI and the kernel refuse a seed under another host authority differently
relations:
- derived_from: story:store-open-semantics
revision: 1
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
