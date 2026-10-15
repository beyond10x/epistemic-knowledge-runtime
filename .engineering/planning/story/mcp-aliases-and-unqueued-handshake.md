---
format: aep.planning-md/3
id: story:mcp-aliases-and-unqueued-handshake
kind: story
status: active
title: The MCP takes common argument aliases, and its handshake never queues behind a tool call
summary: 'Argument aliases and a handshake answered off the store thread (ekr-mcp-fixes-20261015 #8, #12).'
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
- depends_on: story:bounded-node-detail
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr/src/cli/http.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T13:21:17Z", actor: "agent:ekr-controller-planner", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T13:21:17Z", actor: "agent:ekr-controller-planner", revision: 4}
---
## Context

A consumer's measured patch (ekr-mcp-fixes-20261015, items 8 and 12):

- #8: agents pass `id` for `describe_node`'s `node`, `edge` for `edge_type`, one string for
  `expand`'s `seeds`, and `id` for `explain`'s `assertion`, and are answered -32602. 2 of 28 agent
  trials were hurt.
- #12: with 5 sessions in parallel, 11 of 12 failed to connect within a 30 s connect timeout. From
  the code (not observed in the trial): `ekr mcp-http` answers every request on one handler thread,
  behind a 16-deep queue (`QUEUE_LIMIT`, `crates/ekr/src/cli/http.rs`) with a 30 s wait, so
  `initialize` and `tools/list` wait behind other sessions' tool calls, which on a cold host include a
  20-33 s store open.

Neither changes an answer document: an alias names an input the tool already takes, and the
handshake is answered with the bytes it is answered with today.

## Build (spec first)

1. **The spec first**:
   - `systems/ekr/domains/views.yaml`, the paragraph that binds the commands to the MCP tools (it
     names the MCP's -32602 today): the argument aliases (#8) — `node` also as `id` and `node_id`;
     `edge_type` also as `edge`; `seeds` also as `seed`, `node` and `nodes`, as one id or a list;
     `assertion` also as `id` and `assertion_id`; timeline's `subject` also as `node` — and that an
     alias changes no answer. Two names for one input in one request are refused as -32602.
   - The handshake rule (#12) is transport behaviour that no domain under `systems/ekr/` declares
     (none declares `ekr mcp` or `ekr mcp-http`). This story does not invent a domain for it: its
     contract is the module documentation of `crates/ekr/src/cli/mcp.rs` and `http.rs` and
     `docs/cli.md`, and the story says so where it lands. Whether the MCP transport gets an ESS
     domain is a separate question for the planner.
2. `crates/ekr/src/cli/mcp.rs`: the serde aliases, `seeds` as one id or a list, the input schemas
   naming the aliases.
3. `crates/ekr/src/cli/http.rs` and `mcp.rs` (#12): the connection thread answers every `/mcp`
   message that is not `tools/call` (initialize, ping, tools/list, notifications, malformed lines)
   and every transport refusal itself, before the queue; only `tools/call` reaches the store's
   thread. The store-free path opens no store.
4. `docs/cli.md`, `CHANGELOG.md`.

## Acceptance

Each case names "ekr-mcp-fixes-20261015 #8" or "#12". No case passes or fails on wall-clock time.

- `crates/ekr/tests/mcp.rs`, `an_aliased_argument_answers_what_its_input_answers` (#8):
  `describe_node {"id": …}`, `describe_node {"node_id": …}`, `expand {"seeds": "<one id>"}`,
  `expand {"seed": …}`, `explain {"id": …}` and `timeline {"node": …}` each answer the same bytes as
  the call with the input's own name; on 0.0.34 each is -32602. A call naming both `node` and `id`
  is -32602.
- `cli::mcp::tests::a_handshake_is_answered_while_a_tool_call_holds_the_store_thread` (#12): a
  `tools/call` is held on the store's thread by a barrier the test owns; while it is held,
  `initialize` and `tools/list` from a second connection are answered (with today's bytes); the held
  call is then released and answered. The case asserts the order of answers, not a duration (the
  patch's version asserts "under 1 s while a 3 s call is held", which this repository's rule against
  wall-clock tests refuses).
- `crates/ekr/tests/mcp.rs` `initialize_then_tools_list_names_exactly_the_nine_read_tools_with_input_schemas`
  passes with the aliases in the schemas, and the adversary protocol suite
  (`crates/ekr/tests/adversary_mcp_protocol.rs`) passes unchanged.

## Scope

- `systems/ekr/domains/views.yaml`
- `crates/ekr/src/cli/mcp.rs`, `crates/ekr/src/cli/http.rs`
- `crates/ekr/tests/mcp.rs`
- `docs/cli.md`, `CHANGELOG.md`

## Overlaps and order

Shares `views.yaml`, `mcp.rs`, `crates/ekr/tests/mcp.rs`, `docs/cli.md` and `CHANGELOG.md` with every
story of the wave, and `http.rs` with `ekr view`'s serving path. Lands last: `edge_type` is
`story:bounded-node-detail`'s input, and the aliases cover inputs the earlier stories add.

## Out of scope

OAuth refresh in front of the server; the cold store open itself.
