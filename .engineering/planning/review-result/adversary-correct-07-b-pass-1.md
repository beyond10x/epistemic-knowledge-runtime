---
format: aep.planning-md/3
id: review-result:adversary-correct-07-b-pass-1
kind: review-result
status: active
title: Adversary, correct-07 unit B, pass 1
relations:
- reviews: task:seed-document-bounds-alias-expansion
revision: 1
---
CONFIRMED

Adversary pass 1 on unit B (`impl/document-bounds` at `cb4961e9`; cases committed as `538a9a05`, one red and ignored). `cargo test -p ekr-kernel`: 518 passed, 5 ignored; targeted `-p ekr`: 5 passed.

Held: nested aliases, merge keys, scalar and key aliases and redefined anchors are charged; limits are inclusive and exact for nodes, text and depth in block, flow and alias form; recursive aliases stay `seed-decode`; later documents are bounded; file and stdin read at most the cap plus one byte; the transaction reader bounds before any full load; the repository's seed fixtures load.

```findings
- file: crates/ekr-kernel/src/yaml.rs
  line: 105
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "a seed under the byte cap with no alias is refused as seed-alias-expansion because the escapes \\L and \\P decode 2 input bytes to 3 text bytes, while docs/cli.md says the limits are what a written-out document of the cap could hold"
```
