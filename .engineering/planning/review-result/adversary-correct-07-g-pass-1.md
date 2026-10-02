---
format: aep.planning-md/3
id: review-result:adversary-correct-07-g-pass-1
kind: review-result
status: active
title: Adversary, correct-07 unit G, pass 1
relations:
- reviews: task:divergence-is-a-typed-store-error
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit G (`impl/typed-divergence` at `1945bf5e`; cases committed as `3cb98d1b`, two red and ignored). `cargo test -p ekr-store`: 151 passed; targeted session, MCP and view lanes: 42 passed.

The reopen guard (`session.rs:303`, `:382`) fires after any failure while the held store is diverged, so a request that fails before reading the store, in a session holding an open proposal, prints `store-replaced-proposals-open` (exit 2) instead of its own failure (exit 1); at base the guard matched the failure's own text. The acceptance's source guard checks one needle, so `message.contains("diverged")` passes it.

Held: no non-divergence provider message can contain the divergence sentence (every `Backend` site at `fe8a0a7` checked); no divergence slips through as `Backend`; one retry, no loop; the original failure is returned when `head()` does not answer `Diverged`; MCP and view handle one request at a time; MCP and view printed text unchanged.

```findings
- file: crates/ekr/src/cli/session.rs
  line: 303
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the reopen guard fires after any failure while the held store is diverged, so a request failing before any store read in a session with an open proposal prints store-replaced-proposals-open (exit 2) instead of its own failure (exit 1)"
- file: crates/ekr/src/cli/session/tests.rs
  line: 93
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the acceptance's source guard checks one needle, so a CLI matching the divergence message by contains(\"diverged\") passes it"
```
