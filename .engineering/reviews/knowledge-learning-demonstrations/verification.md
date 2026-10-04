# Verification

Executed native checks:

- `cargo test --locked -p ekr-kernel --test knowledge_learning_demonstrations -- --nocapture`
- `cargo test --locked -p ekr-kernel --test knowledge_retention --test completed_incubation_progress -- --nocapture`
- `cargo test --locked -p ekr --lib --test knowledge_cli -- --nocapture`
- `cargo test --locked -p ekr-store --lib application_audit -- --nocapture`
- Focused viewer historical-capture and held-session replacement regressions named in the logs below.

Each final demonstration loops over File and SQLite, reopens with full replay and compares all
published provider events on repeat. Both pass in runtime-final.log after the production corrections.
The full CLI run covers native signed review/application SDK dispatch, read-only viewer HTTP
boundaries and existing session replacement behavior. The focused store controls still refuse
unreadable application feeds. These are native tests, not ESS conformance attainment.

Before the behavioral health red, runtime-1 was a fixture compilation error and runtime-2 bound
the wrong upgrade statement. Viewer-progress-1 used an unsorted trust-scope fixture; viewer-progress-2
found absent application serialization, corrected by rendering retained records only when present.
The historical-capture red then reproduces false correction completion at the schema-only revision.
Clippy's first run found an unnecessary test Box allocation; the exact-value update removed it.
Broader CLI red reproduces application audit error classification hiding file-history divergence;
its green rerun restores all existing replacement cases without weakening feed availability.

Retained raw logs and browser captures are outside disposable compiler output. Paths in this
inventory are relative to that evidence root. The screenshot was generated with installed headless
Chrome and visually inspected for readable completion, revision and escaped source text. HTML
captures come from the both-provider native test. No live hosted UI or full gate is claimed.

Measured test summaries and SHA-256 digests:

```text
knowledge-learning-demonstrations/runtime-health-green.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 64.85s
knowledge-learning-demonstrations/runtime-3.log:test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 168.37s
schema-application-kernel/application-audit-controls.log:test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.04s
schema-application-kernel/application-audit-replacement-red.log:test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 69 filtered out; finished in 0.05s
schema-application-kernel/viewer-progress-capture-green.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 69 filtered out; finished in 19.07s
schema-application-kernel/application-audit-replacement-green.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 69 filtered out; finished in 0.05s
knowledge-learning-demonstrations/runtime-final.log:test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 164.07s
knowledge-learning-demonstrations/cli-regressions-2.log:test result: ok. 70 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.06s
knowledge-learning-demonstrations/cli-regressions-2.log:test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
schema-application-kernel/viewer-progress-capture-red.log:test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 69 filtered out; finished in 11.13s
schema-application-kernel/viewer-progress-3.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 69 filtered out; finished in 3.66s
knowledge-learning-demonstrations/cli-regressions.log:test result: FAILED. 67 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.43s
knowledge-learning-demonstrations/completed-progress-green.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.37s
knowledge-learning-demonstrations/projection-regressions.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.15s
knowledge-learning-demonstrations/projection-regressions.log:test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
knowledge-learning-demonstrations/completed-progress-red.log:test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.37s
f0fc0aaa5de7359937b95ac19413dc7801c7e5f819bbe277fb6c78639e499f1c  knowledge-learning-demonstrations/runtime-1.log
01d8adbec30ddd9a0655b3b889fb1386d07fc17bc78a5b2ba1a8eda8349411bb  knowledge-learning-demonstrations/runtime-2.log
8ea1720a8910835ba244058d2c223a34b84517c589f522ecb1e7fac87a5f35a5  knowledge-learning-demonstrations/runtime-3.log
b05b75fe5bf4dd85d0caebc2b3c2b75a1ef9b0d1ca4ef48819114cee38a293ed  knowledge-learning-demonstrations/runtime-health-green.log
23162c215791b23fd5c8966f3c1dbaca319230bbed5360a69cf24cafbca4e28e  knowledge-learning-demonstrations/completed-progress-red.log
bd0c4b7d7b48f3982529c8c550a7cabb62b6bd77a8b83d8db981da1bc23d6f06  knowledge-learning-demonstrations/completed-progress-green.log
f6aa5246e22058ead71ba0735ccb40b9696d0d1cf9bdf35870d96693285afb09  knowledge-learning-demonstrations/projection-regressions.log
9d90fee192924ca665a9166088b966bc786db01902e31d7008d5326065dd1b91  knowledge-learning-demonstrations/cli-regressions.log
5f1df722a4e32aa774b1e53013aa3caa7e9664130aa4da4f1b40d9568a189e8b  knowledge-learning-demonstrations/cli-regressions-2.log
d781669a209b2c2aba86bf0e000d56e4bf1fa86345c2a69b720d62e5342adc39  knowledge-learning-demonstrations/runtime-final.log
bab7fe7f4f816177562a50eadd2eff651a973e425ef2ee652f88cd69efee7b60  knowledge-learning-demonstrations/clippy.log
558193d6e7b785b5d9798e70b6aa0cf301fea96a71502c42892968bf6938d18a  knowledge-learning-demonstrations/clippy-2.log
4437c8925bedbce51958b2ad464157375b551efbb7b434bccd6bd5c32388a66f  schema-application-kernel/viewer-progress-1.log
05ba51058364d256bf5505aedba865987c20e961c785237e38acee196aea2646  schema-application-kernel/viewer-progress-2.log
b9602c5376004d90f834cfc1f0d741211ff076249573bb66409a2104153a99d6  schema-application-kernel/viewer-progress-3.log
9abc47e4da6a7bdfbcbfb24cd7a0ffc4041bc2fd306d91f57a603871202a16db  schema-application-kernel/viewer-progress-capture-red.log
eeab26b47c492d3a031e45ca6ba049304f0194974990c014c6aa452b8a9886e4  schema-application-kernel/viewer-progress-capture-green.log
b6e693a2ad9ebf5839fbaa728f22c72c02f41c173a919fcef37f6ff40aa9afb0  schema-application-kernel/application-audit-replacement-red.log
c9387ddd2b388096f5bcc62571eb8b1e40e0e4fb806a63c56bb6dc2b1cfb3781  schema-application-kernel/application-audit-replacement-green.log
5b5fc5d35e98bdeb0abd8507572fd72df80cb5e877994bd6cd5d04b96df4aec9  schema-application-kernel/application-audit-controls.log
d0c0921252bce43ce3240f0c38f617931d96a64a801a179adf2d6cc55cb0013e  schema-application-kernel/viewer-progress/file-applied-proposal.html
150181333d05d287383b5a04c335b68a53a510c6ea4bbdb233038ac1de7c7e87  schema-application-kernel/viewer-progress/sqlite-applied-proposal.html
12ad9e51d0901d2e40d0d8a35346049dac98506826da23e34bbc180c1e9f9348  schema-application-kernel/viewer-progress/applied-proposal.png
```

Final auxiliary checks exit zero: `cargo clippy --locked -p ekr-store -p ekr-kernel -p ekr --all-targets -- -D warnings`, repository `xtask fmt --check`, ESS validation and AEP validation. AEP reports 497 artifacts and five pre-existing prose-only review warnings. Specification/generated files are unchanged from the parent; no regeneration or new full conformance claim is made.

```text
8b5cb1d85544438de56c4ff687fc743d6ee0cf6536c68a253ce41a0384dcad80  knowledge-learning-demonstrations/clippy-final.log
c08afffae7afbcd449659f281950c64757a324806db7043c8f9cde253bd74456  knowledge-learning-demonstrations/fmt.log
f6900f5808bf8ba047379a039bb8fa45bc8d0e99abb8c564663a31024bc339f9  knowledge-learning-demonstrations/ess.log
fdb6b858aaede785f2cb053cffba79b4a4f6d6caa9ec41cd26e0b943605783e3  knowledge-learning-demonstrations/aep-final.log
40c6845337321e72459a667f05083e329ea85bbfb9ce6811a61a6e89d13ef898  knowledge-learning-demonstrations/demonstrations.patch
a7b4adae93f80ace6097e161774480c80f1c01b49d24ec0d1165ed69f10886aa  knowledge-learning-demonstrations/report.md
fdb0e4b79309bcc48c7967181fe9f2d1cff3af5c5e73f8a4e58d0faa1ab99fcb  knowledge-learning-demonstrations/completed-progress-review-r1.md
0547d2d51abe5b15bae798946fb55ae929d79e7a489960b3461de8e21d9d8336  schema-application-kernel/viewer-progress-review-r1.md
b23f103119b46c3373316bfbfc358847dea0cdebeb6e9488377293ac635b29d4  schema-application-kernel/viewer-progress-review-r2.md
33889c12ebece40e5daa23f8e0289b0c1c8acad35a048980d565e9520f68c5d5  schema-application-kernel/application-audit-replacement-review-r1.md
```
