---
format: aep.planning-md/1
id: task:trybuild-stderr-is-toolchain-pinned
kind: task
status: draft
title: The compile-fail expectations are pinned to one rustc's diagnostic wording
revision: 1
---
## What is wrong

`crates/ekr-graph/tests/compile_fail/*.stderr` was generated with `TRYBUILD=overwrite` on the
toolchain present on 2026-09-21. `trybuild` compares compiler output byte for byte, and rustc
rewords diagnostics between releases, so these files go red on a toolchain change for a reason that
has nothing to do with the membrane they exist to prove.

The workspace pins `rust-version` in `Cargo.toml`, which is a minimum rather than an exact
toolchain, and there is no `rust-toolchain.toml`.

## What closes this

One of: a `rust-toolchain.toml` pinning the exact version the gate runs, so the `.stderr` files
have a fixed target; or a trybuild configuration that matches on the error code rather than the
full text; or a documented step that regenerates them on a toolchain bump. Pick one and say why in
the commit.

Until then, a red `membrane` lane after a toolchain change is to be read as this task before it is
read as a regression.
