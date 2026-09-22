//! Small lexical tripwires for repository policy, not a Rust parser or semantic coverage proof.
//!
//! Comments disappear; literals remain single opaque tokens. Consequently neither a string's
//! contents nor two identifier fragments separated by a comment become an identifier use.

/// Code tokens, retaining borrowed source slices and literal boundaries.
pub(crate) fn tokens(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at].is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if bytes[at..].starts_with(b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
            continue;
        }
        if bytes[at..].starts_with(b"/*") {
            let mut depth = 1usize;
            at += 2;
            while at < bytes.len() && depth > 0 {
                if bytes[at..].starts_with(b"/*") {
                    depth += 1;
                    at += 2;
                } else if bytes[at..].starts_with(b"*/") {
                    depth -= 1;
                    at += 2;
                } else {
                    at += 1;
                }
            }
            assert_eq!(depth, 0, "unterminated Rust block comment");
            continue;
        }
        let start = at;
        if let Some(end) = literal_end(source, at) {
            result.push(&source[start..end]);
            at = end;
            continue;
        }
        // Raw identifiers are identifiers; r#"..."# was already consumed as one literal.
        if bytes[at..].starts_with(b"r#")
            && bytes.get(at + 2).is_some_and(|byte| identifier_byte(*byte))
        {
            at += 2;
            let name = at;
            while at < bytes.len() && identifier_byte(bytes[at]) {
                at += 1;
            }
            result.push(&source[name..at]);
            continue;
        }
        if identifier_byte(bytes[at]) {
            at += 1;
            while at < bytes.len() && identifier_byte(bytes[at]) {
                at += 1;
            }
        } else if ["::", "->"]
            .iter()
            .any(|punctuation| source[at..].starts_with(punctuation))
        {
            at += 2;
        } else {
            at += 1;
        }
        result.push(&source[start..at]);
    }
    result
}

fn identifier_byte(byte: u8) -> bool {
    // Treat every non-ASCII codepoint as part of one identifier token. This deliberately avoids
    // interpreting a prefix of a Unicode identifier as a tested ASCII name.
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 128
}

fn literal_end(source: &str, at: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote = at;
    if matches!(bytes[at], b'b' | b'c') {
        quote += 1;
    }
    if bytes.get(quote) == Some(&b'r') {
        let mut opening = quote + 1;
        while bytes.get(opening) == Some(&b'#') {
            opening += 1;
        }
        if bytes.get(opening) == Some(&b'"') {
            let hashes = opening - quote - 1;
            let mut end = opening + 1;
            while end < bytes.len() {
                if bytes[end] == b'"'
                    && bytes
                        .get(end + 1..end + 1 + hashes)
                        .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
                {
                    return Some(end + 1 + hashes);
                }
                end += 1;
            }
            panic!("unterminated Rust raw string");
        }
    }
    if bytes.get(quote) == Some(&b'"') {
        return Some(quoted_end(bytes, quote, b'"'));
    }
    if bytes.get(quote) == Some(&b'\'') {
        let next = quote + 1;
        if bytes.get(next) == Some(&b'\\') {
            return Some(quoted_end(bytes, quote, b'\''));
        }
        // One character followed immediately by a quote is a character, not a lifetime.
        if let Some(character) = source.get(next..).and_then(|text| text.chars().next()) {
            let end = next + character.len_utf8();
            if bytes.get(end) == Some(&b'\'') {
                return Some(end + 1);
            }
        }
    }
    None
}

fn quoted_end(bytes: &[u8], opening: usize, quote: u8) -> usize {
    let mut at = opening + 1;
    while at < bytes.len() {
        if bytes[at] == b'\\' {
            at += 2;
        } else if bytes[at] == quote {
            return at + 1;
        } else {
            at += 1;
        }
    }
    panic!("unterminated Rust quoted literal");
}

#[cfg(test)]
mod tests {
    use super::tokens;

    #[test]
    fn comments_and_all_string_forms_are_not_code_identifiers() {
        let source = r####"
            // Phantom::use_it()
            /* outer /* Phantom */ comment */
            "Phantom::use_it() /* not a comment */"
            b"Phantom" c"Phantom" r"Phantom" r###"Phantom"#"###
            br##"Phantom"## cr#"Phantom"#
            Real::use_it()
        "####;
        let actual = tokens(source);
        assert!(!actual.contains(&"Phantom"));
        assert_eq!(
            &actual[actual.len() - 5..],
            ["Real", "::", "use_it", "(", ")"]
        );
    }

    #[test]
    fn chars_escapes_and_lifetimes_keep_distinct_boundaries() {
        let actual =
            tokens(r"fn borrow<'a, 'long>(value: &'a Real) { 'x'; 'é'; '\n'; '\''; b'x'; }");
        assert!(actual.windows(3).any(|part| part == ["'", "a", ","]));
        assert!(actual.windows(3).any(|part| part == ["'", "long", ">"]));
        assert!(actual.windows(3).any(|part| part == ["'", "a", "Real"]));
        for literal in ["'x'", "'é'", r"'\n'", r"'\''", "b'x'"] {
            assert!(actual.contains(&literal), "{literal}: {actual:?}");
        }
    }

    #[test]
    fn identifiers_do_not_join_across_comments_or_split_at_unicode() {
        assert_eq!(
            tokens("Pub/* comment */lic LongerType Typeé r#type"),
            ["Pub", "lic", "LongerType", "Typeé", "type"]
        );
    }
}
