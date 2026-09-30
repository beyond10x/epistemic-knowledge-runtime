---
format: aep.planning-md/3
id: task:session-request-cost-grows-with-evidence
kind: task
status: archived
title: An ekr session request costs 20-34 ms on a store with 11 MB of evidence
relations:
- serves: vision:o5
- derived_from: story:ekr-session
revision: 2
transitions:
- {from: "draft", to: "archived", at: "2026-09-30T12:21:07Z", actor: "human:timo", revision: 2}
---
## What is wrong

Measured at the close of wave ingest-01 (release build of `wave/ingest-01` at `7d4c8d90`), on
fresh stores seeded from the 11 MB evidence seed of `epic:ingestion-throughput` (112 MB on disk,
one revision), resolving one typed reference:

| provider | 0.0.11 one-shot | ingest-01 one-shot | ingest-01 `ekr session`, 1,000 requests |
|---|---|---|---|
| file | 2.58 s | 1.47 s | 33.9 s (33.9 ms each, 1,000 exit 0) |
| SQLite | 1.72 s | 0.77 s | 19.8 s (19.8 ms each, 1,000 exit 0) |

The wave plan's target was 1,000 session resolves in under 5 s. A session request still costs
20–34 ms on this store, while the same request on a 1.4 MB store costs 4.4 ms
(`story:ekr-session` commit message), so the cost grows with the retained evidence.

Hypothesis, not verified: the verified read behind `resolve` clones the seed input, including every
evidence payload, into each `VerifiedRead` (`crates/ekr-kernel/src/read.rs`, `seed_input`), and
re-derives the head graph per request.

## What closes this

A profile of a session serving resolve requests on this store names where the time goes, and a
change brings 1,000 session resolves under 5 s on both providers, measured the same way.
