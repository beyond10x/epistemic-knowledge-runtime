//! Build the repository's bounded, script-free public project documentation.

use std::{collections::BTreeSet, fs, path::Path, path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

const HTML: &str = include_str!("../../../website/index.html");
const NOT_FOUND: &str = include_str!("../../../website/404.html");
const CSS: &str = include_str!("../../../website/styles.css");
const ICON: &str = include_str!("../../../website/favicon.svg");
const BASE: &str = "/epistemic-knowledge-runtime/";

#[derive(Parser)]
#[command(about = "Validate and build EKR's public documentation")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Validate the authored document without writing files.
    Check,
    /// Emit the static site and provenance for an exact Git revision.
    Build {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        commit: String,
    },
}

fn attributes<'a>(html: &'a str, name: &str) -> Vec<&'a str> {
    let prefix = format!(" {name}=\"");
    html.split(&prefix)
        .skip(1)
        .filter_map(|tail| tail.split('"').next())
        .collect()
}

// The authored source uses double-quoted attributes and no executable content.
// This deliberately validates that small format rather than accepting arbitrary HTML.
// The index names its canonical URL; the not-found page is served at any path and names none.
fn check(html: &str, canonical: bool) -> Result<(), String> {
    if !html.starts_with("<!doctype html>")
        || !html.contains("lang=\"en\"")
        || !html.contains("name=\"viewport\"")
    {
        return Err("missing document declaration, language or mobile viewport".into());
    }
    let ids = attributes(html, "id");
    let unique: BTreeSet<_> = ids.iter().copied().collect();
    if ids.len() != unique.len() {
        return Err("duplicate document anchor".into());
    }
    for href in attributes(html, "href") {
        if let Some(anchor) = href.strip_prefix('#') {
            if !unique.contains(anchor) {
                return Err(format!("missing anchor: {anchor}"));
            }
        } else if !href.starts_with("https://")
            && ![
                BASE.to_owned(),
                format!("{BASE}styles.css"),
                format!("{BASE}favicon.svg"),
            ]
            .contains(&href.to_owned())
        {
            return Err(format!("unexpected route: {href}"));
        }
    }
    if html.to_ascii_lowercase().contains("<script") {
        return Err("the public documentation must remain script-free".into());
    }
    let declares = html.contains(&format!(
        "rel=\"canonical\" href=\"https://beyond10x.github.io{BASE}\""
    ));
    if canonical != declares || (!canonical && html.contains("rel=\"canonical\"")) {
        return Err("wrong canonical URL".into());
    }
    Ok(())
}

fn check_site() -> Result<(), String> {
    check(HTML, true)?;
    check(NOT_FOUND, false).map_err(|error| format!("404.html: {error}"))?;
    if !ICON.starts_with("<svg ") || ICON.to_ascii_lowercase().contains("<script") {
        return Err("favicon.svg must be a script-free SVG document".into());
    }
    Ok(())
}

fn build(out: &Path, commit: &str) -> Result<(), String> {
    if commit.len() != 40
        || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        || commit.bytes().all(|byte| byte == b'0')
    {
        return Err("commit must be a nonzero full Git revision".into());
    }
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(out.join(".well-known"))?;
        fs::write(out.join("index.html"), HTML)?;
        fs::write(out.join("404.html"), NOT_FOUND)?;
        fs::write(out.join("styles.css"), CSS)?;
        fs::write(out.join("favicon.svg"), ICON)?;
        fs::write(out.join(".nojekyll"), "")?;
        // All fields are constants except the strictly hexadecimal commit above.
        fs::write(
            out.join(".well-known/b10x-site.json"),
            format!(
                "{{\n  \"schema\": \"b10x-project-site/v1\",\n  \"repository\": \"epistemic-knowledge-runtime\",\n  \"commit\": \"{commit}\",\n  \"baseUrl\": \"{BASE}\"\n}}\n"
            ),
        )
    };
    write().map_err(|error| format!("cannot write site: {error}"))
}

fn run(cli: Cli) -> Result<(), String> {
    check_site()?;
    match cli.command {
        Action::Check => println!("EKR documentation: anchors, routes and document valid"),
        Action::Build { out, commit } => {
            build(&out, &commit)?;
            println!("EKR documentation built at {}", out.display());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ekr-docs: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_document_is_valid() {
        check_site().expect("valid public documentation");
    }

    #[test]
    fn broken_links_duplicate_anchors_and_scripts_are_refused() {
        assert!(check(&HTML.replace("id=\"quickstart\"", "id=\"missing\""), true).is_err());
        assert!(check(&HTML.replace("id=\"quickstart\"", "id=\"main\""), true).is_err());
        assert!(check(
            &HTML.replace(&format!("{BASE}styles.css"), "/styles.css"),
            true
        )
        .is_err());
        assert!(check(&format!("{HTML}<script></script>"), true).is_err());
        assert!(check(
            &NOT_FOUND.replace(&format!("{BASE}favicon.svg"), "favicon.svg"),
            false
        )
        .is_err());
        assert!(check(&format!("{NOT_FOUND}<SCRIPT></SCRIPT>"), false).is_err());
    }

    #[test]
    fn only_the_index_names_the_canonical_url() {
        assert!(check(HTML, false).is_err());
        assert!(check(NOT_FOUND, true).is_err());
    }

    #[test]
    fn build_writes_the_not_found_page_and_icon() {
        let out = std::env::temp_dir().join(format!("ekr-docs-build-{}", std::process::id()));
        build(&out, &"a".repeat(40)).expect("build");
        for (name, bytes) in [
            ("index.html", HTML),
            ("404.html", NOT_FOUND),
            ("favicon.svg", ICON),
            ("styles.css", CSS),
        ] {
            assert_eq!(fs::read_to_string(out.join(name)).expect(name), bytes);
        }
        fs::remove_dir_all(&out).expect("remove build output");
    }

    #[test]
    fn invalid_commit_is_refused_before_writing() {
        for commit in ["main", "", &"0".repeat(40), &"z".repeat(40)] {
            assert_eq!(
                build(Path::new("unused-output"), commit).unwrap_err(),
                "commit must be a nonzero full Git revision"
            );
        }
    }
}
