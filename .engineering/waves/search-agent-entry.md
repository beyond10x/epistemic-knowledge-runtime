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

## Current stage

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
