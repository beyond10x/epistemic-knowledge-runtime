# Search and agent entry

The operator requests live search results, visible MCP connection guidance and llms.txt,
under existing authorization to implement and publish generic upstream capabilities.
This wave follows AEP implementing skill 0.19.1. The authorization covers the unit commit,
review corrections, integration, closing planning commit and publication; the separately
authorized source release still requires the full release procedure.

## Selection

`aep plan artifact waves --kind story --status active` selected:

- wave 1: `story:live-search-agent-entry`
- collisions: `[]`
- unassessed: `[]`
- cycles: `[]`

This is one story. A decomposition critic panel is skipped because there is no multi-item
decomposition to compare. A read-only scoper has checked the existing Rust renderer, viewer
routes and browser harness. Typed scope records cited existing files and inferred additions.

## Roles and ownership

The coordinator alone owns AEP writes, the integration branch and the release procedure.
The existing worker will perform the `aep:implementor` role and independent review follows
the `aep:adversary` role procedure. This host uses general collaboration agents rather than
native plugin role types; that dispatch adaptation is explicit.

Coordinator: managed `ekr-search-agents`, branch `agent/search-agents`.
Unit: managed `ekr-search-agents-unit`, branch `agent/search-agents-unit`, based on opening
commit `7e71751e2`; tested static checkpoint `798b7d5fc776143390da5e4f0a6114bef86cc800`,
not pushed or integrated. Follow-up checkpoint `63f508cad432fea986af8696a5e3e98bb904252c`
retains the independent tests and corrects the exact CLI-option expectation; it is also
not pushed or integrated. The source binary is unchanged by these test-only commits.
Review: managed `ekr-search-agents-review`, branch `review/static-agent-guidance`, test-only
checkpoint `b73f1118f7c3bb457f485351fa579fb75184a8f2` above the static implementation.
The bounded review found no issues; its four additional process tests and the existing
three-case driver all passed. `review-result:static-agent-entry-review-1` preserves the
publishable report verbatim. Review lease ended; no Cargo target was created for it.
Build and scratch paths are recorded in the private coordinator handoff; builds never share
a target concurrently. Current machine free space is near the build floor, so no full build
starts until adequate task-owned disposable space has been recovered.

## Language and implementation boundary

The narrow browser JavaScript exception was not approved and is not inferred from existing
graph-viewer code. Rust/WebAssembly is the selected baseline. Static help, option validation
and Markdown rendering are Rust. A disposable real-browser smoke is testing a pinned
tool-generated automatic module loader; source implementation follows only if that proof
succeeds. No authored JavaScript is committed, including snippets hidden inside Rust strings.
The optional exception question does not by itself block the permitted Rust path.

Reuse server-rendered search results and revision links. No store, provider, search-ranking,
MCP mutation, identity system or consumer-specific data belongs in this change.
An explicitly configured endpoint is documentation only, never permission to infer one from
an untrusted Host/forwarded header or start another server.

## Gate and completion

Require meaningful real-browser typing, stale-response, composition, focus, clearing and
no-script fallback checks; static help must work when the store is unavailable. Verify
escaping, URL validation, authority/method rejection and an exact CSP script policy if used.
The coordinator runs the complete repository gate, preserving each step's own exit and
reported skips, before integration and source-release claims. Focused static-guidance tests
have passed; the full gate and live-typing acceptance remain outstanding. Private deployment
and consumer acceptance remain separate.

## Historical static implementation stage

The static Rust half has been authored in seven files. The released baseline failed all
three new real HTTP/CLI probes, as expected: missing-store guidance, discoverable connection
link, and explicit endpoint options. The first candidate build was interrupted: compilation
began above the 10 GiB free-space floor, then unrelated disk activity reduced capacity; its
exact process group was terminated, exit 143, with no survivors. No test case executed in
that attempt. This is incomplete validation, not a product failure or a green result.

After capacity was recovered, the resumed search-page target passed 16 tests, zero failures,
zero ignores, exit 0. This includes configured metadata surviving a real store replacement
and guide tool names checked against the actual MCP tool list. The unchanged three-case
HTTP/CLI probe passed against the candidate, exit 0; this verifies the static-guidance claim
against its failing released baseline. Formatting and changed-file text scans passed.

Current candidate patch SHA-256: `53121522f000294f700560bf5b23305f7f5e7f0d9ea81d607ad8a67594769952`.
Candidate executable SHA-256: `ff550d7740bbb60e4174321a00fbd280721759d5310ded48288a46aa4e6d44de`.
The small report, baseline/treatment outputs and complete patch remain in task-owned scratch.
No JavaScript was authored. Rust/WASM packaging proof, browser implementation and review,
full gate, source release and managed cleanup remain outstanding. This is a partial review
checkpoint, not a completed story or release.

## Compatibility checkpoint

The broader static compatibility run passed agent_cli (29), docs_cli (18) and view_page
(36, with the existing ignored manual screenshot exporter). view_cli initially passed 17
and failed its exact option-list assertion because the two documented guidance flags were
new. The expectation now names both flags without weakening the assertion; its complete
target rerun passed all 18 cases. The independent adversary file was retained in the unit,
and its Cargo target passed all four cases. Package all-target Clippy exited zero with
warnings denied; formatting and changed-file privacy scans passed.

The private unit receipts preserve the initial failing run and corrected run separately:
compatibility.log, view-cli-corrected.log, adversary-integrated.log and clippy-package.log,
with individual process exits. These focused runs reused the existing small unit target;
no full workspace build was started while machine capacity was below the build floor.
This does not prove live-browser typing or the complete repository gate.

## Live candidate and review

Unit checkpoint `5ebde9dafdfde90cfad192c7abd16c38e670c49c` completes the Rust/WASM
implementation. The unit report `live/report.md` records focused browser/HTTP/CLI
checks, native-only installation, actual browser use of the independently installed
binary, reproducible raw bytes from separate source paths and a deliberate artifact-drift
refusal. These are scoped checks; the full repository gate has not run.

Coordinator `9b9535bc489a80e69634dc724c631d959774ff92` prepares the next source version
and changelog. It is unreleased and not integrated with the live implementation yet.

Review tree `335673a9dba2d92704a366ec531d4ef957fa8fd4` has exactly the unit candidate's
content while retaining its earlier static-review ancestry. A new review worker could
not start because the host thread limit was reached, so the coordinator performed the
tests-only adversary role in that separate checkout. Its tests-only bot commit
`3e98343b2ad0808d7ef5221924e575c8590509da` adds in-flight clearing/recovery and reserved
query-character cases. Both passed before the broader suite was selected.

The following combined suite failed in the candidate's no-script browser harness with
a stale document node during form navigation. `review-result:live-browser-review-1`
retains the observed failure and its scope: this is an introduced acceptance-harness
race, not evidence of a user-visible product failure. The implementor is correcting
document-lifecycle handling while preserving all assertions. Do not publish or release
until the correction and full gate are verified. Review scratch is assigned separately;
the idle unit target is reused sequentially, never by concurrent builds.

The corrected unit is `1203a043b252816ae094ba8c9bd602971a3980a8`. It retains the added
tests and waits for the new document loader's load event before DOM inspection. The
second review found no further issue in its bounded pass; all browser cases passed.
`review-result:live-browser-review-2` records the result and the separately retained
temporary-filesystem quota failure. The first finding is recorded as fixed.

The coordinator now combines that reviewed source with the prepared release version.
The complete gate must run on this combined tree before publication. Use the pinned
specification/planning tools, required browser and a synthetic PostgreSQL fixture; keep
fixture credentials and absolute execution paths only in private operational receipts.
The unit target is handed to this gate sequentially, with no concurrent unit/review build.
The default temporary filesystem is currently quota-limited; use the assigned short
private temporary directory and record any filesystem-sensitive tests explicitly.

## Integrated gate correction

The first integrated gate on `6b1b3e91ec9a7e476972e2d49047024af990d066` passed
formatting, raw browser-asset reproduction and workspace/all-target Clippy. Its
coordinator guard lane stopped at `public_surface` with exit 101: the internal
agent-help methods `guide` and `llms` have unrestricted public declarations without
direct external test references. The methods belong to a restricted internal type
and are used only by its sibling viewer module. Match their visibility to that
intended scope; retain the real HTTP output checks and the unchanged public guard.
The original failure is retained in private `release-gate/attempt-1` receipts.

The implementor owns the bounded correction in the existing unit tree. No other
gate process remains. After the scoped correction passes, integrate it and run the
complete gate on a newly frozen source. Remaining release-gate steps have not run;
the story and candidate remain unpublished and incomplete.

Correction `1a016ba0634e44a402aef3fd58285691c827d9aa` changes only the two methods
to `pub(super)`. The failed guard and existing HTTP guidance cases now pass; the
unit report records formatting, package Clippy and text scanning as green. Root
reviewed the exact diff, and the worker released its unit lease and idle target.
This correction is integrated here. Freeze this combined source for a fresh full
gate; no previous partial result is a completed release gate.

## Local gate and required CI discrepancy

The complete local Taskfile gate passed on frozen source
`24f7714475c363701c93b07bf6782cc5ce46d9bf`. The private `release-gate/final-report.md`
combines preserved same-source executions; it is not an uninterrupted-run claim.
Its ledger names every exit, the required real browser and TLS PostgreSQL inputs,
explicit ignored tests, and earlier capacity interruptions. No prerequisite skip
was reported. The story remains active because local success is not the release gate's
only requirement.

Draft pull request [69](https://github.com/beyond10x/epistemic-knowledge-runtime/pull/69)
was published after the integration's pre-PR guards passed, allowing required CI to run
alongside the remaining local gate. This supersedes the earlier blanket wait-before-publication
wording above; no merge or release was authorized by those partial results.
The required correctness job failed the existing sidebar-scroll case both initially
and in its one unchanged-source retry. Before restoration the offsets were `[120,40]`;
after restoration they were `[137,40]`. The local compact target passed, so the release
remains pending diagnosis rather than treating local success as a substitute.

`task:sidebar-scroll-ci-parity` records the exact failure, owns the focused diagnosis,
and inherits the newly recorded compact-test/graph-page scope from this story. Its
separate managed tree allows investigation without changing the frozen validation source.
Do not relax the assertion, change browsers merely to pass, or claim a speculative fix.
The required CI evidence is
[run 37122410085](https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/37122410085).

The completed original unit source is recoverable from the published integration branch.
Its separate review merge history was archived through the worktree CLI; both source
trees were retired through reviewed exact-id garbage collection after checking that
no active gate used them. The active external target and all small receipts remain.
The coordinator and focused diagnostic tree remain active.

Release documentation now describes live search, no-script submission and static agent
guidance without stale hardcoded status versions. These documentation and planning
updates follow the frozen runtime gate; their affected checks and final required CI
must pass before publication as a release.
