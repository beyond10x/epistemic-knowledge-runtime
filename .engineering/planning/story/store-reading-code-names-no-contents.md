---
format: aep.planning-md/3
id: story:store-reading-code-names-no-contents
kind: story
status: draft
title: A check reports store contents named in a consumer's code
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
revision: 1
---
## Context

A consumer instance keeps a 246-line check that its store-reading code names none of the
store's contents, so code stays generic over any ontology (reported 2026-09-29). The runtime has
the same rule for its own viewer (the viewer's data-free rule, crates/ekr/tests/view_page.rs), but
nothing a consumer can run against its own code.

## Build

A verb that takes a store and a set of source files and reports every literal in the files that
equals a type name, property name, alias or canonical name in the store, with file and line;
exit 1 when any is found. Reads the store only.

## Acceptance

- On a fixture store and fixture sources, every planted name is reported with its file and line,
  and none of the store's ids or the runtime's own vocabulary is reported.
- The verb writes nothing.
