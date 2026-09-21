//! What this crate says about `systems/ekr/domains/graph.yaml`, checked against the document.
//!
//! `ekr-ontology` has a case of this shape, and it exists because two review passes each found a
//! different false sentence in the gap between the ESS document and the crate that claims to
//! implement it: `ess specify validate` checks the document against itself and the rest of the
//! suite checks the crate against itself, so nothing reads the two together. `ekr-graph` projects
//! more of a domain than any crate before it, so it gets the same case.
//!
//! `ekr-graph` declares no YAML parser — its dependencies are `ekr-core`, `ekr-ontology`, `serde`
//! and `thiserror`, fixed by `story:workspace-crate-skeleton` — so the document is read as text.
//! The scan is held to its own catch: a parse that stops finding declarations fails loudly rather
//! than passing vacuously.

/// `systems/ekr/domains/graph.yaml`, as text.
fn domain_text() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../systems/ekr/domains/graph.yaml"
    );
    std::fs::read_to_string(path).expect("the ESS domain is beside the crates")
}

/// The variants one `kind: enum` type of the domain declares, in document order.
fn variants_of(type_name: &str) -> Vec<String> {
    let text = domain_text();
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_start() != format!("- name: {type_name}"));
    assert!(
        lines.next().is_some(),
        "the domain declares no type named {type_name}"
    );

    let mut variants = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("- name: ") {
            break;
        }
        if trimmed == "variants:" {
            inside = true;
            continue;
        }
        if inside {
            if let Some(variant) = trimmed.strip_prefix("- ") {
                variants.push(variant.to_owned());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
        }
    }
    assert!(
        !variants.is_empty(),
        "the variant scan is broken, not the domain: {type_name}"
    );
    variants
}

/// Every declaration of the domain that carries a `fields:` block, by name, in document order.
///
/// Derived rather than listed. The first version of the entity guard below carried a hand-written
/// list of five, and the two it left out — `ekr.graph.Assertion` and `ekr.graph.Evidence` — were
/// exactly the two whose projection diverges, so the case was titled for the whole domain and
/// checked the easy half of it. A list a person maintains is the defect; the omission is only its
/// symptom.
fn declarations_with_fields() -> Vec<String> {
    let text = domain_text();
    let mut found = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        // Only a qualified name heads a declaration; `- name: kind` inside a `fields:` block is a
        // field, and reading it as a declaration is how the scan would silently drift.
        if let Some(name) = trimmed.strip_prefix("- name: ekr.") {
            current = Some(format!("ekr.{name}"));
        } else if trimmed == "fields:" {
            if let Some(name) = current.take() {
                found.push(name);
            }
        }
    }
    assert!(
        found.len() >= 8,
        "the declaration scan is broken, not the domain: {found:?}"
    );
    found
}

/// The field names one declaration of the domain carries, in document order.
fn fields_of(name: &str) -> Vec<String> {
    let text = domain_text();
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_start() != format!("- name: {name}"));
    assert!(lines.next().is_some(), "the domain declares no {name}");

    let mut fields = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
        // The next declaration ends this one. Checked before the field pattern, because a
        // declaration head *is* a `- name:` line and reading it as a field is how the scan drifts:
        // `ekr.graph.TypedValue` sits under `types:` and is followed straight by `entities:`, with
        // no `relations:` or `lifecycle:` in between to stop at.
        if trimmed.starts_with("- name: ekr.")
            || matches!(
                trimmed,
                "relations:" | "lifecycle:" | "invariants:" | "views:" | "naming:" | "entities:"
            )
        {
            if inside {
                break;
            }
            continue;
        }
        if trimmed == "fields:" {
            inside = true;
            continue;
        }
        if inside {
            if let Some(field) = trimmed.strip_prefix("- name: ") {
                fields.push(field.to_owned());
            }
        }
    }
    assert!(!fields.is_empty(), "the field scan is broken: {name}");
    fields
}

/// Every `.rs` file of this crate's `src/`, as one string.
fn crate_source() -> String {
    let directory = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| std::fs::read_to_string(&path).expect("a source file"))
        .collect()
}

/// Every enumeration the domain declares, and the Rust type that carries it.
///
/// Transcribed here rather than derived, for the reason `crates/ekr/tests/story_contract.rs`
/// transcribes the story's dependency tables: a case whose expectation is read from the thing it
/// is checking asserts nothing. Adding a variant to `graph.yaml` without adding it here, or here
/// without the crate, turns this red.
const ENUMERATIONS: [(&str, &str); 6] = [
    ("ekr.graph.Space", "Space"),
    ("ekr.graph.ValidationState", "ValidationState"),
    ("ekr.graph.SubjectKind", "Subject"),
    ("ekr.graph.PredicateKind", "Predicate"),
    ("ekr.graph.ObjectKind", "Object"),
    ("ekr.graph.EvidenceKind", "EvidenceKind"),
];

#[test]
fn every_enumeration_the_domain_declares_is_carried_variant_for_variant() {
    let source = crate_source();
    for (declared, rust_type) in ENUMERATIONS {
        for variant in variants_of(declared) {
            assert!(
                source.contains(&variant),
                "{declared} declares the variant {variant}, and {rust_type} does not name it"
            );
        }
    }

    // `ObservationKind` is the one whose variants the crate spells twice — once as the flat kind
    // the domain declares and once as the content form design § 15 gives — so it is checked apart
    // from the list above rather than by a substring that would match either.
    for variant in variants_of("ekr.graph.ObservationKind") {
        assert!(
            source.contains(&format!("{variant},")) || source.contains(&format!("{variant} {{")),
            "ekr.graph.ObservationKind declares {variant} and ObservationKind does not"
        );
    }
}

/// The document's own sentence about validation-state payloads, and the crate's quote of it.
///
/// The sentence is read out of `graph.yaml` **at run time** and the crate is required to contain
/// it. The first version of this case asserted a string literal transcribed from the document;
/// the document was then corrected — by this unit's own finding — and the literal went stale in
/// the same commit that made it wrong. A quote checked against a copy of itself is not a quote.
///
/// What it holds: `graph.yaml`'s comment above `ekr.graph.ValidationState` says one state's
/// payload has a carrier, `ekr.graph.Assertion` carries exactly that one, and
/// `crates/ekr-graph/src/assertion.rs` quotes the sentence as it stands today.
#[test]
fn the_crate_quotes_the_domains_own_sentence_about_validation_state_payloads() {
    let sentence = payload_sentence();
    assert!(
        sentence.contains("payload"),
        "the comment above ekr.graph.ValidationState no longer discusses payloads: {sentence:?}"
    );

    let quoted = normalise(&crate_source());
    assert!(
        quoted.contains(&sentence),
        "crates/ekr-graph/src/assertion.rs must quote graph.yaml's sentence as it stands. The \
         document says:\n  {sentence}\nand the crate does not contain it. Update the quote — or, \
         if the document is wrong again, report it rather than paraphrasing it here."
    );

    // The claim the sentence makes, against the entity it makes it about.
    let fields = fields_of("ekr.graph.Assertion");
    assert!(
        fields.contains(&"superseded_by".to_owned()),
        "the one payload carrier the domain has went missing: {fields:?}"
    );
    for (state, absent) in [
        ("Accepted", "validators"),
        ("Rejected", "issues"),
        ("Disputed", "competing"),
        ("Retracted", "reason"),
    ] {
        assert!(
            !fields.iter().any(|field| field.contains(absent)),
            "ekr.graph.Assertion has grown a carrier for a {state} payload (a field naming \
             {absent:?} in {fields:?}). task:graph-domain-carries-validation-state-payloads \
             carries that gap; closing it means updating the crate, the document's sentence and \
             this case together."
        );
    }
}

/// `graph.yaml`'s comment block above `ekr.graph.ValidationState`, as one normalised sentence.
///
/// The first sentence of it that mentions a payload — the claim the crate has to quote.
fn payload_sentence() -> String {
    let text = domain_text();
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == "- name: ekr.graph.ValidationState")
        .expect("the domain declares ekr.graph.ValidationState");
    let comment: Vec<&str> = lines[..at]
        .iter()
        .rev()
        .map_while(|line| line.trim().strip_prefix("# "))
        .collect();
    assert!(
        !comment.is_empty(),
        "ekr.graph.ValidationState has no comment above it to quote"
    );
    let block: String = comment.into_iter().rev().collect::<Vec<_>>().join(" ");

    let normalised = normalise(&block);
    let sentence = normalised
        .split(". ")
        .find(|sentence| sentence.contains("payload"))
        .expect("the comment says something about payloads");
    sentence.to_owned()
}

/// Text with doc-comment markers stripped and runs of whitespace collapsed, so that a sentence
/// wrapped over three `///` lines compares equal to the same sentence on one `#` line.
fn normalise(text: &str) -> String {
    let words: Vec<&str> = text
        .lines()
        .map(|line| {
            let line = line.trim_start();
            // Doc-comment markers, then the block-quote marker a quoted sentence is wrapped in.
            let line = line
                .strip_prefix("///")
                .or_else(|| line.strip_prefix("//!"))
                .unwrap_or(line)
                .trim_start();
            line.strip_prefix('>').unwrap_or(line)
        })
        .flat_map(str::split_whitespace)
        .collect();
    words.join(" ")
}

/// Every `.rs` file of this crate's `src/`, as `(file name, text)`.
fn crate_modules() -> Vec<(String, String)> {
    let directory = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
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
    assert!(found.len() >= 9, "the module scan is broken: {found:?}");
    found
}

/// The source text belonging to one Rust type: its `struct`/`enum` declaration and every `impl`
/// block on it, and nothing else.
///
/// This is the tie the field guard did not have. Its rule was `pub {field}:` anywhere in the ten
/// modules concatenated, so `GraphRoot.parent` and `Root.parent` answered for a hypothetical
/// `ekr.graph.Node.parent` — and the eight declarations share around thirty field names, so the
/// collision is the ordinary case rather than the exotic one. `KnowledgeState` on `GraphRoot` is
/// P3 and `graph.yaml:121` already says it is coming.
///
/// Braces are matched rather than searched for, because a nested type in a field position (
/// `properties: BTreeMap<PropertyId, Value>`) and a nested block in a method body both put a `}`
/// before the one that closes the item.
fn type_region(type_name: &str) -> String {
    let mut region = String::new();

    for (_, text) in crate_modules() {
        let mut at = 0;
        for line in text.lines() {
            if opens_item(line, type_name) {
                let open = at + line.find('{').expect("an item head opens a block");
                region.push_str(&block_at(&text, open));
            }
            at += line.len() + 1;
        }
    }

    assert!(
        !region.is_empty(),
        "no declaration or impl block found for `{type_name}`; PROJECTIONS names a type this \
         crate does not have"
    );
    region
}

/// Whether `line` opens the declaration of `type_name` or an inherent `impl` block on it.
///
/// A match on the head rather than on three literal strings, because
/// `architecture-decision-record:0005-float-is-not-canonical`, as amended, made `Node`, `Edge`,
/// `Object` and `Assertion` generic over the value they carry: their heads read
/// `pub struct Node<V = CanonicalValue> {` and `impl<V> Node<V> {`, and the literal form matched
/// neither — the scanner found no region at all for `Node` and said so, which is why this arrived
/// as a red case rather than as a guard quietly covering nothing.
///
/// A *trait* impl is deliberately not an item head: `impl<V: Canonical> Canonical for Node<V> {`
/// names `Canonical`, not `Node`. That is the behaviour before this change as well — the region is
/// the fields and inherent methods the domain projection is checked against.
///
/// **Stated bounds**, neither reachable in this crate today and both written down rather than
/// worked around:
///
/// * the head has to be on one line, which `cargo fmt` makes true for every item here;
/// * an item head carrying a `where` clause is missed, because what follows the name is then
///   `where …` rather than `{`. The miss is quiet — `type_region` asserts only that the region it
///   built is non-empty, so a *second* item of a type that already has one would vanish from the
///   region without a word.
fn opens_item(line: &str, type_name: &str) -> bool {
    let line = line.trim();
    let rest = if let Some(rest) = line.strip_prefix("pub struct ") {
        rest
    } else if let Some(rest) = line.strip_prefix("pub enum ") {
        rest
    } else if let Some(rest) = line.strip_prefix("impl") {
        after_generics(rest).trim_start()
    } else {
        return false;
    };

    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name != type_name {
        return false;
    }
    after_generics(&rest[name.len()..])
        .trim_start()
        .starts_with('{')
}

/// The text after a balanced `<…>` at the start of `text`, or `text` unchanged when it has none.
fn after_generics(text: &str) -> &str {
    if !text.starts_with('<') {
        return text;
    }
    let mut depth = 0usize;
    for (at, character) in text.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return &text[at + character.len_utf8()..];
                }
            }
            _ => {}
        }
    }
    text
}

/// The text from the `{` at `open` through its matching `}`.
fn block_at(text: &str, open: usize) -> String {
    let bytes = text.as_bytes();
    assert_eq!(bytes[open], b'{', "block_at was not handed a brace");
    let mut depth = 0usize;
    for (offset, byte) in bytes[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text[open..=open + offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("an item's braces do not balance");
}

/// Each declaration of the domain, and the Rust types that together project it.
///
/// A declaration with an empty list is one this crate does not project at all, and the third
/// column says why — checked to be non-trivial below, for the reason `FUSIONS` entries are.
///
/// More than one type per declaration where the projection is spread: `ekr.graph.Assertion`'s
/// `recorded_from` lives on `TransactionTime`, which is the point of that type existing, and
/// `ekr.graph.Evidence`'s `kind` and `locator` are methods on `EvidenceSource`.
const PROJECTIONS: [(&str, &[&str], &str); 9] = [
    (
        "ekr.graph.TypedValue",
        &[],
        "the ess/1 flattening of ekr.ontology.Value — a kind plus the value's canonical text — \
         which exists because ess types are not recursive. The crate holds a recursive value \
         instead, CanonicalValue, so there is no Rust type to bind to this flattening",
    ),
    ("ekr.graph.GraphRoot", &["GraphRoot"], ""),
    ("ekr.graph.Node", &["Node"], ""),
    ("ekr.graph.Edge", &["Edge"], ""),
    (
        "ekr.graph.Assertion",
        &["Assertion", "TemporalRange", "TransactionTime"],
        "",
    ),
    ("ekr.graph.Support", &["Support"], ""),
    (
        "ekr.graph.Evidence",
        &["Evidence", "EvidenceSource", "Confidence"],
        "",
    ),
    (
        "ekr.graph.Observation",
        &["Observation", "ObservationContent"],
        "",
    ),
    (
        "ekr.graph.Assertions",
        &[],
        "a view, not an entity: a read projection over ekr.graph.Assertion that the store \
         materialises. P1 has no store and design § 47's QueryScope is P4, so nothing here \
         projects it and its four fields are ekr.graph.Assertion's own",
    ),
];

/// Fields the domain declares that this crate deliberately does not carry under that name, each
/// with the reason.
///
/// An exception with no reason beside it is not an exception, it is the hole again. Every entry is
/// checked against the document below, so an exception for a field `graph.yaml` no longer declares
/// turns this red rather than sitting here covering nothing.
const FUSIONS: [(&str, &str, &str); 11] = [
    (
        "ekr.graph.TypedValue",
        "canonical",
        "the crate holds the recursive value itself — CanonicalValue, the kinds canonical state \
         admits; TypedValue is the ess/1 flattening of it, needed because ess types are not \
         recursive, and nothing in the crate needs the flattened text",
    ),
    (
        "ekr.graph.Assertion",
        "subject_kind",
        "fused with `subject` into the `Subject` enum: a kind that can disagree with the id beside \
         it is a state design § 13 does not have",
    ),
    (
        "ekr.graph.Assertion",
        "predicate_kind",
        "fused with `predicate` into the `Predicate` enum, for the same reason",
    ),
    (
        "ekr.graph.Assertion",
        "object_kind",
        "fused with `object_value` and `object_ref` into the `Object` enum",
    ),
    (
        "ekr.graph.Assertion",
        "object_value",
        "the `Object::Value` arm; the domain splits it because ess/1 cannot express a sum type \
         with per-variant payloads",
    ),
    (
        "ekr.graph.Assertion",
        "object_ref",
        "the `Object::Node` and `Object::Type` arms",
    ),
    (
        "ekr.graph.Assertion",
        "superseded_by",
        "the payload of `ValidationState::Superseded`, where design § 17 puts it; carrying it \
         beside the state would let the two disagree",
    ),
    (
        "ekr.graph.Assertion",
        "valid_from",
        "the `from` bound of `valid_time: TemporalRange`",
    ),
    (
        "ekr.graph.Assertion",
        "valid_to",
        "the `to` bound of `valid_time: TemporalRange`",
    ),
    (
        "ekr.graph.Evidence",
        "confidence_bp",
        "`confidence: Confidence`, whose `basis_points()` is this value and whose constructor \
         refuses the range the domain states as an invariant",
    ),
    (
        "ekr.graph.Evidence",
        "observation_id",
        "`EvidenceSource::observation()`, which is `Some` only for the one source kind that has \
         one — the domain carries it as an optional field because ess relations are total",
    ),
];

/// Every declaration of the domain has a field or a method for every field it declares, **on the
/// Rust type that projects it**, or an entry in [`FUSIONS`] saying why not.
///
/// "Carried" means a field of that name **or** a method of that name: `ekr.graph.Observation`
/// declares `kind` and `content_hash` beside its content, and the crate holds one
/// `ObservationContent` that both are a function of — a field that can disagree with the content
/// next to it is a field that eventually does. A reader of the domain still gets
/// `observation.kind()` and `observation.content_hash()`, which is what the projection has to
/// guarantee.
///
/// # The bound this case actually has
///
/// The first version of it searched the whole crate's `src/` concatenated, so it answered
/// "carried" whenever *some* type happened to have a field of that name. Adversary pass 2 measured
/// it: a new `ekr.graph.Node.knowledge_state` was caught and a new `ekr.graph.Node.parent` was
/// not, because `GraphRoot.parent` and `Root.parent` exist — and the doc claimed a new field
/// "cannot be omitted". Every field is now looked for inside [`type_region`] of the types
/// [`PROJECTIONS`] binds that declaration to, which is the same slicing [`fields_of`] does on the
/// document side.
///
/// Three lists have to stay honest for that to hold, and each is checked against the document:
/// every declaration appears in `PROJECTIONS`, every `PROJECTIONS` type exists in the crate, and
/// every `FUSIONS` entry names a field the document still declares.
#[test]
fn every_declaration_of_the_domain_is_carried_field_for_field() {
    let declarations = declarations_with_fields();
    let mut checked = 0usize;
    let mut bound = 0usize;

    for declaration in &declarations {
        let (_, types, reason) = PROJECTIONS
            .iter()
            .find(|(name, _, _)| name == declaration)
            .unwrap_or_else(|| {
                panic!(
                    "{declaration} is declared in graph.yaml and PROJECTIONS does not say which \
                     Rust type projects it. A new entity cannot arrive unnoticed: name it, or \
                     name it with an empty type list and the reason it is not projected."
                )
            });

        if types.is_empty() {
            assert!(
                reason.len() > 40,
                "{declaration} is unprojected without a reason worth reading"
            );
            continue;
        }
        assert!(
            reason.is_empty(),
            "{declaration} has both a projecting type and a not-projected reason"
        );

        let region: String = types.iter().map(|name| type_region(name)).collect();
        for field in fields_of(declaration) {
            checked += 1;
            if FUSIONS
                .iter()
                .any(|(owner, name, _)| owner == declaration && *name == field)
            {
                continue;
            }
            bound += 1;
            assert!(
                region.contains(&format!("pub {field}:"))
                    || region.contains(&format!("fn {field}(")),
                "{declaration} declares {field}, and none of {types:?} has a field or a method of \
                 that name. FUSIONS does not say why either. Add the field, or add an exception \
                 with the reason — an unexplained omission is the hole this case exists to close. \
                 A match anywhere else in the crate does not count: that was the defect."
            );
        }
    }

    assert!(checked > 30, "the field scan is broken: {checked} fields");
    assert!(
        bound > 20,
        "only {bound} fields are actually bound to a type; the rest are exceptions, which would \
         make this case a list of excuses rather than a check"
    );

    // A stale entry in either table covers nothing and hides the next one.
    for (declaration, types, _) in PROJECTIONS {
        assert!(
            declarations.contains(&declaration.to_owned()),
            "PROJECTIONS names {declaration}, which the domain no longer declares"
        );
        for name in types {
            // Panics if the type is gone, which is the assertion.
            let _ = type_region(name);
        }
    }
    for (owner, field, reason) in FUSIONS {
        assert!(
            declarations.contains(&owner.to_owned()),
            "FUSIONS names {owner}, which the domain no longer declares"
        );
        assert!(
            fields_of(owner).contains(&field.to_owned()),
            "FUSIONS excepts {owner}.{field}, which the domain no longer declares"
        );
        assert!(
            reason.len() > 40,
            "{owner}.{field} is excepted without a reason worth reading"
        );
    }
}
