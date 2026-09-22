# Retained review evidence at 209dd5e

This directory preserves the second independent review, its observed probe output, the
probe source, and the deferred cases copied out of the deleted review checkout. The first
review is `review-result:independent-review-p1-core` in the planning store.

The probe manifest uses repository-relative paths and is a standalone Cargo workspace:

```console
cargo run --manifest-path .engineering/reviews/p1-baseline-209dd5e/probe/Cargo.toml --offline
```

The recorded output belongs to the reviewed commit. It is not a claim about a later tree.
The probe may require adaptation when the public kernel API changes; preserve the original
here and put runnable regression cases in the owning crate's test suite.

The five cases under `cases/` are historical acceptance inputs, not an executed test lane.
Three specify seed refusals, one checks validation hash domain separation, and one asserts
that two revision representations have identical field names. Review the last assertion
against the intended metadata/root distinction before adopting it: matching names alone
does not establish a correct projection. Every promoted case must establish a behavior
that remains meaningful after the repair.

Do not treat a source-reading guard as a substitute for executing the real kernel and both
storage providers.
