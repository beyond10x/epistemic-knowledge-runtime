---
format: aep.planning-md/3
id: review-result:adversary-extract-06-p-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit P, pass 1
relations:
- reviews: task:protocol-error-keeps-the-answer
revision: 1
---
CONFIRMED

Adversary pass 1 on unit P (`impl/protocol-error-answer` at `53cb1cb8`; tests committed as `762f4ad1`). Suite: `cargo test -p ekr-sdk` 160 passed, 6 ignored (4 of them this pass's red cases).

| # | where | severity | finding |
|---|---|---|---|
| 1 | `crates/ekr-sdk/src/session.rs:623` | warning | a reply that is JSON but not a `WireReply` (a newer ekr adding a field, under `deny_unknown_fields`) becomes the answer, so request content a usage error quotes reaches the `Protocol` message and every later `Latched` cause |
| 2 | `crates/ekr-sdk/src/transport.rs:112` | note | the message Debug-escapes the answer, so a line with `"` or `\` is not contained in the message as printed |
| 3 | `crates/ekr-sdk/src/transport.rs:180` | note | lossy decoding before measuring triples non-UTF-8 bytes, so a 134–400 byte answer is marked cut |
| 4 | `crates/ekr-sdk/src/transport.rs:181` | note | every trailing CR and LF is dropped, where the docs promise only the final line end |

Held: the cut at every character width and offset, empty and newline-only answers, CRLF, NUL, all three construction sites, existing message matchers.

```findings
- file: crates/ekr-sdk/src/session.rs
  line: 623
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a reply line that is JSON but not a WireReply becomes the answer, putting request content quoted by a usage error into the Protocol message and every later Latched cause"
- file: crates/ekr-sdk/src/transport.rs
  line: 112
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the message Debug-escapes the answer, so a line containing a double quote or backslash is not contained in the message as printed
- file: crates/ekr-sdk/src/transport.rs
  line: 180
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "lossy decoding before measuring expands non-UTF-8 bytes threefold, so a 134-400 byte answer is marked cut contrary to the docs"
- file: crates/ekr-sdk/src/transport.rs
  line: 181
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: every trailing CR and LF is dropped, while the field documentation promises only the final line end is
```
