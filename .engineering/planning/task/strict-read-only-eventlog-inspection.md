---
format: aep.planning-md/1
id: task:strict-read-only-eventlog-inspection
kind: task
status: implemented
title: Provide a nonmutating provider entry for preserving migration inventory
relations:
- derived_from: story:version-persisted-contracts
- serves: vision:o2
revision: 6
---
## Context

The bounded migration inventory and dependency review are recorded in
.engineering/waves/p1-migration-inventory.md and .engineering/waves/p1-read-only-provider-scope.md.
The pinned published Eventlog providers expose ordinary openers that initialize or recover source
state. Cached capture work is unpublished and its SQLite route still opens for writing; it is not
a supported dependency or evidence of byte-unchanged inspection.

## Acceptance

Before EKR claims live provider inventory, a published provider-owned strict inspection API must
read original record metadata without provisioning, recovery or source-side writes. Missing,
unsupported, changing or recovery-required source receives a named refusal. Preserve healthy WAL
content or refuse it explicitly; never silently omit it or use immutable=1 on a live database.
Tests establish source entry/byte preservation through success, refusal and handle drop for both
providers, bounded reads, and retained original event identities/versions/positions.

Re-use Eventlog's own format decoders and fold; do not add a product-specific parser in EKR.
Coordinate any new verified upstream dependency pin across core/file/sqlite and Cargo.lock.
The kernel can implement frozen-document classification while this prerequisite remains open,
but classification is not live-store inventory. The report gives the bounded upstream surface.

## Boundaries

No data copies, source mutation or service cutover were authorized by the inventory itself.
The completion plan separately authorizes verified implementation and migration. Existing v2
data remains in place; private source-path mapping is outside public version control.

## Upstream publication

## Published source verification

The bot integrated the exact checked source at
d90637b2fe2bbb33c9b395b0367cab8fbb80286d through
https://github.com/beyond10x/eventlog/pull/10; the primary checkout is synchronized.
Required source CI ran the real PostgreSQL production, comparative and restart
proof alongside the shared security/privacy check. The downloaded proof identifies
the synthetic PR merge; every candidate source-manifest byte was independently
compared to the integrated source. No candidate source was dirty.

The production proof reports the results appended below. The comparative report
declares laboratory validity and retains the paired configurations and restart
observations. It does not admit production deployment capacity. The CI proof and
source-byte comparison are retained in the completion run's inspection scratch.

This completes the bounded history inspection API. It does not establish a
complete store backup, blob extraction, history verification, migration or EKR
consumer adoption. The consumer will pin this source together with the subsequent
atomic blob publication capability through its own coordinated manifest and lock.
CI production result: 156 passed, 0 failed, 0 skipped; conformance_valid=true. Missing required cases: 0. Source: d512109dc2c651614cdc74a2b5fc17fc5817c50e.

## Consumer adoption and closure

EKR now selects the published combined inspector/atomic source
4ee3dc23f0d02a5726a0e41d097477791f09efe2 consistently for core/file/sqlite,
Cargo.lock and the dependency-contract guard. The complete combined integration
gate in wave-2026-09-22-p1-10.md passed against that revision. This closes the
bounded strict provider inspection prerequisite and its consumer adoption.
It still does not claim that a live source has been inventoried completely,
backed up, verified or migrated; those operations keep their separate acceptance.
