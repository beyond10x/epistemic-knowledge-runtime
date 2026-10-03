# Reviewed schema proposals

Story E, `story:propose-better-vocabulary`, implements the operator's accepted A–F plan under the
sole PR64. AEP implementing skill 0.19.1. The original execution authorization covers this
serialized continuation; no new release or extra PR is proposed. Objective: vision:o2.

Integration is `ekr-knowledge-inbox-20261003` on `feat/knowledge-inbox-schema`, checkpoint
`5398f77681e21c382fee1608eead1e80c3afe65f`. The managed unit is
`ekr-schema-proposals-20261003` on `ekr/schema-proposals-20261003`, based on that checkpoint.
Its owner session is `codex-ekr-schema-proposals-20261003`. Source work is local and unpublished;
all wanted commits must survive through the sole carrier before either tree is retired.

The unit first refined and validated the ESS contracts at `e578455f067244a9ff37ed1b2da669c456993cec`,
then generated both model trees and compiled the fresh semantic workspace. Detailed reports are
under `.engineering/reviews/knowledge-schema-proposals-contracts/`. Independent read-only
contract review requested clearer correspondence between signed target and retained recording
metadata; the comment was corrected. No implementation conformance is implied by this prerequisite.

The AEP story carries exact cited/inferred scope and source scoping findings. Implement discovery,
immutable submission/preview, exact human approve/reject history, typed CLI/session/SDK operations
and read-only proposal pages. Reuse the existing independent reviewer trust and provider abstraction.
No model provider/scheduler, browser/MCP writes, automatic entity creation, destructive migration,
entity merge/split, reclamation or hosted PostgreSQL work. Unknown entity resolution remains visible.

One compiler job uses the owned sequential build directory
`<tmp>/ekr-knowledge-disputes-check.jlypm3/target`; no other tree may build into it concurrently.
Incremental compilation and debug symbols remain disabled. Inode-sensitive tests use the shared
`EKR_INODE_TEST_TMPDIR` override. Preflight observed 15 GiB persistent and 9.4 GiB tmpfs available.
Retain logs outside disposable outputs; no cleanup of another session's files or live processes.

F remains separate implementation work after E. Its contracts must first add durable application
elections and qualified item/provenance receipts. Review identified a required atomicity guard:
canonical-head CAS alone cannot see a concurrent rejection on the independent review stream.
Each F canonical publication must atomically guard the effective review as well as canonical head.
The detailed requirement is recorded in F's AEP story.

Native file/SQLite tests, reopened full replay, exact generated conformance, mutation controls,
CLI/SDK examples, visual inspection, both demonstrations and the integrated full gate remain
required. Do not lower the existing zero-unavailable component floors or mark PR64 ready early.

## F runtime continuation

The generated F prerequisite is committed as 8ec17507fd8c24175100bae9318d9a8b7196c2b7.
Current implementing skill is 0.19.2. The operator's original execution request authorizes
this continuation; no new release or PR is requested. The older prerequisite paragraph above
records the earlier state, not the current contract coverage. The F story records the reviewed
protocol and exclusive ownership.

| Unit | Managed tree / branch | Owner / scope | Evidence directory | Stage |
|---|---|---|---|---|
| Physical application persistence | `<worktrees>/ekr-schema-application-store-20261003`, `ekr/schema-application-store-20261003` | scope_retention; store, graph event codec and corresponding tests | `<retained-evidence>/schema-application-store` | baseline and implementation |
| Generated canonical transaction codec | `<worktrees>/ekr-application-transaction-codec-20261003`, `ekr/application-transaction-codec-20261003` | author_contracts; new kernel application_transaction module; one module-registration line | `<retained-evidence>/application-transaction-codec` | tests and source; awaiting compiler lane |
| Kernel application orchestration | existing `ekr-schema-proposals-20261003` / `ekr/schema-proposals-20261003` | root; all other kernel and cross-package constructor changes | `<retained-evidence>/schema-application-kernel` | implementation |

Each delegated tree starts at the exact prerequisite commit. Each owner takes its named
codex-ekr lease. The one existing bounded compiler directory remains sequential; handoffs are
explicit and a terminal process is observed before another tree compiles. No concurrent build
lane or disposable cleanup is authorized by this continuation. Briefs live in each assigned
retained-evidence directory. Runtime tests, independent review and the full integration gate
remain outstanding. These units implement parts of F, not separate stories or completion claims.
