mod v0;

use std::slice::Iter;
use crate::state_transition::batch_transition::batched_transition::{BatchedTransition, BatchedTransitionMutRef, BatchedTransitionRef};
use crate::state_transition::batch_transition::BatchTransition;
pub use v0::*;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransition;

/// Iterator enum for `BatchTransition` that can handle both V0 and V1.
pub enum DocumentBatchIterator<'a> {
    V0(Iter<'a, DocumentTransition>),
    V1(DocumentBatchV1Iterator<'a>),
}

/// Iterator for version 1, yielding `BatchedTransitionRef<'a>` items.
pub struct DocumentBatchV1Iterator<'a> {
    pub(crate) inner: Iter<'a, BatchedTransition>,
}

impl<'a> Iterator for DocumentBatchV1Iterator<'a> {
    type Item = BatchedTransitionRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|batched_transition| match batched_transition {
                BatchedTransition::Document(doc) => BatchedTransitionRef::Document(doc),
                BatchedTransition::Token(tok) => BatchedTransitionRef::Token(tok),
            })
    }
}

impl<'a> Iterator for DocumentBatchIterator<'a> {
    type Item = BatchedTransitionRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            DocumentBatchIterator::V0(iter) => iter.next().map(BatchedTransitionRef::Document),
            DocumentBatchIterator::V1(iter) => iter.next(),
        }
    }
}

impl DocumentsBatchTransitionAccessorsV0 for BatchTransition {
    type IterType<'a>
        = DocumentBatchIterator<'a>
    where
        Self: 'a;

    /// Iterator for `BatchedTransitionRef` items.
    fn transitions_iter(&self) -> Self::IterType<'_> {
        match self {
            BatchTransition::V0(v0) => DocumentBatchIterator::V0(v0.transitions.iter()),
            BatchTransition::V1(v1) => DocumentBatchIterator::V1(DocumentBatchV1Iterator {
                inner: v1.transitions.iter(),
            }),
        }
    }

    fn transitions_len(&self) -> usize {
        match self {
            BatchTransition::V0(v0) => v0.transitions.len(),
            BatchTransition::V1(v1) => v1.transitions.len(),
        }
    }

    fn transitions_are_empty(&self) -> bool {
        match self {
            BatchTransition::V0(v0) => v0.transitions.is_empty(),
            BatchTransition::V1(v1) => v1.transitions.is_empty(),
        }
    }

    fn first_transition(&self) -> Option<BatchedTransitionRef> {
        match self {
            BatchTransition::V0(v0) => v0.transitions.first().map(BatchedTransitionRef::Document),
            BatchTransition::V1(v1) => v1
                .transitions
                .first()
                .map(|batch_transition| batch_transition.borrow_as_ref()),
        }
    }

    fn first_transition_mut(&mut self) -> Option<BatchedTransitionMutRef> {
        match self {
            BatchTransition::V0(v0) => v0
                .transitions
                .first_mut()
                .map(BatchedTransitionMutRef::Document),
            BatchTransition::V1(v1) => v1
                .transitions
                .first_mut()
                .map(|batch_transition| batch_transition.borrow_as_mut()),
        }
    }

    fn contains_document_transition(&self) -> bool {
        match self {
            BatchTransition::V0(_) => true,
            BatchTransition::V1(v1) => v1
                .transitions
                .iter()
                .any(|transition| matches!(transition, BatchedTransition::Document(_))),
        }
    }

    fn contains_token_transition(&self) -> bool {
        match self {
            BatchTransition::V0(_) => false,
            BatchTransition::V1(v1) => v1
                .transitions
                .iter()
                .any(|transition| matches!(transition, BatchedTransition::Token(_))),
        }
    }
}
