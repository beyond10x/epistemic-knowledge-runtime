---
format: aep.planning-md/3
id: task:checkpoint-seed-payloads-are-unbound
kind: task
status: implemented
title: A replay checkpoint with emptied seed_payloads is admitted, and verified reads lose the seed evidence bytes
summary: 'Medium: restore trusts the checkpoint''s seed payload set instead of the seed envelope'
tags:
- review-2026-09-28
- severity-medium
relations:
- serves: vision:o5
- derived_from: story:eventlog-0-4-batched-reads
- informed_by: architecture-decision-record:0010-replay-checkpoints-record-verified-history
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T02:51:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T02:51:37Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-09-28T06:19:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Severity

Medium (the review's rating, kept). A read silently loses evidence bytes; nothing canonical is
written wrong, and `--full-replay` returns them.

## The review (external review of EKR, 2026-09-28, verbatim)

> Medium — checkpoint restoration can lose evidence from verified reads. Clearing its cached
> seed_payloads makes `VerifiedRead.content()` return no evidence bytes, while full replay returns
> them. Reproduced on both providers. `crates/ekr-kernel/src/checkpoint.rs`

## What was verified (code reading at `wave/p2p3p4-06` 69e5b994; not run)

- **Verified**: `restore_checkpoint` copies `checkpoint.seed_payloads` into the authority's cache as
  the seed's required payloads (`crates/ekr-kernel/src/checkpoint.rs:250`, `:257`) and into the
  restored state (`checkpoint.rs:468`) without comparing them with the seed envelope's
  `evidence_payloads`. No check in `restore_checkpoint` or `restored` reads the field.
- **Verified**: `required_objects` returns that cached set without re-reading the envelope
  (`crates/ekr-kernel/src/commit.rs:33-35`, `seed_requirements` at `:69-80`). The envelope path it
  skips derives the set from `envelope.input.evidence_payloads` (`commit.rs:36-40`).
- **Verified**: `VerifiedRead::content` answers only from `history.objects`
  (`crates/ekr-kernel/src/read.rs:48-50`, filled at `:103-107`), so a payload the store was not
  asked to load is absent.
- **Verified**: the forgery test does not touch `seed_payloads`
  (`crates/ekr-kernel/tests/replay_checkpoint.rs:309-326`).
- **Inferred, not observed by me**: that the store then loads no evidence payloads and `content()`
  returns `None` for a seed evidence hash. The review says it reproduced this on both providers.

## Reproduce

Add a forgery beside those at `replay_checkpoint.rs:309` that sets `forged["seed_payloads"]` to
`[]` on a seed with at least one evidence payload. Install it, open without full replay, take a
verified read and call `content(<seed evidence hash>)`; compare with the same call after
`open_in_full`.

## Acceptance

On both providers, after a checkpoint whose `seed_payloads` differs from the seed envelope's
evidence payload set is installed, a non-full open's `VerifiedRead::content` returns the same bytes
for every seed evidence hash as a `--full-replay` open (the checkpoint is ignored, or the set is
re-derived from the envelope). A forgery case in `crates/ekr-kernel/tests/replay_checkpoint.rs`
that empties the set fails before the fix and passes after it.
