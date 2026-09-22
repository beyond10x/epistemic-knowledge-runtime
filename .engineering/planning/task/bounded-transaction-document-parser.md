---
format: aep.planning-md/1
id: task:bounded-transaction-document-parser
kind: task
status: active
title: Parse and retain bounded transaction documents without changing their bytes
relations:
- derived_from: story:commit-and-revision-lineage
- serves: vision:o2
- decomposes: story:commit-and-revision-lineage
revision: 4
---
## Contract

Implement the already approved frozen transaction-document/1 parsing profile in
.engineering/waves/p1-durable-record-decisions.md and the bounded route in
.engineering/reviews/p1-transaction-parser-readiness.md. This is independent
preparation for story:commit-and-revision-lineage while its released compiler
prerequisite is verified. It does not activate graph/seed/event version two or
establish persistence, conformance or a completed writer.

## Acceptance

The shared kernel parser retains exact UTF-8 input bytes and their payload hash,
parses the typed YAML operation grammar from those bytes, and refuses unsupported
versions, unknown and duplicate semantic fields, duplicate decoded map keys,
wrong container shapes, multiple documents and every exceeded frozen budget.
Aliases charge their expanded position. Nonfinite Float input stays representable
for later named validation refusal. Preserve supported typed string semantics,
ordinary Record keys, outer multiplicity and explicit empty inner collections.

Write boundary and mutation-sensitive regressions before implementation; preserve
the full frozen profile and the readiness report's positive controls. Do not claim
pre-loader allocation quotas the selected parser cannot enforce. No unrestricted
Value tree, invented YAML lexer, new dependency, provider write or live-store read.

The writer owns final-format operation integration, retained Proposed/restart
acceptance and no-write provider controls. This task remains active until its
parser passes independent review and the final operation-shape integration checks.

## Scope

Inferred new modules and tests: crates/ekr-kernel/src/document.rs,
crates/ekr-kernel/src/document/, crates/ekr-kernel/tests/transaction_document.rs.
Cited integration: crates/ekr-kernel/src/lib.rs and transaction.rs. Any necessary
strict nested declaration annotation in ontology or graph requires a measured
scope addition before editing. Original legacy representations stay frozen.

# Parser readiness corrections during implementation

Owners: the coordinator owns the readiness acceptance premise; the implementor
found its contradiction with the actual identity decoder. This is a correction
to preparation, not a relaxation of duplicate refusal or an implementation verdict.

The readiness report asks for two supported textual spellings of one typed
property identity. `crates/ekr-core/src/identity.rs:125` instead explicitly accepts
only lowercase hyphenated UUID text, through `is_canonical_uuid_text` before UUID
parsing. Uppercase, simple, braced and URN spellings are not supported alternatives.
The implementor's retained `transaction-parser/iteration-5.log` reproduces the
uppercase input refusing as an invalid UUID before any duplicate-key result.
Root inspected both the source and that log on 2026-09-22.

Keep canonical identity syntax unchanged. The applicable duplicate witnesses are
equivalent YAML escapes that decode to the same key, and identical canonical
typed keys with an invalid second value proving the duplicate refuses before that
value is decoded. Add explicit noncanonical-spelling refusals as controls. Nested
ontology operations need the same typed-key protection. The original failed
probe remains retained; this correction does not claim those replacement cases
or the parser implementation passed independent review.

The same readiness report qualifies BOM preservation by the selected parser's
accepted syntax. The implementor's `transaction-parser/marker-probe.log` compares
direct and bounded decoding: the pinned YAML parser refuses the BOM input, while
non-BOM CRLF, markers and trailing comments are accepted. Preserve exact bytes of
admitted inputs; do not strip a BOM or add a parallel YAML normalization path.
