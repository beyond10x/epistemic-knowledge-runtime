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
