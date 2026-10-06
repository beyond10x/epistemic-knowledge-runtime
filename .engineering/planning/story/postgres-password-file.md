---
format: aep.planning-md/3
id: story:postgres-password-file
kind: story
status: active
title: An ekr.postgres/1 configuration takes its password from a separate file
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T09:15:42Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-06T09:15:42Z", actor: "agent:claude", revision: 3}
---
## Outcome

An `ekr.postgres/1` configuration can take the database password from a separate
`password_file`, so the connection file and the configuration directory hold no secret.

## Why

Operator decision 2026-10-05: database credentials for a PostgreSQL store come from a Connectors connection held in Connectors' secret backends, and cortex never sees them. Design B of the cortex design note (chosen 2026-10-06, operator: "do it"): Connectors launches an operator-pinned `ekr` with the connection's password document on fd 3; EKR reads a `password_file`; cortex starts every `ekr` through the launch. Order: EKR `postgres-password-file` and the Connectors launch story can run in parallel. The
cortex story depends on both: its test needs a real `ekr` with `password_file`, and its stand-in
mimics only the Connectors verb.

## Work
1. Add optional `password_file: PathBuf` to `PostgresConfiguration`. Resolve it like `ca_file`, so an absolute path such as `/proc/self/fd/3` is used as written.
2. When present: refuse if `connection_file` already carries a password (sanitised `postgres-configuration` refusal). Read it with `bounded_file(…, 65536)`, decode exactly `{"password": string}` (`deny_unknown_fields`, no trimming of the value), and set it on the parsed connection before `PostgresConfig::verified`.
3. Never echo the value. Diagnostics stay closed-category.
4. Document the field in `docs/cli.md` § Hosted PostgreSQL, add an amendment line to design § 105.1, and add a CHANGELOG entry.

## Acceptance
- `cargo test -p ekr-store postgres_password_file` passes:
  (a) a config with `password_file` pointing at a regular file opens a docker PostgreSQL store with a password-less DSN;
  (b) the same config with `password_file` set to `/proc/self/fd/N` for a sealed memfd opens it;
  (c) a DSN with a password plus `password_file` is refused;
  (d) unknown fields, non-string or oversized documents are refused, and the sentinel password is absent from stderr;
  (e) every existing `ekr.postgres/1` fixture without the field still parses.

## Files (from the design, unverified) `crates/ekr-store/src/postgres.rs`, `crates/ekr-store/tests/runtime_context.rs` (or a new `tests/postgres_password_file.rs`), `docs/cli.md`, `docs/epistemic-knowledge-runtime-design.md`, `CHANGELOG.md`.