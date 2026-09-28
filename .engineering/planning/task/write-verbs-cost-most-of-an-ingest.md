---
format: aep.planning-md/3
id: task:write-verbs-cost-most-of-an-ingest
kind: task
status: draft
title: A write verb costs about 0.6 s inside ekr session
relations:
- serves: vision:o5
- derived_from: story:ekr-session
revision: 1
---
## What is wrong

A write verb costs about 0.6 s inside `ekr session` and 0.8 s one-shot on small stores (1.4 MB to
21 MB), measured by the implementor of `story:ekr-session` over five propose/validate/commit trios.
A resolve in the same session costs 4.4–18.2 ms, so writes dominate an ingest that creates nodes
one batch at a time: a consumer rebuild with 4,127 new references reports the write path as its
largest cost after 0.0.12.

Where the time goes has not been measured.

## What closes this

A profile of `propose`, `validate` and `commit` inside one session names the cost, and a change
brings each under 100 ms on the 19 MB store of that measurement, measured the same way, with every
existing refusal unchanged.
