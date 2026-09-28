---
format: aep.planning-md/3
id: review-result:roadmap-2026-09-28-parallel-r1
kind: review-result
status: active
title: Parallel critic, round 1, roadmap 2026-09-28 drafts
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

I worked read-only in `<worktree>`, running `aep plan artifact show` on the release-plan, the 5 drafted stories, and the 3 stories of active wave `ingest-01`, plus `aep plan artifact waves --kind story --status draft --format json`.

**Findings**

```
story:seed-envelope-v3-references-payloads — its scope names `crates/ekr-kernel/src/seed.rs`, the exact file the active story:seed-envelope-decoded-once (wave ingest-01, in flight) rewrites to cache envelope decode; the story names no dependency on that wave finishing — aep plan artifact show story:seed-envelope-v3-references-payloads (scope, inferred) vs aep plan artifact show story:seed-envelope-decoded-once (scope, cited)

story:seed-envelope-v3-references-payloads — its scope names `crates/ekr-store/src/eventlog.rs`, the exact file the active story:history-loaded-once-per-process (wave ingest-01, in flight) rewrites to add per-handle verification caching; the story names no dependency on that wave finishing — aep plan artifact show story:seed-envelope-v3-references-payloads (scope, inferred) vs aep plan artifact show story:history-loaded-once-per-process (scope, cited)

story:add-evidence-operation — this, story:mcp-read-tools and story:v1-chat-raw-becomes-observations (roadmap's ingest-02, read-01 and obs-01, which the roadmap explicitly allows read-01 to run "beside") plus the active story:ekr-session (wave ingest-01) all land on `docs/cli.md`, and none of the four bodies nor the roadmap names the shared file — aep plan artifact waves --kind story --status draft --format json (collisions block, inferred) and aep plan artifact show story:ekr-session (scope, cited)
```

**What I read:** 9 artifacts (release-plan:roadmap-2026-09-28; the 5 drafted stories; the 3 active `ingest-01` stories), via 9x `aep plan artifact show` and one `aep plan artifact waves --kind story --status draft --format json`. Surfaces: 0 cited, 5 inferred (all 5 drafted stories — the CLI itself labels every one of their scope entries `inferred`), 0 unplaceable.

**What I could not establish:** none of the 5 draft bodies cites its own paths in prose — every surface came from the store's recorded scope metadata (inferred), not from a path named in the artifact's Context/Build/Acceptance text, so a stronger (cited) confirmation would need `git grep` on the symbols named (`AddEvidence`, `seed::envelope`, `ekr.views`) or `aep plan artifact graph`, neither of which I ran. Whether the `seed.rs`/`eventlog.rs` collision is a mergeable text conflict or a deeper design incompatibility between "cache the decoded envelope" (active) and "restructure the envelope format" (draft) is a design-critic question, out of my lane — I flag only that the two are unaware of each other. The discrepancy between the roadmap's named waves (ingest-02/obs-01/read-01) and the CLI's own topological wave grouping is a sequencing question, also out of my lane.

```findings
- file: aep plan artifact show story:seed-envelope-v3-references-payloads
  line: null
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: its scope names `crates/ekr-kernel/src/seed.rs`, the exact file the active story:seed-envelope-decoded-once (wave ingest-01, in flight) rewrites to cache envelope decode; the story names no dependency on that wave finishing
- file: aep plan artifact show story:seed-envelope-v3-references-payloads
  line: null
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: its scope names `crates/ekr-store/src/eventlog.rs`, the exact file the active story:history-loaded-once-per-process (wave ingest-01, in flight) rewrites to add per-handle verification caching; the story names no dependency on that wave finishing
- file: aep plan artifact waves --kind story --status draft --format json
  line: null
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: story:add-evidence-operation, story:mcp-read-tools and story:v1-chat-raw-becomes-observations (roadmap's ingest-02, read-01 and obs-01, which the roadmap explicitly allows read-01 to run beside) plus the active story:ekr-session (wave ingest-01) all land on `docs/cli.md`, and none of the four bodies nor the roadmap names the shared file
```
