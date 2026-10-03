# Native answer recovery evidence

Story `story:resolve-knowledge-question`, root-local verification. The production contracts and
implementation are unchanged; this extends recovery coverage of the published signed-answer path.

The kernel elects an exact preparation under normal authority. A child restores that request and
verifies its native fingerprint before submitting it to the unmodified File or SQLite provider.
The child exits before submission, inside the first or last inline event callback, or after native
commit. The callback reads every staged blob inside the actual provider transaction before exiting;
a persisted witness and exit code distinguish reaching that boundary from an unrelated failure.

All 16 combinations passed: two providers, four boundaries, and choosing a claim or correcting its
time. Fresh native handles verify that a precommit exit exposes exactly the previous event feed and
blob bindings. A postcommit exit retains every expected event, its coordinates and all blob bytes.
Full kernel replay then reproduces answer history and attention. A new process repeats the signed
answer without sampling time and obtains the exact elected record; another retry changes no native
event coordinates or blob bytes. Temporal recovery retains one explicit replacement and the still
overlapping question. Changed input is refused before retry.

`recovery.log` contains 2 answer cases (including the 16-cell native matrix) and 5 existing recovery
cases. `regressions.log` contains 10 existing adversarial recovery cases. All 17 test executions passed
with zero failures or ignores. Targeted Clippy with warnings denied and the repository's actual
`xtask fmt --check` passed. `measurements.json` binds the tested source bytes and recorded time.

This witnesses abrupt process death, including live native transactions; it does not simulate power
loss, torn journal frames, failed fsync, every provider-internal instruction, or an interruption during
preparation election. The normal EKR adapter is used for election and retry; the child uses the
provider directly for the exact elected request so a public inline callback can stop inside its
transaction. No production failpoint or modified dependency is used. These tests are not named ESS
conformance, independent review, either end-to-end schema-learning demonstration, or the full gate.
