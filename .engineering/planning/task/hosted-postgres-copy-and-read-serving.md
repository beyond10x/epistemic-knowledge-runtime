---
format: aep.planning-md/3
id: task:hosted-postgres-copy-and-read-serving
kind: task
status: draft
title: Hosted PostgreSQL, preserving store copy and remote read serving
relations:
- serves: vision:o5
- serves: vision:o2
revision: 2
---
# Hosted storage and read serving requirements

Support an independently operated hosted EKR instance initialized from an existing development SQLite store, without rerunning observation extraction. This is a generic upstream requirement handoff, not an implementation or a completed feature.

## Storage acceptance

- Expose eventlog PostgreSQL through EKR runtime, SDK and CLI, with verified TLS, bounded pools and separate schema-management/application authority.
- Copy a consistent SQLite store into an empty PostgreSQL destination, preserving every historical revision, schema version, stable identifier, assertion lifecycle, retained evidence byte and logical content hash. Emit a machine-readable equivalence report. Refuse non-empty unrelated destinations; interrupted copies are never served as complete.
- Stage a validated update against a captured base and publish only its new revision suffix atomically under an expected-head check. A conflict, invalid suffix or failure leaves served state unchanged. Exact retry is idempotent.
- Define safe credential-file/secret-reference configuration; redact credentials from diagnostics. Existing SQLite operation remains compatible.
- Test real PostgreSQL, restart recovery, interrupted copies, historical reads, evidence byte equality and concurrent publication with invented synthetic fixtures.

## Serving acceptance

- Add configurable network listeners to the read-only viewer and support Streamable HTTP MCP alongside existing stdio. Preserve existing read tool semantics, bounded request handling and revision consistency.
- Provide a search-first viewer entry with relevant results, retained evidence previews and links into existing graph and history views. Keep the viewer ontology-independent.
- Support operation behind an external authentication/routing service. This request does not introduce an identity provider, public exposure or authentication subsystem.
- Publish documented CLI/SDK configuration and health/readiness semantics in a release tag before a consumer adopts the capability. Guessed command-line options do not form a contract.

## Ownership and next action

The EKR session owns decomposition, ESS changes, implementation and release. These outcomes extend existing store and view domains; this handoff introduces no consumer-specific entity or fixture. Review the existing eventlog PostgreSQL provider and preserving migration kernel before selecting implementation units. This task remains draft until that session scopes and schedules it.
