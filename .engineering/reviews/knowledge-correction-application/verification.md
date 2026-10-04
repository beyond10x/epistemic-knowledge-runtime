# Native correction verification

Base: c16643b7fca7003e7e4f1724d5425c7b96ceffc3 plus this correction change.
One sequential Cargo lane, one compilation job, no incremental/debug artifacts.
No full task check or generated conformance acceptance is claimed here.

Executed commands (the pinned environment and raw logs are retained privately):

- cargo test --locked -p ekr-kernel --lib --test schema_applications --test schema_application_corrections --test attention_answers_recovery --test human_review --test attention_explanations --test schema_evidence -- --nocapture
- cargo test --locked -p ekr --test story_contract --test public_surface --test docs_cli --test agent_cli --test schema_application_surface
- cargo clippy --locked -p ekr-kernel -p ekr-sdk -p ekr --all-targets -- -D warnings
- cargo run --locked -p xtask --bin xtask -- contracts-check --ess <pinned-ess-0.52.0>
- cargo run --locked -p xtask --bin xtask -- fmt --check
- ess specify validate --path systems/ekr
- aep plan artifact validate

Measured native summaries from the final logs:

```text
correction-crosscrate.log:test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
correction-crosscrate.log:test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
correction-crosscrate.log:test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.99s
correction-crosscrate.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
correction-crosscrate.log:test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
correction-regression-1.log:test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.99s
correction-regression-1.log:test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.24s
correction-regression-1.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
correction-regression-1.log:test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
correction-regression-1.log:test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 104.30s
correction-regression-1.log:test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 163.56s
correction-regression-1.log:test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
```

The final five-case correction target includes the explicit originating-review identity assertion,
detached-provider Explain equality, and removal/alteration of proof, policy and statement objects
against both warm and fresh authorities. These expanded controls postdate the bounded R2 review
and are now executed by the coordinator. The helper tests, reconstruction recovery cases and
atomic mapping-before-correction case run in the listed final regression suite.

Strict Clippy, formatting and contract drift/compilation exit zero. The seven conformance suite
artifacts were regenerated from the same specification; generation is not runtime conformance.
The planning validation retains the five existing prose-only review warnings.

Generated identities:

```text
    "contract_digest": "db51800daf271146ca5948dce9aaf371fac263e98eae1f4dd09a51739c94022c",
    "spec_digest": "95e0cc738a4105082ac236fd226f432bb61b31bd4c0c3f8bafd2688d0a257609",
```

Raw evidence SHA-256 (outside reproducible build outputs):

```text
ad013608ede79d3af60b5e1eec3ffaecdf380383f90dca84bb6e7876eb56d631  correction-integration-red.log
39a2244b0e063c0e1a724705887959614b2ec9f09ac76396cb2999c73d6f4824  correction-integration-red-2.log
794536675c2fe7acc7b524dce8b0d8aedf74f5871d2751834e97486cd29614c4  correction-integration-1.log
cdd1300e92404ec5d8365824153cee306d824841028c401cd6c7300dfc75b7ba  correction-integration-2.log
8f0049537494fdcddfa64c313c370fadc01281d121ba24b0cddbc21e1abdc294  correction-integration-3.log
0b67ac3cc954a216856c0a0e81c5878cb68ad9ba4e10cecef2a9e65edf2f1d27  correction-renewal-red.log
09d7c8f7836f33e2da10849fc85552898f0865fb5cd529eedafd8355af700345  correction-integration-4.log
d52cfcb7566502041c615c34f3c87eac0ec8e6ad2f8812fc6161ef8a669c4796  correction-recovery-1.log
e4848714d019824f5b32613b56915039d73ba3f87b475c2396c315e2f2bfb10a  correction-recovery-2.log
03e30e9feb3c21fb12137c9b04567041a016ceec174ba3dc185f0438852e1f73  correction-after-mappings.log
c73598da8ee3fc74ab74a8819f025fe525a4b2c5e0393a8afa2350682415e8ce  correction-regression-1.log
dbdc19079fb40d74e7dc45aecb8f17e4d06fc61e40e977f72f15f06934232637  correction-crosscrate.log
7c1a51cdce1a24cc6ae95c5f0b518ff16cc5461b4d3880e4e4687681508df04f  correction-clippy-1.log
6f6b4f2bfb9fb7792bc72855a5a1796e72b7b04c530573a81998b901b09ecb64  correction-clippy-2.log
09f339fcea852355b37bb8c077b297b3f0179cf22aa3b72540a3f82df4624ef6  correction-contracts.log
f2732d1db6aeab2294dadbd9ed733dbfd1c017d329d0ccb41615c962d084c0ee  correction-fmt.log
f6900f5808bf8ba047379a039bb8fa45bc8d0e99abb8c564663a31024bc339f9  correction-ess-final.log
```
