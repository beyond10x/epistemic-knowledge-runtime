# Wave 2026-10-15a — bounded, named, forgiving MCP reads

Skill: aep:implementing (wave mode). Coordinator: the repository's controlling Claude session of
2026-10-15.
Status: **integrated**: U1–U6 merged into `wave/20261015a` in sequence; the six stories implemented on their package gates; the release that fixes a stage begin (0.0.35) merged in; released as 0.0.36 after the full gate.

Source: a consumer's measured patch for the MCP tools, against 0.0.34, cited by item as
"ekr-mcp-fixes-20261015 #N" (13 items, each with a measurement, a change and a test). It is a
proposal; this repository owns the formats. Out of scope: fuzzy search, merging duplicate entities,
and the patch's open items (cold `changes_since` over hundreds of revisions, cold store open, search
over property values).

Decided by the operator before planning:

- Every MCP answer stays byte for byte what `ekr view` (or the session verb, `ekr explain`,
  `ekr resolve`) answers for the same request.
- Each bounded or named answer is a NEW versioned format, declared first in the spec
  (`systems/ekr/domains/views.yaml`, or `kernel.yaml` / `integrate.yaml` for explain and resolve),
  served by both the MCP tool and the verb. Old versions stay served.
- Where the MCP uses a smaller default (#4), the identity holds against `ekr view` given that same
  explicit value, and the spec says so.
- The 9 MCP tests in `crates/ekr/tests/mcp.rs` and the 4 golden-byte tests
  (`crates/ekr-views/tests/query.rs` x3, `lineage.rs` x1) move to the new formats; none is deleted.

Base: `main` at `c12847649` (release 0.0.34). Integration branch `wave/20261015a`. One pull request
from it.

Width: 1 tree, units run in sequence: / has 23G free and the last wave tree peaked at 41G, so a second tree's target/ does not fit.

## Units

In landing order. Every unit is spec first: its spec change is green under
`ess specify validate --path systems/ekr` and the conformance suites it drives before its code.

| unit | story | items | serves | main surface | spec change | dispatch |
|---|---|---|---|---|---|---|
| U1 | `story:bounded-node-detail` | #1, #10, #11 | o5 | `crates/ekr-views/src` lib/index/query + new detail module; `crates/ekr/src/cli` mcp/view/session/views | `views.yaml`: `ekr.node-detail/2`, a refusal naming a type | `aep:implementor`, `aep:adversary` |
| U2 | `story:search-words-type-and-time` | #2, #6, #9 | o5 | `crates/ekr-views/src` lib/index/query; `crates/ekr/src/cli` mcp/view/session/views | `views.yaml`: `ekr.node-matches/2`, tier `Words`, `type`/`from`/`to` | `aep:implementor`, `aep:adversary` |
| U3 | `story:names-inline-in-read-answers` | #3, #13 | o5 | `crates/ekr-views/src` lib/index/query/timeline/changes; `crates/ekr/src/cli` mcp/view/session/views/explain/resolve | `views.yaml`: `ekr.graph-slice/2`, `ekr.graph-overview/3`, `ekr.graph-timeline/2`, `ekr.graph-changes/2`; `kernel.yaml`: `ekr.explanation/3`; `integrate.yaml`: a named resolve format | `aep:implementor` |
| U4 | `story:changes-since-small-default-and-cached` | #4, #5 | o5 | `crates/ekr-views/src` changes/index/lib; `crates/ekr/src/cli/mcp.rs` | `views.yaml`: per-surface defaults, ChangesSince cost clause | `aep:implementor` |
| U5 | `story:explain-scoped-to-cited-evidence` | #7 | o5 | `crates/ekr/src/cli/explain.rs`; `crates/ekr-kernel/src/explain.rs`; `mcp.rs` | `kernel.yaml`: `ekr.explanation/3` scope, `omitted_evidence`, 48 KiB cap | `aep:implementor`, `aep:adversary` |
| U6 | `story:mcp-aliases-and-unqueued-handshake` | #8, #12 | o5 | `crates/ekr/src/cli/mcp.rs`, `http.rs` | `views.yaml`: MCP argument aliases; the handshake rule has no ESS home (no domain declares the MCP transport) and is stated in the module docs and `docs/cli.md` | `aep:implementor`, `aep:security-reviewer` |

Scopes are inferred (`aep plan artifact scope … --inferred`); none is cited by a scoper yet.

## Overlaps, and why the units run in this order

Every unit lands on `crates/ekr/src/cli/mcp.rs`, `crates/ekr/tests/mcp.rs`, `docs/cli.md` and
`CHANGELOG.md`; U1–U4 and U6 on `systems/ekr/domains/views.yaml`; U1–U4 on
`crates/ekr-views/src/{lib,index}.rs`; U1–U3 on `crates/ekr-views/src/query.rs`,
`crates/ekr-views/tests/query.rs`, `crates/ekr/src/cli/view.rs` and `crates/ekr/src/cli/session/views.rs`;
U3 and U4 on `crates/ekr-views/src/changes.rs`; U3 and U5 on `kernel.yaml` and
`crates/ekr/src/cli/explain.rs`. `aep plan artifact waves` puts each in a wave of its own.

- U1 before U2: U1's type-id refusal points at `search` with `type`, which U2 adds.
- U3 after U1 and U2: they name their own new formats (`type_name` in the detail and the matches);
  U3 names the rest.
- U4 after U3: U3 moves `changes_since` to `ekr.graph-changes/2` and moves the two changes tests
  there; U4 then moves their limit.
- U5 after U3: U3 declares `ekr.explanation/3` with `names`; U5 extends that same /3 with the scope,
  `omitted_evidence` and the cap before the wave's release, so no released /3 lacks them.
- U6 last: its aliases cover inputs U1 and U2 add (`edge_type`; `query`/`q` are U2's own).

Conditions:

- The per-surface default for each new format: the MCP answers the new version; `ekr view`, the
  session verb, `ekr explain` and `ekr resolve` keep their current default and answer the new one
  when asked. Every moved MCP test compares against the verb with the format (and, for U4, the limit)
  named explicitly, and a kept case holds each old version's bytes.
- No case passes or fails on wall-clock time. The patch's handshake test (#12) asserts "under 1 s
  while a 3 s call is held"; U6 restates it as an ordering held by a barrier the test owns. U4's
  cache (#5) is counted in transaction-document parses, not timed.
- The SDK's typed reads (`crates/ekr-sdk/src/read/views.rs`) keep reading the old versions; typing
  the new ones is not this wave.
- After each unit lands, `crates/ekr/tests/docs_cli.rs` runs, since every unit edits `docs/cli.md`.

Near this wave, not in it:

- `story:read-answers-name-types` (draft) asked for names on five reads; U1, U2 and U3 cover it.
  Whether it is archived or superseded is a planning decision after this wave.
- `story:describe-type-read` (draft): U1 only refuses a type id by name; the read that answers a type
  id with its declaration stays that story's.
- `story:changes-since-by-time-within-deadline` (active, carried from wave 2026-10-14a) shares
  `crates/ekr-views/src/changes.rs` with U3 and U4.

## Trees

| unit | branch | worktree id | build dir | scratch | stage |
|---|---|---|---|---|---|
| all, in sequence | `wave/20261015a` | `ekr-w20261015a` | `<tree>/target` | `~/.cache/epistemic-knowledge-runtime-w20261015a` | integrated; finished after the merge |

## Gate

Not run. Each unit runs `cargo check -p` and `cargo test -p` on its crates; one full `task check`
on the combined head, then the cross-crate checks of `COORDINATOR.md` check 9, before the pull
request.
