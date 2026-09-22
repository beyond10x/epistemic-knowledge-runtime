# serde_yaml_ng provenance and local changes

This directory vendors the published `serde_yaml_ng` **0.10.0** crate from
crates.io. Its package checksum is
`7b4db627b98b36d4203a7b458cf3573730f2bb591b28871d916dfa9efabfd41f`.
The package's VCS metadata identifies commit
`45bce7bd7efb754ffc9dc910f3d1bae7edd0adf4` in
<https://github.com/acatton/serde-yaml-ng>, with an empty `path_in_vcs`.
The original MIT license, README, normalized manifest and `Cargo.toml.orig` are
retained. Registry cache markers and upstream CI configuration are omitted.

## Observation facade

- `src/observation.rs` adds an opt-in safe view of the existing loader's buffered
  events: decoded tags, scalar text and classification, containers, resolved alias
  positions and retained document errors. It does not expand aliases or replace
  the loader or scalar resolver. Callers must bound input and traversal.
- `src/lib.rs` exports that module.
- `src/de.rs` makes the existing `visit_scalar` and `parse_tag` helpers visible
  within the crate so the facade shares their behavior.
- `tests/test_observation.rs` exercises the facade alongside direct decoding,
  including tags and directives, scalar categories, collection tags, alias
  positions, multiple documents and loader errors.

Normal Serde and `Value` decoding remain unchanged. In particular, scalar
classification through `deserialize_any` can differ from decoding a requested
Rust String or number; observing a category does not override that behavior.

## Compiler compatibility changes

The unchanged package failed under the repository's Rust 1.98.1 compiler with
nine `dangerous_implicit_autorefs` diagnostics and eight lifetime spelling
warnings. These mechanical changes implement the compiler's suggestions:

- `src/libyaml/error.rs`: eight raw-pointer field accesses spell the existing
  implicit shared reference explicitly as `(&*pointer).field`.
- `src/libyaml/parser.rs`: the parser-error field access spells the same shared
  reference explicitly.
- `src/mapping.rs`: the return types of `entry`, `iter`, `iter_mut`, `keys`,
  `values` and `values_mut` spell their elided lifetime as `'_`.
- `src/number.rs` and `src/value/de.rs`: each `unexpected` return type spells
  its elided lifetime as `'_`.

No unsafe preconditions, assertions or existing tests are changed. No lint
allowance is added to the source or build configuration.

## Locked compatibility checks

The original standalone manifest is unchanged. A standalone `Cargo.lock` pins
its test dependencies independently of the consuming workspace. The root
workspace excludes this directory from membership and selects it with a local
`patch.crates-io` entry. The root lock changes only this package's source; its
version and dependency list remain unchanged.

Run the original upstream suite and the additional observation tests with:

```sh
cargo test --offline --locked --manifest-path vendor/serde_yaml_ng/Cargo.toml
```

All original test files are retained byte for byte. Their existing formatting
in `tests/test_de.rs`, and the existing formatting in `src/with.rs`, differ from
Rust 1.98.1 rustfmt output; those unrelated bytes are deliberately retained.
The local facade and its tests are formatted with the repository's toolchain.
