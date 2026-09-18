use std::slice::Iter;

use crate::state_transition::batch_transition::batched_transition::{
    BatchedTransition, BatchedTransitionMutRefV1, BatchedTransitionRefV1, BatchedTransitionV1,
    DocumentTransition,
};
use crate::state_transition::batch_transition::{
    BatchTransition, BatchTransitionV0, BatchTransitionV1, BatchTransitionV2,
};

/// The view of a batch that sees every batch format.
///
/// Items are borrowed through the transition shells introduced with batch
/// format 2, which know every kind the older formats carry plus the erase
/// kind. Formats 0 and 1 are viewed through a lossless conversion of their own
/// shells.
pub trait DocumentsBatchTransitionAccessorsV1 {
    /// Associated type for the iterator.
    type IterType<'a>: Iterator<Item = BatchedTransitionRefV1<'a>>
    where
        Self: 'a;

    /// Returns an iterator over the `BatchedTransitionRefV1` items.
    fn transitions_iter_v1(&self) -> Self::IterType<'_>;

    fn transitions_len_v1(&self) -> usize;
    fn transitions_are_empty_v1(&self) -> bool;

    fn first_transition_v1(&self) -> Option<BatchedTransitionRefV1<'_>>;

    fn first_transition_mut_v1(&mut self) -> Option<BatchedTransitionMutRefV1<'_>>;
    fn contains_document_transition_v1(&self) -> bool;
    fn contains_token_transition_v1(&self) -> bool;
}

/// Iterator over the items of any batch format, yielding
/// `BatchedTransitionRefV1<'a>` items.
pub enum DocumentBatchIteratorV1<'a> {
    V0(Iter<'a, DocumentTransition>),
    V1(Iter<'a, BatchedTransition>),
    V2(Iter<'a, BatchedTransitionV1>),
}

impl<'a> Iterator for DocumentBatchIteratorV1<'a> {
    type Item = BatchedTransitionRefV1<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::V0(iter) => iter
                .next()
                .map(|transition| BatchedTransitionRefV1::Document(transition.into())),
            Self::V1(iter) => iter.next().map(Into::into),
            Self::V2(iter) => iter.next().map(BatchedTransitionV1::borrow_as_ref),
        }
    }
}

impl DocumentsBatchTransitionAccessorsV1 for BatchTransition {
    type IterType<'a>
        = DocumentBatchIteratorV1<'a>
    where
        Self: 'a;

    fn transitions_iter_v1(&self) -> Self::IterType<'_> {
        match self {
            Self::V0(transition) => DocumentBatchIteratorV1::V0(transition.transitions.iter()),
            Self::V1(transition) => DocumentBatchIteratorV1::V1(transition.transitions.iter()),
            Self::V2(transition) => DocumentBatchIteratorV1::V2(transition.transitions.iter()),
        }
    }

    fn transitions_len_v1(&self) -> usize {
        match self {
            Self::V0(transition) => transition.transitions.len(),
            Self::V1(transition) => transition.transitions.len(),
            Self::V2(transition) => transition.transitions.len(),
        }
    }

    fn transitions_are_empty_v1(&self) -> bool {
        self.transitions_len_v1() == 0
    }

    fn first_transition_v1(&self) -> Option<BatchedTransitionRefV1<'_>> {
        self.transitions_iter_v1().next()
    }

    fn first_transition_mut_v1(&mut self) -> Option<BatchedTransitionMutRefV1<'_>> {
        match self {
            Self::V0(transition) => transition
                .transitions
                .first_mut()
                .map(|transition| BatchedTransitionMutRefV1::Document(transition.into())),
            Self::V1(transition) => transition.transitions.first_mut().map(Into::into),
            Self::V2(transition) => transition
                .transitions
                .first_mut()
                .map(BatchedTransitionV1::borrow_as_mut),
        }
    }

    fn contains_document_transition_v1(&self) -> bool {
        self.transitions_iter_v1()
            .any(|transition| matches!(transition, BatchedTransitionRefV1::Document(_)))
    }

    fn contains_token_transition_v1(&self) -> bool {
        self.transitions_iter_v1()
            .any(|transition| matches!(transition, BatchedTransitionRefV1::Token(_)))
    }
}

impl DocumentsBatchTransitionAccessorsV1 for BatchTransitionV2 {
    type IterType<'a>
        = DocumentBatchIteratorV1<'a>
    where
        Self: 'a;

    fn transitions_iter_v1(&self) -> Self::IterType<'_> {
        DocumentBatchIteratorV1::V2(self.transitions.iter())
    }

    fn transitions_len_v1(&self) -> usize {
        self.transitions.len()
    }

    fn transitions_are_empty_v1(&self) -> bool {
        self.transitions.is_empty()
    }

    fn first_transition_v1(&self) -> Option<BatchedTransitionRefV1<'_>> {
        self.transitions
            .first()
            .map(BatchedTransitionV1::borrow_as_ref)
    }

    fn first_transition_mut_v1(&mut self) -> Option<BatchedTransitionMutRefV1<'_>> {
        self.transitions
            .first_mut()
            .map(BatchedTransitionV1::borrow_as_mut)
    }

    fn contains_document_transition_v1(&self) -> bool {
        self.transitions
            .iter()
            .any(|transition| matches!(transition, BatchedTransitionV1::Document(_)))
    }

    fn contains_token_transition_v1(&self) -> bool {
        self.transitions
            .iter()
            .any(|transition| matches!(transition, BatchedTransitionV1::Token(_)))
    }
}

impl DocumentsBatchTransitionAccessorsV1 for BatchTransitionV0 {
    type IterType<'a>
        = DocumentBatchIteratorV1<'a>
    where
        Self: 'a;

    fn transitions_iter_v1(&self) -> Self::IterType<'_> {
        DocumentBatchIteratorV1::V0(self.transitions.iter())
    }

    fn transitions_len_v1(&self) -> usize {
        self.transitions.len()
    }

    fn transitions_are_empty_v1(&self) -> bool {
        self.transitions.is_empty()
    }

    fn first_transition_v1(&self) -> Option<BatchedTransitionRefV1<'_>> {
        self.transitions
            .first()
            .map(|transition| BatchedTransitionRefV1::Document(transition.into()))
    }

    fn first_transition_mut_v1(&mut self) -> Option<BatchedTransitionMutRefV1<'_>> {
        self.transitions
            .first_mut()
            .map(|transition| BatchedTransitionMutRefV1::Document(transition.into()))
    }

    fn contains_document_transition_v1(&self) -> bool {
        !self.transitions.is_empty()
    }

    fn contains_token_transition_v1(&self) -> bool {
        false
    }
}

impl DocumentsBatchTransitionAccessorsV1 for BatchTransitionV1 {
    type IterType<'a>
        = DocumentBatchIteratorV1<'a>
    where
        Self: 'a;

    fn transitions_iter_v1(&self) -> Self::IterType<'_> {
        DocumentBatchIteratorV1::V1(self.transitions.iter())
    }

    fn transitions_len_v1(&self) -> usize {
        self.transitions.len()
    }

    fn transitions_are_empty_v1(&self) -> bool {
        self.transitions.is_empty()
    }

    fn first_transition_v1(&self) -> Option<BatchedTransitionRefV1<'_>> {
        self.transitions.first().map(Into::into)
    }

    fn first_transition_mut_v1(&mut self) -> Option<BatchedTransitionMutRefV1<'_>> {
        self.transitions.first_mut().map(Into::into)
    }

    fn contains_document_transition_v1(&self) -> bool {
        self.transitions
            .iter()
            .any(|transition| matches!(transition, BatchedTransition::Document(_)))
    }

    fn contains_token_transition_v1(&self) -> bool {
        self.transitions
            .iter()
            .any(|transition| matches!(transition, BatchedTransition::Token(_)))
    }
}
