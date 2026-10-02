---
format: aep.planning-md/3
id: review-result:next-waves-1001-parallel-safety-r1
kind: review-result
status: active
title: Parallel-safety critic, next waves 2026-10-01, round 1
relations:
- reviews: release-plan:next-waves-2026-10-01
revision: 1
---
needs-revision

1. release-plan:next-waves-2026-10-01 — The correct-07 collision paragraph orders only "S follows G" and lists V's surface as `crates/ekr-sdk` and `crates/ekr/src/cli/extraction.rs` only, but V's own scope cites `crates/ekr/src/cli/session.rs` and G's body names the pair; the branches confirm it: `origin/impl/extraction-verb` splits `respond` into a new `answer` fn at `session.rs:258–266`, which `origin/impl/typed-divergence` edits inside the old `respond` body (`session.rs:~300`), an overlapping-hunk conflict nothing in the plan orders — `.engineering/planning/release-plan/next-waves-2026-10-01.md:47`.
2. release-plan:next-waves-2026-10-01 — The shared-file list says "`docs/cli.md` is shared by V, B and S" but omits A (extract-07b), whose scope records collisions with V on `docs/cli.md`, `cli/agent.rs`, `tests/agent_cli.rs`, and the live branches `origin/impl/evidence-attachment` and `origin/impl/extraction-verb` both edit `docs/cli.md` and `crates/ekr/tests/agent_cli.rs` today — `.engineering/planning/release-plan/next-waves-2026-10-01.md:48-49`.

What I read: the release plan and units M, V, G, S, F, B, A, W, K, O via `show`; `waves`; `graph --format json`; `git diff --stat 30703729f origin/<branch>` and targeted diffs on the four in-flight branches. Surfaces cited: M, V, G, F, B, A, W; inferred: S, K, O.

What I could not establish: S and O have no branch; whether S touches `crates/ekr-store/src/inventory.rs`, M's file.

```findings
- file: .engineering/planning/release-plan/next-waves-2026-10-01.md
  line: 47
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the correct-07 collision paragraph orders only S after G and omits that V also edits crates/ekr/src/cli/session.rs in lines overlapping G's edit, which V's scope, G's body and the live branches confirm"
- file: .engineering/planning/release-plan/next-waves-2026-10-01.md
  line: 48
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the docs/cli.md shared-file list omits A (extract-07b), whose scope and live branch already edit docs/cli.md and crates/ekr/tests/agent_cli.rs beside V"
```
