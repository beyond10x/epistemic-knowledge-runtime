//! The command-line surface of the Epistemic Knowledge Runtime.
//!
//! This binary implements no single ESS domain. It composes the four the runtime declares in
//! `systems/ekr/components.yaml`, dispatching each verb it grows to the crate that owns the
//! domain the command names. The surface is clap derive, as everywhere in the workspace.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use ekr::cli::{execute, serve, system_time, Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mut stdin = std::io::stdin().lock();
    if matches!(cli.command, Command::Session) {
        // A session answers each request as it is read, so it writes to stdout itself.
        let mut stdout = std::io::stdout().lock();
        return match serve(cli, &system_time, &mut stdin, &mut stdout) {
            Ok(()) => ExitCode::SUCCESS,
            Err(failure) => {
                eprintln!("{failure}");
                ExitCode::from(failure.code())
            }
        };
    }
    match execute(cli, &system_time, &mut stdin) {
        Ok(result) => {
            let mut stdout = std::io::stdout().lock();
            if let Err(error) = stdout
                .write_all(result.as_bytes())
                .and_then(|()| stdout.flush())
            {
                eprintln!("ekr: writing the result: {error}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::from(failure.code())
        }
    }
}
