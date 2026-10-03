# Schema-only application recovery

Follow-up to checkpoint `2886e477559c1c60e7563e94bece005a1a40892d`.

An application whose schema validation becomes stale now preserves the immutable semantic step,
records the exact terminal Stale predecessor and elects a successor changing only the transaction
ID. It uses the store's elected winner. Unknown publication errors propagate before any successor
is elected, and a bounded contention limit returns a retryable conflict. A completed schema can
recover its missing reporting receipt from the actual linked commit.

Independent review identified an additional boundary: an unpublished Validate preparation already
fixes the requested revision. Choosing a new head after unrelated advancement changed the immutable
input and left repeated application refused. The regression reproduced this with a real checked
preparation from a closed provider snapshot. The ordinary validation-key construction is now shared;
application recovery reads the original checked validation/rejection basis and invokes the ordinary
handler with it. Its existing input-hash, authority and publication checks remain in force.

The final test exercises both file and SQLite at five boundaries: validated before Stale, already
Stale, committed without final receipt, unpublished Validate preparation with unrelated advancement,
and the same preparation without advancement. It requires cold full replay, exact frozen fields,
one semantic step, the expected attempts and receipt, and no effects on a completed retry.
It also refuses successor time before the terminal decision without retaining new state.

The immutable independent reviews are `schema-application-stale-recovery-r1` and
`schema-application-stale-recovery-r2` in the planning store. The first finding was reproduced and
fixed; the bounded rereview approved the correction. Neither reviewer executed a test or accepted
all of Story F. `recovery-verification.txt` preserves actual runner output and raw-log hashes.

The earlier checkpoint's unfinished stale-successor item is superseded by this evidence.
Source/observation-backed schema application, mappings, selected corrections, residual human review,
full conformance and the full PR gate remain unfinished.
