# Completed proposal attention

The existing ESS ListAttention contract excludes settled questions. A real schema-only
application still appeared as an approval question after cold full replay. The focused red
reproduces that behavior. The projection now consults its captured authenticated application
prefix and retires a proposal only when schema, selected mappings and selected corrections
have actually committed. Approval alone and retained progress receipts cannot settle it.

Approved incomplete proposals ask about integration or renewed review. Original review and
application histories remain available. The expanded focused regression loops over both
providers with complete schema-only and unresolved-correction applications; it preserves
another pending proposal, checks completed handles retire, and verifies reads append no events.
Existing exact-review regressions also pass. No new model or generated artifact is introduced.

Independent source/log review: review-result:completed-proposal-attention-r1. This bounded
follow-up closes the completed-proposal limitation noted in the preceding checkpoint. A real
partial multi-mapping inbox scenario is not added by this test; full F and the repository gate
remain outstanding. ESS416's separate generated-fixture blocker remains open.

Executed commands: cargo test --locked -p ekr-kernel --test schema_proposal_attention
--test schema_proposal_reviews -- --nocapture; strict kernel all-target Clippy; repository fmt.
Retained logs below live under schema-application-kernel outside disposable build outputs.

```text
completed-proposal-attention-red.log:test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
completed-proposal-attention-green.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.38s
completed-proposal-attention-final.log:test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.38s
completed-proposal-attention-final.log:test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
6f78e8252adefd5743956b89c4697ae832ffbde28c0faf98d8f3e10772c70bf4  completed-proposal-attention-red.log
d78f5efeb247b838271362eb03a0d5d555efbf4f59a5432d62e0c10c3eb323c7  completed-proposal-attention-green.log
0f16c82092bd61578695c015e993a3325b54a6ea7b2c27d1b25020b78a86c754  completed-proposal-attention-final.log
d90c7237d44bde7b7cc10f895199980ee923aea191db8a8a707677d781658940  completed-proposal-attention-clippy.log
4bcefa3bdec93b4f80f67ba754eec254173b37ed25a49d2436bd51e4c93600b5  completed-proposal-attention-fmt.log
977a1f5bb731a9c4e6c6b6c1851367af28e881771bb16deee9254e4bc334e944  completed-proposal-attention-review-r1.md
```
