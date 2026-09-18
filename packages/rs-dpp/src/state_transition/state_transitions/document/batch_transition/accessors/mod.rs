mod v0;
mod v1;

use std::iter::Empty;
use std::slice::Iter;
use crate::state_transition::batch_transition::batched_transition::{BatchedTransition, BatchedTransitionMutRef, BatchedTransitionRef};
use crate::state_transition::batch_transition::BatchTransition;
pub use v0::*;
pub use v1::*;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::DocumentTransition;

/// Iterator enum for `BatchTransition` that can handle both V0 and V1.
pub enum DocumentBatchIterator<'a> {
    V0(Iter<'a, DocumentTransition>),
    V1(DocumentBatchV1Iterator<'a>),
    /// Batch format 2 has no view through the format 0 and 1 accessors; see
    /// the `DocumentsBatchTransitionAccessorsV0` implementation below.
    V2(Empty<BatchedTransitionRef<'a>>),
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
            DocumentBatchIterator::V2(iter) => iter.next(),
        }
    }
}

/// The view of a batch through the transition shells of batch formats 0 and 1.
///
/// Batch format 2 carries a document transition shell with an erase kind that
/// [`BatchedTransitionRef`] cannot represent, so a format 2 batch has no view
/// here: every accessor reports an empty batch for it. Code that must see every
/// format uses [`DocumentsBatchTransitionAccessorsV1`]. The generations that
/// still read this view are the ones selected for protocol versions that reject
/// a format 2 batch by its wire version before any of them runs, and the first
/// of them rejects an empty batch, so the empty view fails closed rather than
/// silently dropping transitions.
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
            BatchTransition::V2(_) => DocumentBatchIterator::V2(std::iter::empty()),
        }
    }

    fn transitions_len(&self) -> usize {
        match self {
            BatchTransition::V0(v0) => v0.transitions.len(),
            BatchTransition::V1(v1) => v1.transitions.len(),
            BatchTransition::V2(_) => 0,
        }
    }

    fn transitions_are_empty(&self) -> bool {
        match self {
            BatchTransition::V0(v0) => v0.transitions.is_empty(),
            BatchTransition::V1(v1) => v1.transitions.is_empty(),
            BatchTransition::V2(_) => true,
        }
    }

    fn first_transition(&self) -> Option<BatchedTransitionRef<'_>> {
        match self {
            BatchTransition::V0(v0) => v0.transitions.first().map(BatchedTransitionRef::Document),
            BatchTransition::V1(v1) => v1
                .transitions
                .first()
                .map(|batch_transition| batch_transition.borrow_as_ref()),
            BatchTransition::V2(_) => None,
        }
    }

    fn first_transition_mut(&mut self) -> Option<BatchedTransitionMutRef<'_>> {
        match self {
            BatchTransition::V0(v0) => v0
                .transitions
                .first_mut()
                .map(BatchedTransitionMutRef::Document),
            BatchTransition::V1(v1) => v1
                .transitions
                .first_mut()
                .map(|batch_transition| batch_transition.borrow_as_mut()),
            BatchTransition::V2(_) => None,
        }
    }

    fn contains_document_transition(&self) -> bool {
        match self {
            BatchTransition::V0(_) => true,
            BatchTransition::V1(v1) => v1
                .transitions
                .iter()
                .any(|transition| matches!(transition, BatchedTransition::Document(_))),
            BatchTransition::V2(_) => false,
        }
    }

    fn contains_token_transition(&self) -> bool {
        match self {
            BatchTransition::V0(_) => false,
            BatchTransition::V1(v1) => v1
                .transitions
                .iter()
                .any(|transition| matches!(transition, BatchedTransition::Token(_))),
            BatchTransition::V2(_) => false,
        }
    }
}
