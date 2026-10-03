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
Unit: assigned after the opening record is committed; it must use its own managed tree.
Build and scratch paths are recorded in the private coordinator handoff; builds never share
a target concurrently. Current machine free space is near the build floor, so no full build
starts until adequate task-owned disposable space has been recovered.

## Language and implementation boundary

The narrow browser JavaScript exception is awaiting an operator decision. It is not approved
by existing graph-viewer code. Static help, option validation and Markdown rendering remain
Rust and can proceed independently. No custom script is committed until that question is
settled; Rust/WebAssembly remains the baseline language requirement.

Reuse server-rendered search results and revision links. No store, provider, search-ranking,
MCP mutation, identity system or consumer-specific data belongs in this change.
An explicitly configured endpoint is documentation only, never permission to infer one from
an untrusted Host/forwarded header or start another server.

## Gate and completion

Require meaningful real-browser typing, stale-response, composition, focus, clearing and
no-script fallback checks; static help must work when the store is unavailable. Verify
escaping, URL validation, authority/method rejection and an exact CSP script policy if used.
The coordinator runs the complete repository gate, preserving each step's own exit and
reported skips, before integration and source-release claims. No tests have run for this
new story yet. Private deployment and consumer acceptance remain separate.

## Current stage

Scoped and accepted for implementation. Static Rust guidance may start; browser-language
decision, implementation, adversarial review, full gate, source release and managed cleanup
remain outstanding.
