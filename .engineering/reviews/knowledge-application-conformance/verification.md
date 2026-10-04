# Verification

The changed application adapter was tested with the released ESS 0.52.0-generated suite on
File and SQLite. The full suite still fails on the unsupported finite proposal witness described
in the open dependency blocker. These are local observations; no CI-equivalent run or full
conformance attainment is claimed. Three consecutive final runs use the same source and suite.
Their unmodified count reports are retained in reports/.

Executed commands:

- cargo test --locked -p ekr --test conformance_integrate -- --nocapture
- cargo test --locked -p ekr --lib application_report_guard_rejects_claims_not_present_in_replayed_receipt -- --nocapture
- cargo clippy --locked -p ekr --all-targets -- -D warnings
- cargo run --locked -p xtask --bin xtask -- contracts-check --ess <verified-ESS>
- cargo run --locked -p xtask --bin xtask -- fmt --check
- ess specify validate --path systems/ekr
- aep plan artifact validate

The baseline log witnesses the two application branches as unsupported. Their final reports
witness both as passed. SubmitSchemaProposal/outcome/answered remains failed, and neither the
baseline nor quarantine was weakened. The obsolete partial-delivery test was replaced by a
focused application test; both original full-suite provider gates remain mandatory and red.

Independent R1 identified incomplete application-report observation. The coordinator's behavioral
red regression lists every escaped alteration of application identity, correction flag, completion
flag, stop reason, item omission and integrated disposition on both providers. The same regression
passes with typed complete-report equality. R2 source/log review closes that finding without
claiming independent execution. A separate intermediate compile error used FromStr for OutcomeRef;
it was corrected to the existing structured decoding surface and is not a behavioral red.

The report clock is deterministic and must not be interpreted as the actual execution date.
All raw logs and reports remain outside reproducible build outputs in retained evidence storage.

Measured summaries, generated identities and SHA-256 fingerprints follow.

```text
File: partial counts (16, 1, 0, 2, 0)
Sqlite: partial counts (16, 1, 0, 2, 0)
Final run 7
File {"error":0,"failed":1,"passed":18,"skipped":0,"total":19,"unsupported":0}
Sqlite {"error":0,"failed":1,"passed":18,"skipped":0,"total":19,"unsupported":0}
Final run 8
File {"error":0,"failed":1,"passed":18,"skipped":0,"total":19,"unsupported":0}
Sqlite {"error":0,"failed":1,"passed":18,"skipped":0,"total":19,"unsupported":0}
Final run 9
File {"error":0,"failed":1,"passed":18,"skipped":0,"total":19,"unsupported":0}
Sqlite {"error":0,"failed":1,"passed":18,"skipped":0,"total":19,"unsupported":0}
application-conformance-report-green.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 68 filtered out; finished in 15.29s
application-conformance-report-red.log:test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 68 filtered out; finished in 15.54s
{"system":"ekr","specification_version":"v1","source_digest":"2289ab6b66263f553d91e328b960af9f6934eef272c699c1b9ad967900abb2f4","contract_digest":"a468fb767edcd11ae49e6758907a635f9a371e456b8690b65bee6007ab27151d"}
```

Raw log digests:

```text
dea871263b02b047a778631112428b3f887751b4fa521e6fe048c8026a989806  application-conformance-baseline.log
f3323c5ff4764236eb937eb3a02a9717802bdd3f25912dda9412d1979985fabf  application-conformance-report-red.log
adfef2975ec9bf95afaa542391e40744bbee36d508b97f84937c7202065c8493  application-conformance-report-green.log
6d6af14880a4790c404ac05363c060152bd3dd5af390c907f886ad648c1347df  application-conformance-7.log
5c0a9ebc4d34df8796a74a62daac4378acbb723ec3dfafc62f3d56fc56c9e21d  application-conformance-8.log
87dfa0cf968a647447b5a1a09e002a312c0d71bbb0986bb791c5d7a5ff49a94c  application-conformance-9.log
3cd1e5a979fdf612101eb209859bb27a5c2305b3e415627aaae039c47eb358c8  application-conformance-clippy.log
```

Final auxiliary checks exited zero: strict CLI all-target Clippy, full generated-tree drift/compile, formatting, ESS validation and AEP validation. The planning store retains its existing prose-only review warnings. R2 is recorded as review-result:application-conformance-r2.

```text
4cb32235f60225d51fb015b63b2405021f8aa27656fe8da77d01269d8de7adab  application-conformance-contracts.log
b640a54d946773347d4403e3ef73a18113b0a7c6df0046ddfb801e76478dbed5  application-conformance-fmt.log
f6900f5808bf8ba047379a039bb8fa45bc8d0e99abb8c564663a31024bc339f9  application-conformance-ess.log
53e2d3082bcf5fd31f48d0ec99785b47218b5cd86ec222c3db1925c92055c1c4  application-conformance-aep.log
c81f0927eb3a1552e1ae5aded1d7cfedfcb615e1a01c04724e2ecff4b3c8521b  application-conformance-review-r1.md
3968653f9a424f53a8110806600792628f6b1db36cc8ee8f4cb43398b7db2b58  application-conformance-review-r2.md
```
