---
format: aep.planning-md/3
id: story:read-surface-specified
kind: story
status: draft
title: One read surface is specified for MCP and HTTP, and its OpenAPI is generated
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
- depends_on: story:explain-bounds-documents
revision: 2
---
## Context

`ekr view` and `ekr mcp-http` serve the same reads under different names and with different coverage (tag
0.0.30):

| read | `ekr view` | `ekr mcp-http` tool |
|---|---|---|
| search | `GET /search?q=` | `search{text}` |
| changes | `GET /changes` | `changes_since` |
| one node | `GET /node/{id}` | `describe_node` |
| evidence, projection, roles | `GET /evidence/{id}`, `/projection`, `/roles` | none |
| explain, resolve | none | `explain`, `resolve` |

Sources: `crates/ekr/src/cli/view.rs:706-719` (routes), `:1265` (`q`); `crates/ekr/src/cli/mcp.rs:433-441`
(tools). In `systems/ekr`, `ekr-views` accepts only `ekr.views.ProjectGraph` and no component declares
`reached_by`, so `ess generate --kind openapi` publishes no read surface.

Probed with ESS 0.52.0 on a scratch copy of `systems/ekr` at 0.0.30 (2026-10-04):

1. A component accepting a command of a domain another component owns is refused, `ESS-COMPONENT-004`
   (`conflicting_declaration`), so `ekr.kernel.Explain` cannot sit in the `ekr-views` document as it stands.
2. `reached_by: network` on `ekr-views`, accepting `ProjectGraph` and its six bounded reads, validates and generates
   `POST /views/commands/<wire>` per command.
3. Those commands' inputs are the CLI contract: `store` (`ekr.views.StoreLocation`) is required and the
   revision is `at`, where the MCP tools take `revision` and no store.
4. The success body carries `outcome` and `published` only; the command's `response:` document is absent,
   also with `returns: true` at `format: ess/17` (the specification validates unchanged at `ess/17`).
   beyond10x/ess#423. Every success projects `202`: beyond10x/ess#424, accepted until fixed.

## Build

- The network read surface in `systems/ekr`: 11 operations, each with `naming.wire` equal to its MCP tool
  name: `head`, `overview`, `search`, `describe_node`, `expand`, `timeline`, `changes_since`, `explain`,
  `resolve`, `describe_evidence`, `projection`.
- New commands where none exists: a head read, `resolve` (today an `ekr resolve` verb and the
  `ekr.integrate.ResolvedReference` type only), and one evidence record. `explain` is modelled inside
  `ekr.views` as a read over the kernel's explain chain, or through a binding if ESS offers one by then;
  record which in the domain.
- Network inputs carry no store location; the revision input is named the same on every surface.
- `reached_by: network` on the component that serves them; the success outcomes return their document.
- The generated OpenAPI is committed under `systems/ekr/` and the gate regenerates and compares it. The release
  attaches it as an asset.

## Acceptance

- `ess specify validate --path systems/ekr` is valid and `ess generate --kind openapi` twice gives identical
  bytes.
- The generated document has one `POST /views/commands/<wire>` per operation above, no `store` in any request
  body, and each success response carries the operation's `ekr.*` document.
- A test compares `tools/list` names and input schemas with the commands' inputs in
  `ess specify compile --format json`.

## Added 2026-10-06 (plan-critic round 2)

`explain`'s input carries `offset` and `limit` (bytes), which `story:explain-bounds-documents` adds before this story lands; this story models them with the rest of the input. `describe_type` (`story:describe-type-read`) joins the surface after this story as a twelfth operation.
