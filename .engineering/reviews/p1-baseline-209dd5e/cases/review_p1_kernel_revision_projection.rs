//! Independent review of the P1 core: `ekr.kernel.Revision` against the Rust type the store writes.
//!
//! `systems/ekr/domains/kernel.yaml` declares `ekr.kernel.Revision` — the entity the P1 exit
//! criterion ("replay from the seed reproduces the root hash") is about — and `ekr_graph::Root`
//! (`crates/ekr-graph/src/root.rs`) is what `ekr-store`'s fold produces and `head()` returns.
//! `crates/ekr-graph/tests/domain_projection.rs` binds every `graph.yaml` declaration to a Rust
//! type or lists it as unbound; nothing does that for `kernel.yaml`'s entities, so this case does
//! it for the one entity a lineage is made of, by reading both documents at run time.
//!
//! The repository is located through `std::env::var("CARGO_MANIFEST_DIR")` at **run time** — the
//! value `cargo test` sets for the process — and not `env!`, which is baked in at compile time and
//! is the hazard `task:guards-read-source-through-a-compile-time-path` records. That is the
//! cheapest fix for the nineteen files that carry the pattern, and this file is it, once.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// The workspace root, from the manifest directory cargo hands this process when it runs it.
fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    PathBuf::from(manifest_dir)
        .ancestors()
        .nth(2)
        .expect("crates/ekr sits two levels below the workspace root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// The identity field and every declared field of one entity of an ESS domain document.
fn entity_fields(domain: &str, entity: &str) -> BTreeSet<String> {
    let header = format!("  - name: {entity}");
    let mut lines = domain.lines().skip_while(|line| line.trim_end() != header);
    assert!(lines.next().is_some(), "{entity} is declared by the domain");
    let mut fields = BTreeSet::new();
    let mut section = "";
    for line in lines {
        // The next entity at the same indent ends this one.
        if line.starts_with("  - name: ") {
            break;
        }
        match line.trim() {
            "identity:" => section = "identity",
            "fields:" => section = "fields",
            "relations:" | "lifecycle:" | "invariants:" => section = "",
            _ => {}
        }
        let trimmed = line.trim();
        let name = match section {
            "identity" => trimmed.strip_prefix("name: "),
            "fields" => trimmed.strip_prefix("- name: "),
            _ => None,
        };
        if let Some(name) = name {
            fields.insert(name.to_owned());
        }
    }
    fields
}

/// Every `pub` field of one struct in a Rust source file.
fn struct_fields(source: &str, name: &str) -> BTreeSet<String> {
    let opener = format!("pub struct {name} {{");
    let body = source
        .split_once(opener.as_str())
        .unwrap_or_else(|| panic!("{name} is declared"))
        .1;
    let body = body.split_once("\n}").expect("the struct closes").0;
    body.lines()
        .filter_map(|line| line.trim().strip_prefix("pub "))
        .filter_map(|rest| rest.split_once(':'))
        .map(|(field, _)| field.trim().to_owned())
        .collect()
}

#[test]
fn ekr_kernel_revision_and_ekr_graph_root_declare_the_same_fields() {
    let declared = entity_fields(
        &read("systems/ekr/domains/kernel.yaml"),
        "ekr.kernel.Revision",
    );
    let carried = struct_fields(&read("crates/ekr-graph/src/root.rs"), "Root");
    assert!(
        declared.len() >= 5 && carried.len() >= 5,
        "the scans are broken, not the documents: {declared:?} / {carried:?}"
    );

    let missing: Vec<&String> = declared.difference(&carried).collect();
    let extra: Vec<&String> = carried.difference(&declared).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "ekr.kernel.Revision and ekr_graph::Root are two descriptions of one record and they \
         disagree. Declared by the domain and carried by no field of Root: {missing:?}. Carried by \
         Root and declared by no field of the domain: {extra:?}. The domain gives the revision an \
         identity, an optional transaction id and a committed_at; Root has no id, a required \
         transaction *hash* that at the seed is the address of a JSON document, and no instant. \
         Root follows design § 34; the domain does not, and no projection guard reads kernel.yaml."
    );
}
