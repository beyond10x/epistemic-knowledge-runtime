---
format: aep.planning-md/3
id: review-result:adversary-extract-06-q-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit Q, pass 1
relations:
- reviews: story:fact-quality-by-judged-sample
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit Q (`impl/fact-quality-sample` at `2ae44eaf`; 9 cases committed as `00e08caa`, 2 red and ignored). The built `ekr` prints z, lower and upper values that differ from the library's: the CLI re-parses the library's bytes into `serde_json::Value`, and without `serde_json/float_roundtrip` (enabled only through a dev-dependency, so `cargo test` hides it) 10,303 of 119,988 values move to a neighbouring binary64. The documented example in `docs/cli.md` shows the library's values, which a built binary does not print.

Held: z within 4 ULP of 60-digit quantiles at 20 confidences including the AS 241 seam, monotone over 1–9999 bp; the Wilson interval at n=0, n=1, 1 bp, 9999 bp, n=10⁶; determinism across providers and revisions; no retracted or superseded assertion drawn; non-UTF-8 evidence as base64; size beyond the population; `JudgedTwice`; the empty document; evidence exposure no wider than `ekr explain` and the viewer.

```findings
- file: crates/ekr/src/cli/sample.rs
  line: 86
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "fact-quality prints serde_json::Value of the library bytes, and without float_roundtrip (every non-test build) 10303 of 119988 z, lower and upper values print as a neighbouring binary64, against views.yaml: every host answers the same binary64 values"
- file: docs/cli.md
  line: 713
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the documented fact-quality example is not what a built ekr prints; cargo test masks it because jsonschema enables float_roundtrip
```
