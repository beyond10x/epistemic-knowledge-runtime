---
format: aep.planning-md/1
id: story:agent-discoverable-cli
kind: story
status: implemented
title: The ekr binary is usable and discoverable by an agent through the CLI alone
relations:
- decomposes: epic:p4-operator-surface
- serves: vision:o5
scope:
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/ekr/src/cli
- confidence: inferred
  path: crates/ekr/tests
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
revision: 9
---
## Context

Operator goal (2026-09-23): the `ekr` binary is usable by an AI agent such as Claude Code through
the CLI alone, with the manipulation operations exposed in a discoverable way.

Today (`ekr --help` on `wave/wave-p1-14` @ `16fd4e1`) the binary has six verbs — seed, propose,
validate, commit, snapshot, explain — and three required global flags on every call. An agent
can see the verbs but not what to write: `propose` takes an `ekr.transaction-document/1` YAML
document whose twelve operation kinds (`GraphOperation`, `crates/ekr-kernel/src/transaction.rs`),
YAML tags, id fields and time fields are documented nowhere the binary prints. Ids must be minted
by the caller, `validate --against` needs the head revision number, which no verb prints on its
own, and type and predicate ids must be read out of a whole-graph snapshot.

## Acceptance

An agent with only the binary and `ekr --help` can seed a store, add a node, an edge and an
assertion, retract and supersede an assertion, and read the result back, without reading source
or fixtures. Concretely:

1. `ekr guide` prints the workflow (roles, propose → validate → commit, exit codes 0/1/2, where
   ids and revision numbers come from) as text.
2. `ekr operations` lists the twelve operation kinds with one line each; `ekr operations <Kind>`
   prints its fields and a complete example operation. Every printed example parses through the
   real `ekr.transaction-document/1` reader (a test holds every kind).
3. `ekr example <format>` prints a complete document for `ekr.transaction-document/1`,
   `ekr-seed/2` and `ekr.cli-host/1`; each printed document is accepted by the real reader, and
   the seed and host examples seed a fresh store on both providers.
4. `ekr mint <kind>` prints a fresh id of a named id kind (node, edge, assertion, transaction,
   evidence, …).
5. `ekr head` prints the head revision number and root as JSON; `validate --against` defaults to
   the head when omitted.
6. `ekr transactions [--state <State>]` lists retained transactions with id, state and proposer.
7. `ekr ontology` prints node types, edge types and properties by name and id at the head.
8. `--host`, `--store` and `--backend` also read `EKR_HOST`, `EKR_STORE`, `EKR_BACKEND`.
9. Every store verb and `mint` writes JSON to stdout on success (`guide`, `operations` and `example` print text and documents) and names its refusal on stderr; `--help` of every
   verb names its input format and points at `ekr example`/`ekr operations`.
10. An end-to-end test drives the retraction example through fresh processes using only strings
    the binary printed (examples, minted ids, head numbers), on both providers.

## Out of scope

MCP tools (`ekr-mcp`, P4), attention pages, rendered views, a query language. No kernel semantics
change: every new verb reads through the existing `Runtime` or prints static, tested text.
