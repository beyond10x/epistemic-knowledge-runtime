# Explicit authority transition development evidence

This is a partial implementation of `story:show-disputed-knowledge`. It does not complete
the knowledge inbox, schema learning, PR acceptance or the full repository gate.
`verification.txt` records measured execution counts and the generated specification digests.
The first line identifies the parent checkpoint, not a commit containing this increment.

The new `Commit` methods preview and apply an exact human-reviewed transition. The original
seed anchor and historical event bytes remain unchanged. The transition atomically retains
the generated record, target profile, public reviewer policy, signed proof, statement and
independently provisioned host binding. Both native providers compare that binding with their
actual tenant audience. Full replay verifies the retained inputs and reproduces the new root.
Old pending validations require revalidation under the new authority.

The knowledge profile recomputes deterministic One-cardinality conflicts after the transition
and ordinary commits. Both competitors become disputed; equal values do not conflict.
Original validator attribution is retained for later recomputation. The prior authority's
replay rules and accepted assessments remain visible at their original revisions.

`cargo test --locked -p ekr-kernel` completed successfully on the frozen implementation;
`upgrade-kernel-final.log` includes the authority-upgrade integration tests and existing
kernel regression suite. Child-process recovery executions are included in the measured
totals. The focused provider log overlaps this run and must not be added as distinct cases.
The upgrade tests exercise file and SQLite stores, reopen/full replay, stale and forged
reviews, pending validation invalidation, new conflicting commits, transition tampering,
missing evidence and exact retries. Before-write/after-write port faults are followed by
a fresh-process retry. These are not native mid-transaction crash tests.

The lost-CAS regression was observed failing with `PublicationInputConflict` before its fix
(`upgrade-race-red.log`). A lineage-wide preparation slot stranded a newly reviewed attempt
after another writer advanced the stream. The corrected version-four preparation uses the
exact reviewed stream predecessor, allowing a new review after that race while keeping retries
of one attempt identical. The initial transition implementation preceded its integration test;
only this later race correction claims a red-first cycle.

ESS validation passed. Fresh semantic and data synthesis with the pinned development executable
matched both generated directories byte for byte, excluding only ESS's operational ownership
journal. The command summaries, obligations and refusals remain in `upgrade-drift-final.log`.
This check did not compile the standalone generated workspace or establish released-generator
acceptance. Member-only formatting and `git diff --check` also passed without compilation.

The retained cross-crate log is a failed intermediate run: a source-reading architecture guard
matched one whitespace layout of an expression that rustfmt split. Its correction removes
whitespace before checking the same expression; the authority refusal assertion remains.
The corrected cross-crate run, final workspace Clippy, the newest graph bridge test, native
crash coverage, B's authored ESS conformance, attention/CLI/SDK/viewer delivery and the full
`task check` remain pending. Earlier green Clippy output is not final evidence for this increment.
Stories C-F and both end-to-end demonstrations remain in scope.

Owners: the preparation-key defect and source-format guard correction were coordinator-owned.
All review here was root-local; no independent adversarial review or human approval is claimed.
The provider tests use synthetic signing fixtures, not an operator's production approval.

After the kernel process exited successfully, its raw logs were retained outside the target
directory. The owned, unused build output was cleaned in response to the shared disk constraint.
No other session's outputs were cleaned. Published copies replace the local home prefix;
raw evidence remains in the task-owned cache. No additional build lane was started.
