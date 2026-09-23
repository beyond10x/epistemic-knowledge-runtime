//! `ekr head`: the verified head revision number and root, through `Runtime::head`.

use ekr_graph::Root;
use ekr_kernel::{CommitError, Runtime};
use serde::Serialize;

use crate::exit::Failure;

/// The head as `ekr head` prints it.
#[derive(Serialize)]
pub(super) struct Head {
    revision: u64,
    root: Root,
}

/// The verified current root. An unseeded store is the kernel's `NotSeeded`, a fault.
pub(super) fn root(runtime: &Runtime) -> Result<Root, Failure> {
    runtime
        .head()
        .map_err(Failure::fault)?
        .ok_or_else(|| Failure::from(CommitError::NotSeeded))
}

pub(super) fn run(runtime: &Runtime) -> Result<Head, Failure> {
    let root = root(runtime)?;
    Ok(Head {
        revision: root.revision.get(),
        root,
    })
}
