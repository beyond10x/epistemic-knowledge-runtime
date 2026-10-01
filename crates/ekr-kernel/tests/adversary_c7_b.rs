//! Adversarial cases for wave correct-07 unit B (`task:seed-document-bounds-alias-expansion`):
//! the seed document's size, depth and alias-expansion limits, driven through
//! `SeedDocument::from_yaml` with node and byte counts written out by hand.

use ekr_kernel::{SeedDocument, SeedError, SEED_LIMITS};

fn fixture(name: &str) -> String {
    let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

/// The reason of a refused seed, which starts with its code.
fn reason(text: &str) -> String {
    match SeedDocument::from_yaml(text) {
        Err(SeedError::Invalid(reason)) => reason,
        other => panic!("expected a named seed refusal, got {other:?}"),
    }
}

fn code(text: &str) -> String {
    let reason = reason(text);
    reason.split(':').next().unwrap().to_owned()
}

/// A flow sequence of `count` copies of `item`.
fn list(item: &str, count: usize) -> String {
    format!("[{}]", vec![item; count].join(", "))
}

/// Block style counts depth as flow style does: the root mapping is one, a document exactly at
/// `SEED_LIMITS.depth` reaches the decoder and one container more is `seed-too-deep`.
#[test]
fn block_style_depth_is_counted_like_flow_style_at_the_boundary() {
    let seed = fixture("seed-minimal-v2.yaml");
    let depth = SEED_LIMITS.depth;
    // Compact block sequences: `deep:` then `- - … x`, each `- ` one sequence.
    let sequences = |count: usize| format!("{seed}deep:\n  {}x\n", "- ".repeat(count));
    assert_eq!(code(&sequences(depth - 1)), "seed-decode");
    assert_eq!(code(&sequences(depth)), "seed-too-deep");
    // Block mappings, one space deeper per level: `deep:` then `k:` lines, the last `k: x`.
    let mappings = |count: usize| {
        let mut text = format!("{seed}deep:\n");
        for level in 1..count {
            text.push_str(&format!("{}k:\n", " ".repeat(level)));
        }
        text.push_str(&format!("{}k: x\n", " ".repeat(count)));
        text
    };
    assert_eq!(code(&mappings(depth - 1)), "seed-decode");
    assert_eq!(code(&mappings(depth)), "seed-too-deep");
    // Flow mappings.
    let flow = |count: usize| {
        format!(
            "{seed}deep: {}x{}\n",
            "{k: ".repeat(count),
            "}".repeat(count)
        )
    };
    assert_eq!(code(&flow(depth - 1)), "seed-decode");
    assert_eq!(code(&flow(depth)), "seed-too-deep");
}

/// An anchored container repeated so that it reaches exactly the depth limit is not refused for
/// depth; one level deeper is.
#[test]
fn depth_through_an_alias_is_exact_at_the_boundary() {
    let seed = fixture("seed-minimal-v2.yaml");
    let half = SEED_LIMITS.depth / 2;
    let text = |around: usize| {
        format!(
            "{seed}shallow: &deep {}{}\ndeep: {}*deep{}\n",
            "[".repeat(half),
            "]".repeat(half),
            "[".repeat(around),
            "]".repeat(around)
        )
    };
    // Root 1 + `around` + the anchored node's `half`.
    assert_eq!(code(&text(half - 1)), "seed-decode");
    assert_eq!(code(&text(half)), "seed-too-deep");
}

/// The node limit is inclusive and counted exactly: a document that expands to exactly
/// `SEED_LIMITS.expanded_nodes` values, keys and containers is not refused for expansion, and one
/// more node is, at the count it reached.
#[test]
fn the_node_limit_is_exact() {
    assert_eq!(SEED_LIMITS.expanded_nodes, 33_554_432);
    // root 1; key a 1; the anchored sequence 1 + 4095 empty sequences = 4096; key b 1; sequence b
    // 1; 8190 aliases x 4096; key c 1; sequence c 1; `padding` empty sequences.
    let text = |padding: usize| {
        format!(
            "a: &a {}\nb: {}\nc: {}\n",
            list("[]", 4095),
            list("*a", 8190),
            list("[]", padding)
        )
    };
    let at = 1 + 1 + 4096 + 1 + 1 + 8190 * 4096 + 1 + 1 + 4090;
    assert_eq!(at, 33_554_432);
    assert_eq!(code(&text(4090)), "seed-decode", "{}", reason(&text(4090)));
    let over = reason(&text(4091));
    assert!(over.starts_with("seed-alias-expansion: "), "{over}");
    assert!(over.contains("(at least 33554433)"), "{over}");
}

/// The text limit is inclusive and counts UTF-8 bytes, not characters: an anchored scalar of 2048
/// two-byte characters is 4096 bytes, and a document repeating it to exactly
/// `SEED_LIMITS.expanded_text_bytes` passes while one byte more is refused.
#[test]
fn the_text_limit_is_exact_and_counts_multibyte_scalars_in_bytes() {
    assert_eq!(SEED_LIMITS.expanded_text_bytes, 16_777_216);
    let wide = "\u{e9}".repeat(2048);
    assert_eq!(wide.len(), 4096);
    // keys a, b, c 3; the anchored scalar 4096; 4094 aliases x 4096; c's scalar `tail` bytes.
    let text = |tail: usize| {
        format!(
            "a: &a \"{wide}\"\nb: {}\nc: \"{}\"\n",
            list("*a", 4094),
            "x".repeat(tail)
        )
    };
    assert_eq!(3 + 4096 + 4094 * 4096 + 4093, 16_777_216);
    assert_eq!(code(&text(4093)), "seed-decode", "{}", reason(&text(4093)));
    let over = reason(&text(4094));
    assert!(over.starts_with("seed-alias-expansion: "), "{over}");
    assert!(over.contains("(at least 16777217)"), "{over}");
}

/// A merge key does not dodge the charge: `<<: *b` repeats everything `b` holds.
#[test]
fn merge_keys_are_charged_as_the_alias_they_are() {
    let seed = fixture("seed-minimal-v2.yaml");
    // b: map 1 + key 1 + sequence 1 + 4096 = 4099 nodes; 9000 merges = 36,891,000.
    let text = format!(
        "{seed}base: &b {{k: {}}}\nmerged: {}\n",
        list("[]", 4096),
        list("{<<: *b}", 9000)
    );
    assert_eq!(code(&text), "seed-alias-expansion", "{}", reason(&text));
    // A merge chain doubling through the merge-sequence form.
    let mut chain = format!("{seed}l0: &l0 {{a: x, b: x}}\n");
    for level in 1..=40 {
        let previous = level - 1;
        chain.push_str(&format!(
            "l{level}: &l{level} {{<<: [*l{previous}, *l{previous}]}}\n"
        ));
    }
    assert_eq!(code(&chain), "seed-alias-expansion", "{}", reason(&chain));
}

/// A redefined anchor binds later aliases to its latest node: redefining a small anchor as a large
/// one is charged as the large one, and the reverse is charged as the small one.
#[test]
fn a_redefined_anchor_is_charged_as_the_node_it_now_names() {
    let seed = fixture("seed-minimal-v2.yaml");
    // 9000 aliases of a 4097-node sequence: 36,873,000.
    let dodge = format!(
        "{seed}x: &a []\ny: &a {}\nz: {}\n",
        list("[]", 4096),
        list("*a", 9000)
    );
    assert_eq!(code(&dodge), "seed-alias-expansion", "{}", reason(&dodge));
    let reverse = format!(
        "{seed}x: &a {}\ny: &a []\nz: {}\n",
        list("[]", 4096),
        list("*a", 9000)
    );
    assert_eq!(code(&reverse), "seed-decode", "{}", reason(&reverse));
}

/// Aliases to scalars and aliases in key position are charged like any other.
#[test]
fn scalar_aliases_and_aliases_as_keys_are_charged() {
    let seed = fixture("seed-minimal-v2.yaml");
    // A 1,000,000-byte scalar repeated 20 times: 21,000,000 bytes of text.
    let scalars = format!(
        "{seed}s: &s {}\nt: {}\n",
        "x".repeat(1_000_000),
        list("*s", 20)
    );
    assert_eq!(
        code(&scalars),
        "seed-alias-expansion",
        "{}",
        reason(&scalars)
    );
    // A 4097-node key repeated in 9000 single-entry mappings.
    let keys = format!(
        "{seed}k: &k {}\nm: {}\n",
        list("[]", 4096),
        list("{? *k : x}", 9000)
    );
    assert_eq!(code(&keys), "seed-alias-expansion", "{}", reason(&keys));
}

/// A recursive alias keeps its earlier code, `seed-decode`, in a sequence, a mapping and nested
/// below its anchor.
#[test]
fn a_recursive_alias_stays_seed_decode() {
    let seed = fixture("seed-minimal-v2.yaml");
    for recursive in ["r: &r [*r]\n", "r: &r {k: *r}\n", "r: &r [[[*r]]]\n"] {
        let text = format!("{seed}{recursive}");
        assert_eq!(code(&text), "seed-decode", "{recursive}: {}", reason(&text));
    }
}

/// A later document of the input is held to the same bounds: a deep second document is refused
/// as too deep rather than loaded in full.
#[test]
fn a_second_document_is_bounded_too() {
    let seed = fixture("seed-minimal-v2.yaml");
    let open = SEED_LIMITS.depth + 20_000;
    let deep = format!("{seed}---\n{}{}\n", "[".repeat(open), "]".repeat(open));
    assert_eq!(code(&deep), "seed-too-deep", "{}", reason(&deep));
    let mut bomb = format!("{seed}---\nl0: &l0 [ha, ha]\n");
    for level in 1..=40 {
        let previous = level - 1;
        bomb.push_str(&format!(
            "l{level}: &l{level} [*l{previous}, *l{previous}]\n"
        ));
    }
    assert_eq!(code(&bomb), "seed-alias-expansion", "{}", reason(&bomb));
}

/// The repository's seed fixtures load as before: the v2 fixture decodes, and the v1 fixture is
/// refused for its version, not by one of the new limits.
#[test]
fn the_repository_seed_fixtures_are_not_refused_by_the_new_limits() {
    SeedDocument::from_yaml(&fixture("seed-minimal-v2.yaml")).unwrap();
    if let Err(SeedError::Invalid(reason)) = SeedDocument::from_yaml(&fixture("seed-minimal.yaml"))
    {
        for new in ["seed-too-large", "seed-too-deep", "seed-alias-expansion"] {
            assert!(!reason.starts_with(new), "{reason}");
        }
    }
}

/// `docs/cli.md`: "The expansion limits are what a document of the byte cap could hold written
/// out, so an alias can save writing but cannot make a seed decode to more than that", and the
/// `seed-alias-expansion` row: "YAML aliases make the document decode to more than ...". A
/// document under the byte cap with no alias at all must not be refused as alias expansion. The
/// escape `\L` is two bytes of input and decodes to U+2028, three bytes of text.
#[test]
#[ignore = "adversary c7-b F1: an alias-free seed under the byte cap is refused as seed-alias-expansion (\\L decodes 2 bytes to 3)"]
fn an_alias_free_seed_under_the_byte_cap_is_not_refused_as_alias_expansion() {
    let seed = fixture("seed-minimal-v2.yaml");
    let text = format!("{seed}x: \"{}\"\n", "\\L".repeat(6_000_000));
    assert!(text.len() < SEED_LIMITS.input_bytes);
    assert!(!text.contains('&') && !text.contains('*'));
    let reason = reason(&text);
    assert!(!reason.starts_with("seed-alias-expansion"), "{reason}");
}
