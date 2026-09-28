---
format: aep.planning-md/3
id: task:replay-refusals-have-no-forged-history-case
kind: task
status: implemented
title: Kernel replay refusals have no case that forges the retained history
relations:
- serves: vision:o2
- derived_from: story:commit-and-revision-lineage
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-23T00:17:31Z", actor: "agent:claude-579ad7f9-p1-12", revision: 2, decided_on: {"recorded":{"test_result":1,"review_outcome":5}}, imported: true}
- {from: "proposed", to: "active", at: "2026-09-23T00:17:31Z", actor: "agent:claude-579ad7f9-p1-12", revision: 3, decided_on: {"recorded":{"test_result":1,"review_outcome":5}}, imported: true}
- {from: "active", to: "implemented", at: "2026-09-23T00:17:32Z", actor: "agent:claude-579ad7f9-p1-12", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":5}}, imported: true}
---
## What is missing

The store-level fold cases that forged history were migrated in wave p1-11; the property they held
now belongs to kernel replay (ADR 0007), and several kernel refusals have no case that forges a
replayed history to reach them. Found by the `store-fold` unit of wave p1-11.

- A commit whose `root.revision` does not follow its parent: refused at
  `crates/ekr-kernel/src/replay.rs:396` (`root.revision == number`), no case forges it.
  `crates/ekr-store/tests/fold_rules.rs::a_revision_that_does_not_follow_its_parent_is_refused`
  was removed because it cannot compile against the current port.
- A second seed occurrence, and a first occurrence that is not a seed
  (`crates/ekr-kernel/src/commit.rs:27,60`).
- A decision with no retained proposal; a commit after a Stale or Rejected decision; a commit
  event whose payload `knowledge_root` disagrees with its receipt.
- `StoreError::{SeedIsNotFirst, SeedNotStored, ProposalMissing, NoCommitAuthority,
  RevisionOutOfOrder, KnowledgeRootDisagrees}` are raised nowhere in `src/`.

The kernel refuses these at command time; replay of a forged history is unexercised.

## What closes this

A kernel test target that writes each forged history directly through the providers (the way
`crates/ekr-kernel/tests/durable_commands.rs` injects corruption) and asserts reopen refuses with a
named error, on both providers. Remove or wire the unused `StoreError` variants.
