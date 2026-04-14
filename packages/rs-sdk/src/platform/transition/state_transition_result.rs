use std::ops::Deref;

/// Wrapper that bundles a state transition proof result with the transition hash.
///
/// The transition hash (also known as the transaction ID) is computed deterministically
/// from the serialized `StateTransition` before broadcast, so it does not depend on
/// blockchain state and there is no race condition.
///
/// `StateTransitionResult<T>` implements `Deref<Target = T>`, so existing code that
/// only needs the inner result can use it transparently.
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::StateTransitionResult;

    fn sample_hash() -> [u8; 32] {
        [7; 32]
    }

    #[test]
    fn new_exposes_transition_hash() {
        let result = StateTransitionResult::new("value", sample_hash());

        assert_eq!(result.transition_hash(), sample_hash());
    }

    #[test]
    fn into_parts_returns_inner_and_hash() {
        let result = StateTransitionResult::new(42_u8, sample_hash());

        assert_eq!(result.into_parts(), (42_u8, sample_hash()));
    }

    #[test]
    fn into_inner_returns_inner_value() {
        let result = StateTransitionResult::new(String::from("value"), sample_hash());

        assert_eq!(result.into_inner(), "value");
    }

    #[test]
    fn map_preserves_transition_hash() {
        let result = StateTransitionResult::new(21_u8, sample_hash());

        let mapped = result.map(|value| value * 2);

        assert_eq!(mapped.into_parts(), (42_u8, sample_hash()));
    }

    #[test]
    fn deref_exposes_inner_value() {
        let result = StateTransitionResult::new(vec![1_u8, 2, 3], sample_hash());

        assert_eq!(result.len(), 3);
        assert_eq!(result[1], 2);
    }
}
