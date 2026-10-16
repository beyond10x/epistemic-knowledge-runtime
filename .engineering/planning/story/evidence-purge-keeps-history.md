---
format: aep.planning-md/3
id: story:evidence-purge-keeps-history
kind: story
status: draft
title: A purge deletes evidence payloads and their sole dependants, and keeps history as tombstones
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o2
revision: 1
---
## Context

A consumer had to remove every assertion and every evidence item that came from a class of sources
later ruled out of scope. Retraction and supersession (design § 35.1, § 36) keep the removed text
readable in history, which is what they are for, so they could not do it. The consumer re-seeded the
filtered head into a fresh store, which dropped all revision history before the new seed; it checked
the result with `ekr compare` and a dump restore.

Design § 35.3 and § 60 already ask for this: physical deletion of evidence independently of
epistemic history, with dependent assertions retracted, disputed or revalidated. `epic:p6-maintenance-observability`
promises "deletion requests through eventlog redaction"; no story under it builds one. The pinned
Eventlog provider offers `redact` and `delete_blob` as its withdrawals (see
`architecture-decision-record:0013-a-run-is-staged-and-published-whole`, Context).

Requested: a purge selected by an evidence predicate and a stated reason. It deletes the matching
evidence payloads and every assertion that cites only them, leaves a tombstone in each affected
revision that keeps ids, times and the purge reason, and records the purge itself as a revision.
`ekr compare` reports purged ids apart from other differences, and no read surface returns a purged
payload.

## Invariants this story holds

- Invariant 5: committed revisions stay immutable as revisions. The purge does not rewrite a
  revision's identity or order; the payload bytes go through the maintenance path, which is the only
  path that physically reclaims, and what remains in place is a tombstone the replay recognises.
- Invariant 1: the purge's own revision is a `ValidatedTransaction` the kernel builds; the deletion
  of bytes is the maintenance path acting on what that revision names, never a direct store write.
- Invariant 4: an assertion that keeps other admissible evidence is not deleted; it loses the purged
  citation and is revalidated, and becomes `Disputed` or is retracted where policy no longer finds
  its provenance sufficient (design § 60).
- Invariant 6 does not apply: nothing outward is written.

## Build (spec first)

1. `systems/ekr/domains/store.yaml` and `systems/ekr/domains/kernel.yaml` first, green under
   `ess specify validate --path systems/ekr` and their conformance suites before any code:
   - a `Purge` record (the predicate as recorded, the reason, the revision that records it, the
     evidence and assertion ids it removed) and a `Tombstone` for an evidence item or assertion
     (its id, its original recorded and valid times, the purge it belongs to; no payload);
   - the purge command, its refusals (an empty selection, a predicate that is not deterministic, a
     store without maintenance authority) and its effect on dependants;
   - how replay and replay checkpoints (design § 96.3) treat a tombstoned revision: the revision
     still replays, its roots are recomputed over tombstones, and the checkpoint records that the
     history it verified contains a purge.
2. `systems/ekr/domains/views.yaml`: every read, MCP tool and `ekr view` route answers a tombstone
   for a purged id and never its payload; `ekr compare` gains a purged-ids section.
3. `systems/ekr/domains/cli.yaml` and `docs/cli.md`: `ekr purge --evidence-filter <predicate>
   --reason <text>`, with a dry run that lists the selection without deleting.
4. A design amendment (a new dated section) saying how this purge relates to § 35.3, § 38–39 (grace
   period) and § 60.

## Acceptance

- `purge_removes_payloads_and_sole_dependants`: on a synthetic store with evidence from two source
  classes, a purge selecting one class leaves no byte of the selected payloads in the store, removes
  every assertion that cited only them, and keeps every assertion that also cites the other class.
- `purge_keeps_history_as_tombstones`: after the purge, `ekr history` and an explicit historical
  read of every earlier revision still answer, with each purged id present as a tombstone carrying
  its id, its times and the purge reason, and no payload.
- `purge_is_its_own_revision`: the head after the purge is one revision past the head before it,
  and that revision names the purge, its reason and the ids it removed.
- `partially_supported_assertion_is_revalidated`: an assertion citing one purged and one retained
  evidence item stays, cites only the retained one, and its validation state follows policy.
- `compare_reports_purged_ids_separately`: `ekr compare` between the store before and after the purge
  lists the purged ids in their own section and reports no other difference.
- `no_read_returns_a_purged_payload`: every MCP tool and every `ekr view` route, asked for a purged
  id at the head and at an earlier revision, answers the tombstone; a test enumerates the read
  surface so a new read is covered.
- `purged_store_reopens_and_replays`: the purged store reopens, with and without `--full-replay`,
  on both providers, with identical roots.
- `dry_run_deletes_nothing`: the dry run lists the selection and leaves the store's bytes unchanged.

## Out of scope

Retention by storage class, garbage collection by reachability and the grace period (design § 37–39)
beyond what a purge needs; an approval workflow for who may purge; purging from a predicate over
assertions rather than evidence; restoring a purge.
