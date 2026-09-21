//! Repository tasks that are not the product.
//!
//! Every verb here is something the gate or a migration needs and no crate of the runtime should
//! carry. The surface is clap derive, as everywhere in the workspace.

use std::path::Path;

use clap::{Parser, Subcommand};

/// Repository tasks that are not the product.
#[derive(Parser)]
#[command(name = "xtask", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report the workspace root and the members the manifest declares.
    Doctor,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => doctor(),
    }
}

fn doctor() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root");
    println!("workspace root: {}", root.display());
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .expect("the workspace manifest is readable");
    for line in manifest.lines() {
        let line = line.trim();
        if let Some(member) = line.strip_prefix('"').and_then(|l| l.strip_suffix("\",")) {
            println!("member: {member}");
        }
    }
}
