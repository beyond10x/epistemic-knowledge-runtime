# Current domain contract test migration

Coordinator-owned integration work over the frozen format/seed checkpoint.
Only the graph domain-projection guard changes; the ontology projection guard
already matches the current contract and passed unchanged.

The graph guard binds the assessment and lifecycle declarations to their
separate current carriers, excludes the frozen legacy module, and checks each
field within its actual carrier. Renamed or derived fields require concrete
members on the named type. The private graph-document envelope is read only
from the current store codec. An unrelated field on another type cannot
satisfy a declaration. These are source-carrier checks, not execution of the
future CLI transport projections.

The old payload quotation case described an intentional omission that no
longer exists. Its replacement constructs the actual current Assertion and
checks that changing retained assessment attribution changes its canonical
bytes under active, retracted and superseded lifecycles. A separate reason
comparison holds the retraction payload. The baseline failed on the obsolete
validation-state name and missing current projection mappings.

Removing the real Assertion encoder's assessment contribution made the new
payload case fail. The exact product source was restored and compared with
its saved bytes before the final run. The mutation is a regression witness
for retained attribution; it does not establish complete replay or provider
durability.

The selected graph and ontology targets, strict selected-target Clippy and
formatting exited zero. Raw baseline, intermediate, mutation and final logs
remain in the private domain-projection-migration evidence directory. Final
runner summaries and the deliberate mutation result, extracted from those
logs when this report was written:

> test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
> test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

> test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

This bounded migration is not the full repository gate. Owners: the coordinator
owns this guard repair and its verification; the writer unit owns the coupled
product implementation and final fixture migration.
