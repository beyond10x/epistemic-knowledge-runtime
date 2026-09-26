---
format: aep.planning-md/2
id: architecture-decision-record:0011-transaction-document-format-2
kind: architecture-decision-record
status: accepted
title: 'ADR 0011 — ekr.transaction-document/2: 10,000 operations in 8 MiB beside the frozen /1'
summary: A new document version with its own frozen profile; /1 keeps 256 operations in 262144 bytes
relations:
- decides: task:bounded-transaction-document-parser
revision: 2
---
## Status

Accepted on 2026-09-26 by the operator's orchestrator for release 0.0.7. This is the explicit
document version decision design § 91.3 requires for a changed frozen profile; design § 97 states
the format.

## Context

`ekr.transaction-document/1` admits 1 to 256 operations in at most 262144 bytes, frozen by § 91.3.
A change of a few thousand operations had to be split into many transactions. Once replay stopped
repeating per verb (§ 96), one propose, validate and commit of 10,000 operations measured about 6 s.

## Decision

Add `ekr.transaction-document/2`: the envelope, grammar and refusals of `/1`, with a frozen profile
of at most 10,000 operations, 10,000 evidence entries and 8388608 bytes (8 MiB), and the other
limits scaled as § 97.1 lists. Every `/2` limit is at least its `/1` value. `/1` stays admitted
under its unchanged profile, and every retained `/1` proposal is read, validated and replayed as
before. The binary's examples, schema and guide write `/2`.

The profile is selected before parsing from the envelope's top-level `format:` line; without one
the `/1` byte cap applies, and every document is held to the profile of the version it declares
(§ 97.2).

## Alternatives

Editing § 91.3's limits in place would change the validity of retained history and is forbidden by
§ 91.3. Making the limit a host setting would let a caller change historical parsing, which § 91.3
also forbids. Parsing first and choosing the byte cap afterwards would read up to 8 MiB of a
document that declares `/1`, before the `/1` byte cap is applied.

## Evidence

`crates/ekr-kernel/tests/transaction_document_v2.rs`, including
`a_v2_document_of_10000_operations_validates_and_commits_on_both_providers`, and the unchanged `/1`
cases in `transaction_document.rs`.
