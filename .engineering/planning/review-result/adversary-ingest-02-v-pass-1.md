---
format: aep.planning-md/3
id: review-result:adversary-ingest-02-v-pass-1
kind: review-result
status: active
title: Adversary, wave ingest-02 unit V, pass 1
relations:
- reviews: story:seed-envelope-v3-references-payloads
revision: 1
---
## Verdict

NEEDS-CHANGE. Three defects in `ekr migrate`; the seed envelope `/3` held under every attack.

```
unit: wave ingest-02 unit V, seed-envelope-v3 + ekr migrate, ekr-i2-v at 1e13b0a4 plus 3 added test files
verdict: NEEDS-CHANGE
cases: executed 383→392 (ekr-kernel), 132→132 (ekr-store); red 3
origin: introduced 3 / pre-existing 0 / undecided 0
```

The run hit the `/dev/shm` per-user quota: the full `ekr` package suite did not compile there, and 13
ekr-kernel test binaries crashed with SIGBUS until copied to disk, where their 169 cases passed.

## Cases added

`crates/ekr-kernel/tests/adversary_seed_envelope_v3.rs` (7 cases, 1 red),
`crates/ekr-kernel/tests/adversary_seed_envelope_v3_elected.rs` (2, green),
`crates/ekr/tests/adversary_migrate_v3_cli.rs` (red: a `--to` inside `--store`; a `--to` under the
store's `blobs/`; green: a symlink to the store refused on both providers; `ekr seed` and
`ekr session --create` write `/3` on both providers).

## Attacked and not broken

A payload blob with the same address and other bytes; a payload blob deleted after seeding; a `/2`
store with an elected, unpublished seed decision; sources with pending, stale and rejected
transactions and checkpoints; migrate with an unresolved decision (refused, writes nothing); the
same `/3` envelope bytes on both providers; zero evidence; two evidence ids sharing one payload;
payload bytes in events, preparation records, checkpoints or a migrated store.

```findings
- file: crates/ekr/src/cli/migrate.rs
  line: 16
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a --to inside --store is created inside the source directory, so the source is not left untouched, and under blobs/ the source can no longer be opened by any command.
- file: crates/ekr/src/cli/migrate.rs
  line: 16
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: migrate --to <store>/blobs/v3 exits 0 and every later open of the source fails with invalid blob object name.
- file: crates/ekr-kernel/src/migrate.rs
  line: 191
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an interrupted migration leaves a destination that opens as an ordinary store with a shorter head and no marker, and docs/cli.md does not mention that outcome.
```
