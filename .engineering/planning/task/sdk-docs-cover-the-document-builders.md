---
format: aep.planning-md/3
id: task:sdk-docs-cover-the-document-builders
kind: task
status: active
title: docs/sdk.md documents the document builders and the ontology by name
relations:
- serves: vision:o5
- decomposes: epic:consumer-sdk
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T19:39:15Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-29T19:39:15Z", actor: "human:timo", revision: 3}
---
## Context

`story:sdk-typed-documents` shipped in 0.0.19 with builders for every document a consumer writes
(`ekr_sdk::document`: `TransactionBuilder`, the seed builder, typed references, local minting and
payload hashing, `OntologySpec`, `Ontology::read`, `Ontology::ensure`, `DocumentError` including
`Limit`). `docs/sdk.md` has no section on them: `TransactionBuilder`, `OntologySpec` and
`Ontology::ensure` appear nowhere under `docs/` (checked 2026-09-29 on `wave/sdk-02` `aa261036`),
while the 0.0.19 `README.md` says `docs/sdk.md` covers "document builders" and "the ontology by
name".

## Build

- A "Documents" section in `docs/sdk.md`, after "Replies" and before "Resolve before you create":
  building a transaction and a seed, minting and hashing locally, the ten transaction limits and
  `DocumentError::Limit`, and `OntologySpec` with `Ontology::ensure` (what it emits, what it
  refuses by name, profile v1).
- Every Rust example in the section compiles: a test in `crates/ekr-sdk/tests/` extracts the
  section's code blocks, or each example is a doc test on the public item it shows.

## Acceptance

- `docs/sdk.md` names `TransactionBuilder`, `OntologySpec`, `Ontology::ensure` and
  `DocumentError::Limit`, and each name it uses exists in the public API.
- The section's examples compile in the crate's tests or doc tests.
