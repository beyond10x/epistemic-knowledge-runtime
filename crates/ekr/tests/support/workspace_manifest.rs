//! Bounded workspace member reader for source guards, not a general TOML parser.
//!
//! Basic/literal relative member paths, ordinary whitespace and comments are supported. Escapes,
//! multiline strings, globs and nonliteral member grammar refuse rather than omit source files.

pub(crate) fn members(manifest: &str) -> Result<Vec<String>, String> {
    let clean = without_comments(manifest)?;
    let mut workspace = Vec::new();
    let mut inside = false;
    let mut seen = false;
    for line in clean.lines() {
        let trimmed = line.trim();
        if let Some(table) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside = table.trim() == "workspace";
            if inside && std::mem::replace(&mut seen, true) {
                return Err("duplicate workspace table".into());
            }
        } else if inside {
            workspace.push(line);
        }
    }
    if !seen {
        return Err("expected an explicit [workspace] table".into());
    }
    let mut assignments = workspace.iter().enumerate().filter_map(|(index, line)| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "members").then_some((index, value))
    });
    let (index, value) = assignments
        .next()
        .ok_or("expected workspace members array")?;
    if assignments.next().is_some() {
        return Err("duplicate workspace members assignment".into());
    }
    let array = std::iter::once(value)
        .chain(workspace[index + 1..].iter().copied())
        .collect::<Vec<_>>()
        .join("\n");
    string_array(&array)
}

fn without_comments(input: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;
    while let Some(ch) = chars.next() {
        if let Some(delimiter) = quote {
            if ch == '\n' || ch == '\r' {
                return Err("multiline TOML strings are unsupported by the source guard".into());
            }
            output.push(ch);
            if ch == delimiter {
                quote = None;
            } else if ch == '\\' && delimiter == '"' {
                output.push(chars.next().ok_or("unterminated TOML string escape")?);
            }
        } else if ch == '#' {
            while chars.peek().is_some_and(|next| *next != '\n') {
                chars.next();
            }
        } else {
            if matches!(ch, '"' | '\'') {
                let mut lookahead = chars.clone();
                if lookahead.next() == Some(ch) && lookahead.next() == Some(ch) {
                    return Err("multiline TOML strings are unsupported by the source guard".into());
                }
                quote = Some(ch);
            }
            output.push(ch);
        }
    }
    if quote.is_some() {
        return Err("unterminated TOML string".into());
    }
    Ok(output)
}

fn string_array(input: &str) -> Result<Vec<String>, String> {
    let mut chars = input.trim_start().chars().peekable();
    if chars.next() != Some('[') {
        return Err("workspace members must be an explicit string array".into());
    }
    let mut result = Vec::new();
    loop {
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            chars.next();
        }
        if chars.peek() == Some(&']') {
            chars.next();
            break;
        }
        let quote = chars.next().ok_or("unclosed workspace members array")?;
        if !matches!(quote, '"' | '\'') {
            return Err("workspace member must be a basic or literal string".into());
        }
        let mut member = String::new();
        loop {
            let ch = chars.next().ok_or("unclosed workspace member string")?;
            if ch == quote {
                break;
            }
            if matches!(ch, '\\' | '\n' | '\r') {
                return Err("escaped or multiline workspace member paths are unsupported".into());
            }
            member.push(ch);
        }
        if member.is_empty()
            || member.contains(['*', '?', '[', ']'])
            || member
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(format!("unsupported workspace member path: {member:?}"));
        }
        if result.contains(&member) {
            return Err(format!("duplicate workspace member: {member}"));
        }
        result.push(member);
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            chars.next();
        }
        match chars.peek() {
            Some(',') => {
                chars.next();
            }
            Some(']') => {}
            _ => return Err("workspace members require commas and a closing bracket".into()),
        }
    }
    if chars
        .take_while(|ch| *ch != '\n')
        .any(|ch| !ch.is_whitespace())
    {
        return Err("unexpected content after workspace members array".into());
    }
    if result.is_empty() {
        return Err("workspace members array must not be empty".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::members;

    #[test]
    fn comments_and_hashes_inside_basic_and_literal_paths_are_distinct() {
        let manifest = "# unrelated members = [\"wrong\"]\n[workspace] # table\n\
            resolver = \"2\"\nmembers= [ # opening\n\
            \"crates/graph#one\", # trailing member comment\n\
            'crates/kernel#two',\n\"xtask\" # no trailing comma\n] # closing\n\
            [workspace.package]\nname = \"fixture\"\n";
        assert_eq!(
            members(manifest).unwrap(),
            ["crates/graph#one", "crates/kernel#two", "xtask"]
        );
    }

    #[test]
    fn unsupported_or_ambiguous_member_grammar_refuses_instead_of_losing_members() {
        for array in [
            r#"["crates/*"]"#,
            r#"["crates/\u006b"]"#,
            "['crates/../outside']",
            "[crate]",
            "['crates/a' 'crates/b']",
            "['crates/a',",
            "[]",
            "['crates/a', 'crates/a']",
            "['crates/a'] trailing",
        ] {
            assert!(
                members(&format!("[workspace]\nmembers = {array}\n")).is_err(),
                "{array}"
            );
        }
        assert!(members("[workspace]\nmembers=['crates/a']\nmembers=['crates/b']\n").is_err());
        assert!(members("[workspace]\nmembers=['crates/a']\n[workspace]\n").is_err());
        assert!(members("[workspace]\nmembers=[\"\"\"crates/a\"\"\"]\n").is_err());
    }
}
