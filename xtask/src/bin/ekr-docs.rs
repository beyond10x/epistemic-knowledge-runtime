//! Build the repository's bounded, script-free public project documentation.

use std::{collections::BTreeSet, fs, path::Path, path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

const HTML: &str = include_str!("../../../website/index.html");
const CSS: &str = include_str!("../../../website/styles.css");
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
fn check(html: &str) -> Result<(), String> {
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
        } else if !href.starts_with("https://") && href != format!("{BASE}styles.css") {
            return Err(format!("unexpected route: {href}"));
        }
    }
    if html.to_ascii_lowercase().contains("<script") {
        return Err("the public documentation must remain script-free".into());
    }
    if !html.contains(&format!(
        "rel=\"canonical\" href=\"https://beyond10x.github.io{BASE}\""
    )) {
        return Err("wrong canonical URL".into());
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
        fs::write(out.join("styles.css"), CSS)?;
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
    check(HTML)?;
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
        check(HTML).expect("valid public documentation");
    }

    #[test]
    fn broken_links_duplicate_anchors_and_scripts_are_refused() {
        assert!(check(&HTML.replace("id=\"quickstart\"", "id=\"missing\"")).is_err());
        assert!(check(&HTML.replace("id=\"quickstart\"", "id=\"main\"")).is_err());
        assert!(check(&HTML.replace(&format!("{BASE}styles.css"), "/styles.css")).is_err());
        assert!(check(&format!("{HTML}<script></script>")).is_err());
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
