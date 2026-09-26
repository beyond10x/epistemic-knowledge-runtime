---
format: aep.planning-md/2
id: task:store-reads-rehash-the-whole-log
kind: task
status: implemented
title: Every store read re-hashes the whole file-provider log, so commands grow quadratically
relations:
- serves: vision:o2
- derived_from: epic:p6-maintenance-observability
revision: 4
---
## What is wrong

Measured on 2026-09-24 by a performance review of the release `ekr` binary, one commit per revision (`home-path:sha256:f5de7990e9d5939ff7e52fe511dcc8b7085e9668f8a6d78f4c021e9cb8bc932a`, single runs, ±30%):

| backend | revision | propose ms | commit ms | head ms |
|---|---|---|---|---|
| file | 1 | 344 | 319 | 41 |
| file | 40 | 2487 | 2655 | 618 |
| sqlite | 40 | 184 | 205 | 29 |

On the file provider the cost is quadratic in revisions. `strace` of one `ekr head` at revision 40 (a 627 KB `events.jsonl`): the log is opened 257 times and 161 MB is read from it. `perf`: about 80% of samples are in software SHA-256. At revision 41, a propose reads the log 1007 times (616 MB).

## Mechanism

- `eventlog-file` at the pinned rev `28e57856`: every store call is one `FileEventStore::transaction`, which runs `Journal::resume` and re-reads and hashes the whole committed prefix before serving anything (`crates/eventlog-file/src/journal.rs`, counter `prefix_bytes_hashed`). `read_stream` and `get_blob` are one transaction each.
- `crates/ekr-store/src/eventlog.rs` `EventlogStore::load_history` (:386) reads each required object with its own `read_stream`, and each blob with its own `get`. Calls per command grow with the number of objects, so bytes read grow with revisions times log size.

The same work makes the test suite slow: the six slowest test binaries (1,141 of 1,402 s in the last gate) are store-heavy.

## What closes this

Either ekr-store reads a command's whole history in one provider transaction (upstream eventlog needs a multi-read entry), or ekr-store keeps the verified fold across calls within one open store. Then per-command cost on the file provider grows linearly, measured at 3 revision counts on both providers.
