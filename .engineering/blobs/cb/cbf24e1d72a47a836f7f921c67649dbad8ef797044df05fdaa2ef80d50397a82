unit: strict CLI host input, 4467038d99d22b1dc99b475e8f45106f4da12a53 plus additive review cases
verdict: nothing found
cases: executed 20→26, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: assigned review directory and compiler target/TMPDIR; private inventory retained separately
needs-coordinator: integrate six uncommitted cases and run the coupled product gate

`git --no-pager diff --stat` (inherited coordinator changes, byte-unchanged):
```text
 docs/epistemic-knowledge-runtime-design.md | 119 ++++++++++++++++++++
 systems/ekr/domains/store.yaml             | 175 +++++++++++++++++++++++++++++
 2 files changed, 294 insertions(+)
```
Review-authored untracked file, measured separately by `git diff --no-index --stat /dev/null crates/ekr/tests/host_input_adversary.rs`:
```text
 crates/ekr/tests/host_input_adversary.rs | 283 +++++++++++++++++++++
 1 file changed, 283 insertions(+)
```
The two non-test dirty paths existed before this review and remain byte-identical. No implementation or inherited assertion was edited.

1. Cases were written and formatted before execution in `crates/ekr/tests/host_input_adversary.rs`. All six were first executed alone with:
   `cargo test -p ekr --test host_input_adversary --locked --offline <case> -- --exact --nocapture`.
   Each exited 0 and reported exactly `1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out`. Complete first-run output is retained in the six numbered logs; there is no red output.

   | Case | Assertion |
   |---|---|
   | all_named_carrier_fields_are_strict_through_both_public_routes | Every field of all five carriers is required; escaped duplicates refuse before malformed repeated values; escaped unknown fields refuse through both from_json and streaming public Deserialize. |
   | named_object_shapes_survive_value_and_streaming_deserializer_boundaries | All five carriers refuse positional and empty arrays, null, integers and booleans through the byte reader, streaming Deserialize and Value deserializer. |
   | decoded_collisions_refuse_without_unicode_normalization | Fully escaped duplicate UUID keys refuse before their values; raw/surrogate-pair spellings of a capability collide; distinct composed/decomposed Unicode, empty and NUL strings remain distinct data. |
   | legitimate_strings_and_lists_retain_values_without_kernel_policy_checks | Numeric-looking and Unicode strings survive exactly; real list/set semantics remain; authority/profile consistency stays kernel-owned; JSON numbers do not coerce into string fields. |
   | valid_dates_follow_independent_gregorian_ordinal_across_centuries | 5,116 valid dates across fourteen boundary years, including year zero, agree with an independent Gregorian ordinal; 504 invalid month-day combinations refuse; corresponding decimal milliseconds agree. |
   | decimal_extremes_and_byte_level_noncanonical_forms_stay_distinct | Signed i64 endpoints and non-binary-exact integer milliseconds stay exact; overflow, alternate numeric forms, embedded controls and noncanonical date suffixes refuse while retaining original error input. |

2. Only after the isolated cases, the selected combined suite ran:
   ```text
   cargo test -p ekr --lib --test host_input_adversary --locked --offline
   test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   exit: 0
   ```
   The before count is the implementor's reported 20 library cases, also observed in this combined run. Full runner output is retained in `suite.log`.
   `cargo clippy -p ekr --lib --test host_input_adversary --locked --offline -- -D warnings` exited 0.
   `cargo fmt --all -- --check` exited 0.
   `git --no-pager diff --check` exited 0.
   All Cargo executions used the assigned exclusive target and TMPDIR, two jobs, debug=0, incremental=0 and an empty RUSTC_WRAPPER. Persistent disk, tmpfs and memory floors held before compilation.

3. No concrete implementation discrepancy or additional judgement finding was found within this scope.

4. Boundaries exercised without a failure: strict required/unknown/duplicate fields, decoded collection collisions, all five named object carriers, legitimate Unicode/string/list contents, preserved kernel policy ownership, exact signed milliseconds and calendar boundaries. The original 20 cases additionally executed complete-document/UTF-8/trailing-data and envelope-format refusals. No current binary caller exists yet; these tests exercise the public library API designated for later CLI integration.

5. Source identity and ownership:
   | Path | Before and after SHA-256 |
   |---|---|
   | crates/ekr/src/host.rs | d2299e4943d741c70eb5ad64244f2d04b7707880138457e652cb4707839205d3 |
   | crates/ekr/src/lib.rs | af14faf2ed6d91a1be323d2473df92b17f5ac08518c4d8d101abd1e4dc291c98 |
   | docs/epistemic-knowledge-runtime-design.md | bd6198b7abc05fc34448531890586f90218f07fc7b138f2d10e60c88fa83f9bc |
   | systems/ekr/domains/store.yaml | 3118bb6a2f3e6e75f52ab351024c3f6324eb464847f2b53e0899586364e0310a |

   Review test SHA-256, unchanged from before first execution through handback:
   `28eeaa1da33c35917b23d0453b0383ab730376d07dfece3ebae3fbacfa472568`.
   Source base comparison is `7444af552563909a52658d99df708a6dd1c6fcc2..4467038d99d22b1dc99b475e8f45106f4da12a53`: two added source files, 827 lines.

   Owners: the implementor owns the frozen host/lib implementation and its 20 cases; this reviewer owns only the new six-case integration file. The coordinator owns both inherited metadata changes, planning, integration and publication. No finding requires repair ownership.

6. Raw logs, exact commands, before/after hashes, resource and lifecycle output remain in the assigned review directory. Full private outside paths are listed in `outside-files.txt`, separated from this public-safe report. Reviewer lease `codex-ekr-cli-host-input-adversary` was released successfully; the coordinator's lease remains. No process started by this review is still running. Added tests are uncommitted; no cleanup or publication was performed.

This is a bounded library-input review. It does not execute the final CLI commands, providers, clocks, retained authority opening, durable writer or a full repository gate. The known old writer-owned integration fixtures were not selected or altered. Arbitrary non-JSON serializers and resource quotas are outside this task's contract.

```findings
[]
```

