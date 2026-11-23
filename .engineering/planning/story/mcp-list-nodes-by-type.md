---
format: aep.planning-md/3
id: story:mcp-list-nodes-by-type
kind: story
status: active
title: ekr mcp lists every node of a type page by page, and search can be continued
summary: Consumer gap of the runbook refresh and finding 29 of the EKR 0.0.42 review
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:14:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:14:17Z", actor: "human:timo", revision: 3}
---
## Outcome

An agent reading a store through `ekr mcp` can list every node of a type, page by page, and a
search that matched more than one page can be continued.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`), 2026-11-23, a
three-round replay into one store: `ekr mcp` has no tool that lists all nodes of a type; `search`
stops at 100 results, and `timeline` returned 3 of 14 nodes of a type; the caller read
`ekr snapshot` instead.

Finding 29 of the external review of EKR 0.0.42 (2026-11-23): for 101 equal names, search reported
101, returned 100 and had no continuation.

- An MCP tool that lists the nodes of a named type at a revision (or head), with a page size, a
  stable order and a continuation token; it names how many nodes there are.
- `search` returns a continuation token when it has more matches than it returned, and takes one.

## Acceptance

With a store of 250 nodes of one type, paging the new tool returns each node once and the total
250; a search with 101 equal names returns 100 and a token, and the token returns the 101st.
