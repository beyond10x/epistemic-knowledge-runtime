//! The amended `architecture-decision-record:0005-float-is-not-canonical` makes the membrane a
//! property of the type system. Two claims it now rests on, held against the source.
//!
//! * **"The address exists exactly where canonical state does"** — `crates/ekr-graph/src/lib.rs`
//!   and the ADR's amendment. `tests/compile_fail/transient_state_has_no_content_address.rs` holds
//!   one half of *exactly*: transient state has no address. Nothing holds the other half, that
//!   canonical state has one. [`CanonicalGraph`](ekr_graph::CanonicalGraph) holds four maps of
//!   entities, and `Root.evidence_root` is the address of the fourth.
//! * **`CanonicalDependency` is "implemented for `CanonicalRef` and for nothing else, and sealed
//!   so that it stays that way"** — `crates/ekr-graph/src/canonical.rs`. The seal is what makes
//!   that true outside the crate; inside it, a type that carries the seal is one line from the
//!   trait. `canonical.rs` already treats a shared marker as a defect worth a second trait for,
//!   because `tests/compile_fail/canonical_dependency_is_sealed.stderr` prints the marker's
//!   implementors to a reader as the sealed trait's.
//!
//! Both are read from the source rather than from a trait bound, because a missing implementation
//! is a compile error at a use site and a compile error is not a case anybody can run.

use std::path::Path;

/// Every `.rs` file of `ekr-graph`'s `src/`, by file name, sorted.
fn crate_modules() -> Vec<(String, String)> {
    let directory = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut found: Vec<(String, String)> = std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| {
            (
                path.file_name()
                    .expect("a file has a name")
                    .to_string_lossy()
                    .into_owned(),
                std::fs::read_to_string(&path).expect("a source file"),
            )
        })
        .collect();
    found.sort();
    assert!(
        found.len() >= 10,
        "the module scan is broken, not the crate"
    );
    found
}

/// The type named after `head` on each line that carries it, without its generic arguments.
///
/// `impl<T: CanonicalTarget> sealed::Sealed for LocalRef<T> {}` with head `sealed::Sealed for `
/// gives `LocalRef`.
fn implemented_for(source: &str, head: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("impl"))
        .filter_map(|line| line.split(head).nth(1))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .filter(|name: &String| !name.is_empty())
        .collect()
}

/// The names, sorted and deduplicated, that `head` is implemented for across the crate.
fn implementors(head: &str) -> Vec<String> {
    let mut found: Vec<String> = crate_modules()
        .iter()
        .flat_map(|(_, source)| implemented_for(source, head))
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The seal on `CanonicalDependency` is carried by exactly the types that are one.
///
/// `CanonicalDependency`'s doc says it is "implemented for `CanonicalRef` and for nothing else,
/// and sealed so that it stays that way", and `sealed::Sealed` is the only thing that makes the
/// second half true. A type that carries the seal and not the trait is one line inside this crate
/// from canonical state accepting it — and this crate is where the membrane is defined, so that is
/// where the line would be written.
///
/// It is also what a reader is told. `canonical.rs` introduced a *second* marker,
/// `sealed::SealedTarget`, for exactly this reason, in its own words: one marker shared between two
/// public traits "made rustc's own 'the following types implement the trait' list on a sealing
/// error name the entities as implementors of `CanonicalDependency`, which they are not. The
/// compile-fail cases exist to be *read*, so the message they pin has to be true."
/// `tests/compile_fail/canonical_dependency_is_sealed.stderr` still ends with that list, and it
/// still names a type that does not implement the trait.
#[test]
fn the_seal_of_canonical_dependency_is_carried_only_by_a_canonical_dependency() {
    let sealed = implementors("sealed::Sealed for ");
    let dependencies = implementors("CanonicalDependency for ");

    assert_eq!(
        sealed, dependencies,
        "a type carries the seal of CanonicalDependency without being one, so the seal's own \
         diagnostic — pinned verbatim in tests/compile_fail/canonical_dependency_is_sealed.stderr \
         — lists it to a reader as an implementor of a trait it does not implement, and nothing \
         inside this crate stops the impl being written"
    );
}

/// Every entity canonical state holds has a content address.
///
/// The amendment's claim is that the address exists *exactly* where canonical state does.
/// `CanonicalGraph` holds four maps — nodes, edges, assertions and evidence — and design § 34
/// gives `Root` a sub-root over each storage class, `evidence_root` among them
/// (`crates/ekr-graph/src/root.rs:59`). The compile-fail case added this round holds one direction
/// of *exactly*: that transient state has no address. This is the other direction, and nothing
/// held it.
#[test]
fn every_entity_canonical_state_holds_has_a_content_address() {
    let modules = crate_modules();
    let missing: Vec<&str> = ["Node", "Edge", "Assertion", "Evidence"]
        .into_iter()
        .filter(|type_name| {
            !modules
                .iter()
                .any(|(_, text)| canonical_impl_head(text, type_name).is_some())
        })
        .collect();

    assert!(
        missing.is_empty(),
        "CanonicalGraph holds nodes, edges, assertions and evidence, and {missing:?} has no \
         Canonical implementation, so Root.evidence_root is not computable from evidence state \
         for the same reason Root.knowledge_root was not computable from graph state before this \
         round — and the same argument that landed Node and Edge here, that unit 1 must not have \
         to edit this crate, applies to it unchanged"
    );
}

/// The `{` that opens `impl … Canonical for <type_name> …`, whatever bounds the implementation
/// carries.
///
/// **Structural, and deliberately not a list of spellings.** The list was two —
/// `impl Canonical for Root {` and `impl<V: Canonical> Canonical for Node<V> {` — and
/// `architecture-decision-record:0008-canonical-state-references-are-typed` added a third,
/// `impl<V: ValueSpace + Canonical> Canonical for Edge<V> {`, at which point a scan enumerating
/// spellings reported the type as having *no implementation at all*. A rule enumerated by its
/// instances has a next instance; this one reads the shape — a line beginning `impl`, naming
/// `Canonical for` the type at an identifier boundary, and opening a block.
fn canonical_impl_head(source: &str, type_name: &str) -> Option<usize> {
    let needle = format!(" Canonical for {type_name}");
    source.match_indices(&needle).find_map(|(at, _)| {
        let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
        if !source[line_start..at].trim_start().starts_with("impl") {
            return None;
        }
        // The next character after the name is what keeps `Node` from matching `NodeDraft`.
        if !source[at + needle.len()..].starts_with(['<', ' ', '{']) {
            return None;
        }
        let line_end = source[at..]
            .find('\n')
            .map_or(source.len(), |offset| at + offset);
        source[at..line_end].rfind('{').map(|offset| at + offset)
    })
}
