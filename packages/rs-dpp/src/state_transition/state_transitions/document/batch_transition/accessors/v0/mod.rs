use crate::state_transition::batch_transition::batched_transition::{
    BatchedTransitionMutRef, BatchedTransitionRef,
};
pub trait DocumentsBatchTransitionAccessorsV0 {
    /// Associated type for the iterator.
    type IterType<'a>: Iterator<Item = BatchedTransitionRef<'a>>
    where
        Self: 'a;

    /// Returns an iterator over the `BatchedTransitionRef` items.
    fn transitions_iter(&self) -> Self::IterType<'_>;

    fn transitions_len(&self) -> usize;
    fn transitions_are_empty(&self) -> bool;

    fn first_transition(&self) -> Option<BatchedTransitionRef>;

    fn first_transition_mut(&mut self) -> Option<BatchedTransitionMutRef>;
    fn contains_document_transition(&self) -> bool;
    fn contains_token_transition(&self) -> bool;
}
