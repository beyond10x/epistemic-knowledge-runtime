---
format: aep.planning-md/2
id: task:strict-cli-host-input
kind: task
status: implemented
title: Decode trusted CLI host configuration and valid-time selectors
relations:
- decomposes: story:ekr-cli
- serves: vision:o2
revision: 4
---
## Context

This independently testable part of story:ekr-cli decodes trusted local host
configuration and its valid-time selector. The complete CLI still depends on
the durable writer and Explain; this task does not claim those are ready.

The host portion of `.engineering/waves/p1-cli-explain-contract-r2.md` is the
selected contract. `systems/ekr/domains/kernel.yaml` declares its typed home.
BootstrapContext and AuthorityStateV1 are existing kernel types; reuse their
strict decoding without duplicating identity, authority or profile semantics.

## Acceptance

Expose a small library module in the existing ekr package, used by the eventual
binary. Decode the JSON envelope with exact format, tenant, context and authority;
refuse missing, unknown and duplicate fields, duplicate decoded map/set members,
unsupported format and trailing data. Do not create an implicit host or override
the kernel's authority checks. Tests use only synthetic runtime vocabulary.

Parse valid-time selectors as existing canonical Timestamp decimal milliseconds
or exactly YYYY-MM-DD at midnight UTC, validating the calendar with the workspace
time crate. Hold leap days, pre-epoch dates, invalid calendar values, overflow and
noncanonical decimal/date spellings with cases. Core Timestamp grammar is unchanged.

The tests are unexecuted at dispatch. This task does not implement command
dispatch, clock sampling, provider opening, Snapshot or Explain. Its source is
reviewed independently and integrated into the eventual full product gate.

## Scope

- crates/ekr/src/host.rs: typed JSON envelope and valid-time parser, inferred.
- crates/ekr/src/lib.rs: documented public host module, inferred.
- crates/ekr/Cargo.toml and Cargo.lock: existing workspace serde/time dependencies,
  coordinator-owned; no provider/version change, cited.
- systems/ekr/domains/kernel.yaml: host transport projection, coordinator-owned, cited.

## Delivery

The operator's standing implementation and integration instruction covers this
bounded preparation. A separate managed checkout and compiler target isolate it
from the active durable source unit. The coordinator alone writes planning,
shared specifications and dependency manifests. Full CLI story readiness and its
dependencies remain unchanged.
