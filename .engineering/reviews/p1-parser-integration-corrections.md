# Parser integration corrections

Owners: the coordinator owns the README/toolchain regression and integrated-gate
closure. The parser implementor omitted direct test use of the public profile
exports. Neither finding changes parsing semantics or frozen limits.

The first actual task check stopped in
crates/ekr/tests/adversary_docs_contract.rs. Updating the released ESS tool
instructions had removed the README's declared minimum Rust version. The
repository still declares rust-version 1.91 and pins the execution toolchain
separately. Restore both facts in the README; retain the unchanged guard.

The next actual task check stopped in
crates/ekr/tests/public_surface.rs. No test directly exercised the public
DocumentLimits and DOCUMENT_V1_LIMITS exports, although the parser's behavior
boundaries were exercised. The existing
original_document_byte_limit_is_inclusive_and_counts_utf8_bytes case now reads
the exported profile through its public type, pins input_bytes to the historical
value and constructs its existing exact/one-over inputs from that value. No
production source, original independent case or guard is changed.

Both original failed gate logs and separate exit statuses remain in the private
parser-integration-gate evidence directory. A passing scoped or full gate must
be recorded separately; this correction report does not assert one.
