---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-parallel-r2
kind: review-result
status: active
title: Parallel critic, round 2, roadmap 2026-09-28 drafts
relations:
- reviews: story:add-evidence-operation
- reviews: story:observations-are-retained
- reviews: story:v1-chat-raw-becomes-observations
- reviews: story:mcp-read-tools
- reviews: story:seed-envelope-v3-references-payloads
- reviews: release-plan:roadmap-2026-09-28
revision: 1
---
approve or needs-revision — see below.

**needs-revision**

I worked read-only in `<worktree>`, comparing this round's drafts against `review-result:roadmap-2026-09-28-parallel-r1`. Both round-1 findings are fixed: `story:seed-envelope-v3-references-payloads` now carries `depends_on story:seed-envelope-decoded-once` and `depends_on story:history-loaded-once-per-process` plus prose naming both files and "starts after both are merged" (revision 6); and `release-plan:roadmap-2026-09-28` now has an explicit paragraph naming `docs/cli.md` as shared by `story:add-evidence-operation`, `story:v1-chat-raw-becomes-observations`, `story:mcp-read-tools` and the active `story:ekr-session`, with "the coordinator owns the merge" as the named remedy. One new collision remains unnamed.

**Findings**

```
story:v1-chat-raw-becomes-observations — its Build adds a new `ekr import` subcommand, which per crates/ekr/src/cli/mod.rs (a single `Command` enum and one dispatch match in `run`, one arm per verb, no per-verb registration elsewhere) requires editing that file; the active story:ekr-session (wave ingest-01, in flight) cites `crates/ekr/src/cli/mod.rs` in its own scope, and unlike story:mcp-read-tools (gated "M1 released", where M1 closes wave ingest-01) this story's wave obs-01 carries no gate on ingest-01 finishing — neither body nor the release-plan names the mod.rs collision — crates/ekr/src/cli/mod.rs:91-220,334-393 (inferred, code read) vs aep plan artifact show story:ekr-session (scope, cited) vs aep plan artifact show release-plan:roadmap-2026-09-28 (obs-01 gate, cited)
```

**What I read:** `release-plan:roadmap-2026-09-28` (revision 2), the 5 drafted stories, `story:ekr-session`, `story:history-loaded-once-per-process`, `story:seed-envelope-decoded-once` via `aep plan artifact show`; `aep plan artifact waves --kind story --status draft --format json`; `git diff` on the changed decision-blocker and story files; and `crates/ekr/src/cli/mod.rs` directly to establish the CLI's verb-registration structure.

**What I could not establish:** whether `story:mcp-read-tools`'s new `ekr mcp` verb would collide on `mod.rs` too is moot — its wave gate ("M1 released") already sequences it after `story:ekr-session` merges, so I did not pursue it further. Whether `obs-01`'s silence on ingest-01 is deliberate (an accepted concurrent-start the operator intends) or an oversight is not mine to say; I only note the plan does not say either way. Whether the eventual `mod.rs` diffs from `story:ekr-session` and `story:v1-chat-raw-becomes-observations` would textually conflict (adjacent enum variants) or merge cleanly is a question neither I nor the store's recorded scope can answer without both diffs existing.

```findings
- file: crates/ekr/src/cli/mod.rs
  line: 92
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: story:v1-chat-raw-becomes-observations adds a new `ekr import` subcommand, which requires editing crates/ekr/src/cli/mod.rs's Command enum and dispatch match; the active story:ekr-session (wave ingest-01, in flight) cites that same file in its own scope, and obs-01 carries no gate on ingest-01 finishing (unlike read-01, gated "M1 released") — neither the story nor the release-plan names this collision
```
