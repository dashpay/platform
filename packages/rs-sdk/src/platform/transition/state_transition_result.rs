use std::ops::Deref;

/// Wrapper that bundles a state transition proof result with the transition hash.
///
/// The transition hash (also known as the transaction ID) is computed deterministically
/// from the serialized `StateTransition` before broadcast, so it does not depend on
/// blockchain state and there is no race condition.
///
/// `StateTransitionResult<T>` implements `Deref<Target = T>`, so existing code that
/// only needs the inner result can use it transparently.
#[derive(Debug, Clone)]
pub struct StateTransitionResult<T> {
    inner: T,
    transition_hash: [u8; 32],
}

impl<T> StateTransitionResult<T> {
    /// Creates a new result bundling the proof result with the transition hash.
    pub fn new(inner: T, transition_hash: [u8; 32]) -> Self {
        Self {
            inner,
            transition_hash,
        }
    }

    /// Returns the transition hash (transaction ID) as a 32-byte array.
    pub fn transition_hash(&self) -> [u8; 32] {
        self.transition_hash
    }

    /// Consumes this wrapper, returning the inner result and the transition hash.
    pub fn into_parts(self) -> (T, [u8; 32]) {
        (self.inner, self.transition_hash)
    }

    /// Consumes this wrapper, returning just the inner result.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Maps the inner value, preserving the transition hash.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> StateTransitionResult<U> {
        StateTransitionResult {
            inner: f(self.inner),
            transition_hash: self.transition_hash,
        }
    }
}

impl<T> Deref for StateTransitionResult<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}
