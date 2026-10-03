// generated from ekr v1
// model digest 76cac94520a78179089e10b0871c7979eacd35197c8e7b584616fac60d49e705
// contract digest c0793b41517a2b54756a5ae9c6d0c85aa53885defe0f88328a27a2c46c880d14
// do not edit: regenerate with `ess synthesize`

//! The typed refusal of an unmet obligation, and the conversion seams owed between contexts.
//!
//! An obligation is a capability the synthesis plan owes the implementor — the contract is declared,
//! the behaviour is not. Until an implementation satisfies one, its stub returns [`UnmetObligation`]:
//! a value naming the plan entry, never a panic and never a guess, so a workspace built on stubs
//! compiles and reports its own gaps.

/// A capability the synthesis plan owes and nothing has satisfied yet.
///
/// The two fields spell the plan entry: look the pair up in `PLAN.md` for the contract being
/// refused. A satisfying implementation never constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmetObligation {
    /// The capability kind, as the plan spells it.
    pub capability: &'static str,
    /// The construct that requires it, in the specification's own spelling.
    pub source: &'static str,
}

impl core::fmt::Display for UnmetObligation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unmet obligation: {} `{}` — see PLAN.md", self.capability, self.source)
    }
}
