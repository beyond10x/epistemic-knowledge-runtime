---
format: aep.planning-md/2
id: task:ekr-store-block-on-cannot-nest
kind: task
status: implemented
title: The synchronous store panics rather than errors when called from inside a runtime
relations:
- serves: vision:o2
- derived_from: story:synchronous-runtime-refusal
revision: 4
---
## The cost this records

`architecture-decision-record:0006-ekr-store-bridges-the-async-port` puts a `tokio` current-thread
runtime inside `ekr-store` and exposes a synchronous `RevisionLog`, so that `ekr-kernel`, the CLI
and everything P1 builds later stay synchronous.

`tokio::runtime::Runtime::block_on` **panics** when it is called from a thread that is already
inside a tokio runtime: "Cannot start a runtime from within a runtime". So the first async consumer
of `ekr-store` meets this as a panic at run time rather than as a type error at compile time.

## Why it is not a problem yet

P1's only consumer of `ekr-store` is `story:ekr-cli`, a command-line program with no runtime of its
own. `ekr-kernel` sits between them and is synchronous by the same decision. Nothing in the P1
roster calls the store from inside a runtime.

## What would make it one

An HTTP or MCP surface over this runtime, an agent host that is already async, or anything that
embeds `ekr-store` in a server. `docs/roadmap.md` puts `ekr-mcp` in a later phase, and an MCP server
is exactly the shape that would hit this.

## What closes this

One of:

- `RevisionLog` gains async methods and the synchronous ones become the wrapper rather than the
  other way round. This is a mechanical widening of one trait, and ADR 0006 says it is not
  foreclosed.
- or `ekr-store` detects an ambient runtime and refuses with a named error rather than panicking, so
  the failure is legible. `tokio::runtime::Handle::try_current` is the check.

The second is cheap and can be done at any time; it converts a panic into an error a reader can act
on without deciding the larger question.
