---
format: aep.planning-md/3
id: review-result:adversary-read-01-m-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit M: ekr mcp'
relations:
- reviews: story:mcp-read-tools
revision: 1
---
unit: story:mcp-read-tools (wave read-01, unit M), commit 6f8ba2a6 plus 2 untracked adversary test files in ekr-read-m
verdict: NEEDS-CHANGE
cases: executed 389→393, red 4
origin: introduced 4 / pre-existing 0 / undecided 0

## Cases added (all red at 6f8ba2a6)

| test | red output |
|---|---|
| `adversary_mcp_resolve_of_an_alias_holding_nel_answers_what_the_verb_prints` | the tool resolved node `…a912`, `ekr resolve` `…a911` |
| `adversary_mcp_resolve_of_an_alias_holding_del_answers_what_the_verb_prints` | `-32602 … control characters are not allowed at position 20` |
| `adversary_mcp_an_integer_id_beyond_64_bits_is_echoed_unchanged` | `{"id":1e+20,…}` |
| `adversary_mcp_a_batch_is_received_under_the_2025_03_26_revision_it_agreed_to` | a batch answered with -32600 |

## What held

Framing at and over the 6,356,992-byte cap, invalid UTF-8, deep nesting, a final line without a
newline; ids of every JSON type; notifications never answered; bounds (`i64::MAX` limit is
LimitExceeded, `u64::MAX` revision is RevisionNotFound); record text holding protocol-looking lines
stays escaped; explain after another process's commit equals `ekr explain`; no call path reaches
propose, validate, commit or seed, and the file store is unchanged byte for byte.

```findings
- file: crates/ekr/src/cli/mcp.rs
  line: 438
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: resolve re-serializes arguments with raw U+0085 which the YAML reader folds to a space, so the tool resolves a different node than ekr resolve for the same reference
- file: crates/ekr/src/cli/mcp.rs
  line: 438
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: resolve passes DEL and C1 controls raw to libyaml, which refuses them with -32602 while ekr resolve resolves the same escaped alias
- file: crates/ekr/src/cli/mcp.rs
  line: 242
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an integer request id beyond 64 bits is echoed as a rounded float (1e+20), not the id the client sent
- file: crates/ekr/src/cli/mcp.rs
  line: 64
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the server agrees to protocol 2025-03-26, which requires receiving JSON-RPC batches, and answers a batch with -32600
- file: crates/ekr/src/cli/mcp.rs
  line: 220
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a blank line is silently ignored while docs/cli.md says a line that is not JSON gets -32700
```
