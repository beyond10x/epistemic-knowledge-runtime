//! The command-line surface of the Epistemic Knowledge Runtime.
//!
//! This binary is the component `ekr` of `systems/ekr/components.yaml`. It composes the domains
//! the runtime declares there, dispatching each verb to the crate that owns the domain the command
//! names, and handles the commands of the one domain that component owns, `ekr.cli` — the stage
//! commands and the view of every stage, which `ekr stage` runs through the kernel (design § 107).
//! The surface is clap derive, as everywhere in the workspace.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use ekr::cli::{execute, serve, serve_mcp, system_time, Cli, Command};

/// `ekr view` serves until it is interrupted and `ekr mcp` until its input ends or it is stopped;
/// neither drops its store then. On SIGTERM, SIGINT or SIGHUP this removes the private copy a
/// read-only File store is read through (`ekr_store`'s `read_only.rs`) and exits with 128 plus
/// the signal's number, as a shell reports a process that signal ended.
fn remove_copies_when_terminated() {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGTERM, SIGINT, SIGHUP]) else {
        return;
    };
    std::thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            ekr_kernel::Runtime::remove_read_only_copies();
            std::process::exit(128 + signal);
        }
    });
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if matches!(
        cli.command,
        Command::View { .. } | Command::Mcp | Command::McpHttp { .. }
    ) {
        remove_copies_when_terminated();
    }
    let mut stdin = std::io::stdin().lock();
    if matches!(cli.command, Command::Session { .. } | Command::Mcp) {
        // A session and an MCP server answer each message as it is read, so they write to stdout
        // themselves.
        let mut stdout = std::io::stdout().lock();
        let served = if matches!(cli.command, Command::Mcp) {
            serve_mcp(cli, &mut stdin, &mut stdout)
        } else {
            serve(cli, &system_time, &mut stdin, &mut stdout)
        };
        return match served {
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
