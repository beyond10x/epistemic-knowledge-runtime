//! A store's refusal reaches the CLI as a fault that says whether a long-running reader opens the
//! store again: `Failure::store` for the kernel's `PersistenceError`, `Failure::unread` for an
//! `ekr.views` read. A diverged history and a replaced database reopen; any other store fault does
//! not, and none of them is a named refusal.

use ekr::exit::Failure;
use ekr_kernel::PersistenceError;
use ekr_views::ProjectError;

/// The flags one fault carries, read through the public predicates.
fn flags(failure: &Failure) -> (bool, bool, bool, u8, Option<&'static str>) {
    (
        Failure::diverged(failure),
        Failure::replaced(failure),
        Failure::reopens(failure),
        failure.code(),
        failure.name(),
    )
}

#[test]
fn a_store_refusal_reopens_only_when_the_store_diverged_or_was_replaced() {
    for (failure, expected, case) in [
        (
            Failure::store(PersistenceError::Diverged("held history".into())),
            (true, false, true, 1, None),
            "store diverged",
        ),
        (
            Failure::store(PersistenceError::Replaced("state.db".into())),
            (false, true, true, 1, None),
            "store replaced",
        ),
        (
            Failure::store(PersistenceError::Backend("disk".into())),
            (false, false, false, 1, None),
            "store backend fault",
        ),
    ] {
        assert_eq!(flags(&failure), expected, "{case}: {failure}");
    }
}

#[test]
fn a_views_read_fault_reopens_only_when_the_store_diverged_or_was_replaced() {
    for (failure, expected, case) in [
        (
            Failure::unread(ProjectError::Diverged("held history".into())),
            (true, false, true, 1, None),
            "views read diverged",
        ),
        (
            Failure::unread(ProjectError::Replaced("state.db".into())),
            (false, true, true, 1, None),
            "views read replaced",
        ),
        (
            Failure::unread(ProjectError::Read("unreadable".into())),
            (false, false, false, 1, None),
            "views read fault",
        ),
    ] {
        assert_eq!(flags(&failure), expected, "{case}: {failure}");
    }
}

#[test]
fn a_plain_fault_and_a_refusal_never_reopen() {
    let fault = Failure::fault("configuration");
    assert_eq!(flags(&fault), (false, false, false, 1, None));
    let refused = Failure::refused("ekr.kernel.InvalidSeed", "seed");
    assert!(!refused.diverged() && !refused.replaced() && !refused.reopens());
}
