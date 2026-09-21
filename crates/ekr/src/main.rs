//! The command-line surface of the Epistemic Knowledge Runtime.
//!
//! This binary implements no single ESS domain. It composes the four the runtime declares in
//! `systems/ekr/components.yaml`, dispatching each verb it grows to the crate that owns the
//! domain the command names. The surface is clap derive, as everywhere in the workspace.

use clap::Parser;

/// The command-line surface of the Epistemic Knowledge Runtime.
#[derive(Parser)]
#[command(name = "ekr", version, about)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
