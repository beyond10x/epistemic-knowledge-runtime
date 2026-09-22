# Current assertion and event serialization test migration

The coordinator temporarily owns only crates/ekr/tests/graph_assertion_serde.rs
and graph_events_serde.rs; the durable worker confirmed both were untouched.
No product source or historical encoder changes in this migration.

The retained baseline fails to compile against removed ValidationState/validation
fields and old bare RevisionEvent variant constructors. The original refusal
checks for inverted temporal ranges and canonical floats remain, as do optional
bounds, sum-type round trips and event variant/wire distinctions.

Assertion cases now cross the independent assessment and lifecycle shapes while
retaining all original assessment payload examples and temporal combinations.
Withdrawal cannot erase the assessment during the round trip. An additional
current-format case refuses either missing state axis and the old combined field.
These codec cases do not claim that every representable assessment is admissible
canonical state; that policy remains the kernel's.

Event cases now round-trip the required versioned occurrence envelope and its
payload. Wire distinctions use the same envelope identity/address across all
payload variants, so differing IDs alone cannot make the test pass. Additional
checks hold required and unique envelope fields, unknown-field refusal and a
proposal's absent optional operations hash. No unsupported-version admission
claim is made for the plain graph data carrier.

The first migrated run passed; selected strict Clippy and formatting exited zero.
The following summaries are extracted from the retained first.log when this
report is written. Original compiler failure and all command logs remain in the
private serde-test-migration evidence directory.

> test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
> test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

This is bounded fixture migration, not the full gate or provider acceptance.
Owners: the coordinator owns these test changes and their verification; after
this commit integrates, fixture ownership returns to the durable implementor.
