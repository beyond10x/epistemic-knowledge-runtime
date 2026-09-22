# Adversary brief — p1-transaction-membrane-repair

Load the installed aep-drive adversary charter. Read AGENTS.md, COORDINATOR.md, the story and
the complete implementation diff against `581005b` in the assigned membrane tree.
The story's current approved scope and acceptance are authoritative.

Tree id: `ekr-p1-07-membrane`; branch: `codex/ekr-p1-07-membrane`.
Tree: `<worktrees>/ekr-p1-07-membrane`.
Build: `<cache>/b10x-target/ekr-p1-07-membrane`.
Scratch: `<cache>/ekr-completion-20260922/membrane`.
Lease: `codex-ekr-p1-07-adversary`.

Write tests only. Do not edit source, existing tests, the planning store or shared documents.
Observe any new failing case before the full suite. No source mutation, even temporary;
mutations belong in copies under assigned scratch. Source-reading tests use runtime paths.

Attack the actual contract: all assertion subject/predicate/object combinations, surviving
references after deletion, create/delete cancellation and operation permutations, mixed writes
to properties and lifecycle state, opaque constraints/effects, and unsupported operations.
Require successful controls as well as refusals. A refusal attributed to an unrelated validator
is not evidence that the intended check works.

Use the real pipeline. Distinguish introduced defects from pre-existing gaps outside this unit.
Seed admission, real operation application, persisted authority, authentication at submission,
format migration and complete graph projections are known later units; do not silently widen
this unit to implement them or imply they are fixed.

Run package tests and report observed counts and command exit statuses. No new compiler run
below the operator's 10 GB free-space floor. Do not alter other sessions' files or build processes.

Return the required six-line report header and a findings block with fields
`file, line, category, severity, verdict, origin, message`. AEP severity vocabulary is
`blocker | warning | note`; use the charter's verdict and origin vocabulary. Include
`Owners:` attribution in prose without inventing counts that the evidence cannot establish.
Release your lease on handoff. Do not commit, publish, or remove the tree.
