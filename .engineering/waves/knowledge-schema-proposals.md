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
