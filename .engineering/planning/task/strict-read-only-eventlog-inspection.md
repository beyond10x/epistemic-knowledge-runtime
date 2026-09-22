---
format: aep.planning-md/1
id: task:strict-read-only-eventlog-inspection
kind: task
status: draft
title: Provide a nonmutating provider entry for preserving migration inventory
relations:
- derived_from: story:version-persisted-contracts
- blocks: story:version-persisted-contracts
revision: 1
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
