---
format: aep.planning-md/3
id: review-result:hosted-sdk-startup-final-1
kind: review-result
status: active
title: SDK required-readiness startup compatibility review
relations:
- reviews: story:hosted-read-serving
revision: 1
---
# SDK viewer startup correction review

Owners: 0 findings, 0 coordinator, 0 implementor.


Independent, bounded static review of the eight-file correction over base `ea93d149c4d7576723a2109565f026c1c79897fc`. The current tracked diff and retained `<cache>/ekr-hosted-runtime/sdk-viewer/candidate.patch` both hash to SHA256 `47e70962a394a95710ddd708d9f1ade020bb81094c6eb3e2252d880e46e8bd6d`. No reviewed product source or tests were edited by this reviewer. The reviewer authored the earlier PostgreSQL/migration foundation and search entry, but did not author this SDK correction.

## Inspected behavior

- `crates/ekr-sdk/src/viewer.rs:62` gates `--require-ready` on the opened binary's numeric version being at least 0.0.28. Earlier supported binaries receive their original viewer arguments. The comparison uses the existing ordered major/minor/patch version type, not a lexical comparison or SDK package version.
- `crates/ekr/src/cli/view.rs:193` opens one `Held`, calls `current()` and the existing verified `head::root` before `http::announce`. Missing stores fail their existing open path; unseeded stores fail the nonempty-root requirement; incomplete migration histories fail the verified head path. The admitted `Held` is retained in the serving loop, so there is no independent successful probe followed by a second ordinary open.
- Failure before announcement unwinds the listener and held runtime. Existing SDK malformed/absent/timed-out announcement paths still kill and wait for the child and collect its stderr tail. Successful viewer stop/drop also retains kill/wait behavior. This patch does not change the existing process timeout or child-cleanup mechanism.
- The clap boolean defaults to false. Without the flag, the listener still announces with no opened store; existing health and embedded-page paths bypass store work. Lazy admission/retry, replacement checks, and MCP behavior remain unchanged. The flag intentionally makes startup admission a prerequisite for that viewer instance, rather than changing hosted defaults.
- CLI and SDK docs distinguish lazy default announcements from the SDK's required-readiness contract. The operational-write clarification agrees with the existing writable-provider open path; physically read-only source assertions remain in the tests.

## Evidence inspected, not executed by this reviewer

Retained files under `<cache>/ekr-hosted-runtime/sdk-viewer/`:

- `version-red.log`: the added version/argument contract test failed before the correction (0 passed, 1 failed).
- `cli-red.log`: both new required-readiness cases failed because the earlier CLI rejected the flag (0 passed, 2 failed).
- `sdk-green.log`: document drift 13 passed; SDK session 16 passed and 2 ignored; spawn-busy 6 passed. The original `viewer_spawn_zero_returns_a_url_whose_head_answers` regression is unchanged and passes. The new Rust stub checks 0.0.19/0.0.27 without the flag and 0.0.28/0.1.0 with it. These argument-compatibility cases are simulated binary versions, not executions of four released engines.
- `cli-regressions.log`: agent CLI 29, documentation 18, hosted HTTP 15, read-only behavior 8, viewer CLI 15; total 85 passed, zero failed or ignored. The hosted suite includes both new cases and the existing default-lazy health/recovery case. New cases cover missing/unseeded/seeded File and SQLite stores, and interrupted File copy completion; writable and physically read-only variants are exercised.

Total inspected green test evidence: 120 passed, 0 failed, 2 ignored. The two ignored SDK cases are not claimed as passes. No Cargo, standalone test binary, client, PostgreSQL fixture, or deployment was run by this reviewer. Final clippy completion and any later source hash are implementor handoff evidence and must be recorded separately if they change this candidate.

Final handoff confirmation: implementor froze the same patch SHA above with no subsequent source edits. The retained `clippy.log` completes the all-target `ekr`/`ekr-sdk` lint lane successfully; implementor reports exit 0, and reports fmt/diff-check exit 0. No additional executable is needed for this static review. This review is anchored to base plus patch hash, not to an as-yet-uncreated commit.

```findings
[]
```
