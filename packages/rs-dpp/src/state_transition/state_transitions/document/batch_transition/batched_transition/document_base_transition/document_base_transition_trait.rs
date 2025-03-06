use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

pub trait DocumentBaseTransitionAccessors {
    /// Returns a reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base(&self) -> &DocumentBaseTransition;

    /// Returns a mut reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;

    /// Sets the value of the `base` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `base` - A value of type `DocumentBaseTransition` to set.
    fn set_base(&mut self, base: DocumentBaseTransition);
}
