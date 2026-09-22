---
format: aep.planning-md/1
id: task:p1-required-correctness-ci
kind: task
status: active
title: Run and require repository correctness on pull requests
relations:
- derived_from: epic:p1-kernel-ontology-core
- serves: vision:o2
revision: 3
---
## Context

The independent review found only the shared privacy workflow. A privacy check does not compile or test the candidate. The approved membrane wave includes a required correctness job.

## Acceptance

Pull requests and main pushes run `task check` under the pinned Rust compiler, ESS and AEP versions. The job receives read-only permissions and no application credentials, and candidate execution uses pull_request rather than pull_request_target. Branch protection requires the exact Repository correctness job. Record a successful CI run against the integrated candidate before closing this task.

## Scope

Cited: `.github/workflows/correctness.yml`, `rust-toolchain.toml` and the repository's required-check configuration. The local compiler is 1.98.1; ESS 0.26.0 and AEP 0.55.0 are pinned by full source revision. The Rust pin also stabilizes trybuild diagnostics while the declared minimum remains unchanged.
