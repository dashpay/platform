use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

pub trait TokenBaseTransitionAccessors {
    /// Returns a reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base(&self) -> &TokenBaseTransition;

    /// Returns a mut reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base_mut(&mut self) -> &mut TokenBaseTransition;

    /// Sets the value of the `base` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `base` - A value of type `DocumentBaseTransition` to set.
    fn set_base(&mut self, base: TokenBaseTransition);
}
