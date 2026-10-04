# Diagnostic record

The original integration tests compiled and executed before correction orchestration existed.
Both failed with selected correction application not implemented (correction-integration-red-2).
The first invocation instead failed to compile because a sequential worker checkout had built older
generated contracts in the shared Cargo cache; refreshing the actual generated input's mtime
recompiled it. That compile error is not a behavioral red.

The initial Choose implementation passed on both providers. Explicit Unresolved initially reported
SchemaCommitted, while the declared outcome was Partial; the final driver records its explicit
pending reason without minting an empty correction transaction. The two-case rerun passed.

Independent review found renewal incorrectly replacing the original review in frozen-operation
validation. A real interrupted-prefix reproduction failed with Partial and correction step has no
prior exact approval. The fix records/authenticates the original review separately and retains the
current review guard; the original reproduction then passed.

The coordinator's temporal test also removed the review proof object from real captured history.
Verification incorrectly succeeded because the embedded proof was still present. The fix compares
each review's proof, policy and statement with its declared independently retained object bytes.
The missing-proof test passed afterward; expanded controls also remove and alter policy and
statement objects against both warm and fresh authorities.

The first receipt-recovery test compared every provider event even though recovery must retain a
new progress receipt. Its expectation was corrected to compare canonical revision events. This
was a test-expectation error, not a production red/green result.

The first strict Clippy pass found one test-only clone on a Copy subject. It was removed without
suppressing the lint; the second strict pass exited zero. Raw logs remain outside disposable build
outputs; verification.md records their hashes and measured summaries.
