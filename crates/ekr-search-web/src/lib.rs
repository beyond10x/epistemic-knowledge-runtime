//! The progressive search adapter. Search and HTML remain owned by the native viewer.

#[cfg(target_arch = "wasm32")]
mod browser;

/// Completion authority is independent of cancellation: an already-completed fetch can still
/// enqueue a continuation after it has been superseded or after composition begins.
#[derive(Default)]
#[cfg(any(target_arch = "wasm32", test))]
struct Generation(u64);
#[cfg(any(target_arch = "wasm32", test))]
impl Generation {
    fn next(&mut self) -> u64 {
        self.0 = self.0.checked_add(1).expect("search generation exhausted");
        self.0
    }
    fn accepts(&self, generation: u64) -> bool {
        self.0 == generation
    }
}

#[cfg(test)]
mod tests {
    use super::Generation;
    #[test]
    fn a_completed_but_superseded_read_has_no_render_authority() {
        let mut state = Generation::default();
        let first = state.next();
        let current = state.next();
        assert!(!state.accepts(first));
        assert!(state.accepts(current));
        state.next(); // clearing or composition also invalidates outstanding continuations
        assert!(!state.accepts(current));
    }
}
