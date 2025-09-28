//! Document subscription management utilities.

/// Document subscription filtering

pub mod document_filter;
/// Token subscription filtering

pub mod token_filter;

/// Contract subscription filtering
pub mod contract_filter;


use contract_filter::DriveContractQueryFilter;

use document_filter::DriveDocumentQueryFilter;

use dpp::document::Document;

use dpp::platform_value::Value;

use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;

use dpp::state_transition::batch_transition::batched_transition::{
    document_transition::DocumentTransition, token_transition::TokenTransition,
    BatchedTransitionRef,
};

use dpp::identifier::Identifier;
use dpp::state_transition::StateTransition;
use dpp::data_contract::DataContract;

use std::collections::BTreeMap;
use token_filter::DriveTokenQueryFilter;

/// Result of evaluating constraints for a transition before potentially fetching the original document.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionCheckResult {
    /// All applicable transition-level checks pass and no original is required.
    Pass,
    /// Some transition-level check fails; do not fetch original.
    Fail,
    /// Transition-level checks pass, original clauses are non-empty and must be evaluated.
    NeedsOriginal,
}

/// Categorised matches for a subscription filter.

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TransitionMatchSet<'a> {
    /// Transitions that satisfy the filter without additional data.
    pub passes: Vec<&'a StateTransition>,
    /// Transitions that need fetching an original state to complete evaluation.
    pub needs_original: Vec<&'a StateTransition>,
}

/// Lookup helper for original state fetched during transition processing.

#[derive(Debug, Default, Clone)]
pub struct SubscriptionOriginalState<'a> {
    /// Documents keyed by their identifiers.
    pub documents: BTreeMap<Identifier, &'a Document>,
}


impl<'a> SubscriptionOriginalState<'a> {
    /// Returns the original document for the provided identifier when available.
    pub fn document(&self, id: &Identifier) -> Option<&'a Document> {
        self.documents.get(id).copied()
    }
}

/// Represents whether a subscription query hit any filters
/// when evaluating against the provided proof data.
///
/// This enum is typically used to distinguish between the case
/// where no filters were matched and the case where at least one
/// filter was matched, along with the associated proof and filter set.
#[derive(Debug, Clone, PartialEq)]
pub enum HitFiltersType<'a> {
    /// Indicates that no filters were matched.
    NoFilterHit,

    /// Indicates that one or more filters were matched.
    ///
    /// Carries both the original proof from GroveDB as raw bytes,
    /// and references to the specific filters that were hit.
    DidHitFilters {
        /// The original proof data returned by GroveDB.
        original_grovedb_proof: Vec<u8>,

        /// The list of filters that matched during evaluation.
        filters_hit: Vec<&'a DriveSubscriptionFilter>,
    },
}

/// Wrapper enum for document and token subscription filters.

#[derive(Debug, Clone, PartialEq)]
pub enum DriveSubscriptionFilter {
    /// Contract Create and Update filter variant.
    Contract(DriveContractQueryFilter),
    /// Document subscription filter variant.
    Document(DriveDocumentQueryFilter),
    /// Token subscription filter variant.
    Token(DriveTokenQueryFilter),
}


impl DriveSubscriptionFilter {
    /// Returns categorised matches for the provided transitions.
    pub fn matches_any_transition<'a>(
        &self,
        transitions: &[&'a StateTransition],
    ) -> TransitionMatchSet<'a> {
        let mut matches = TransitionMatchSet::default();

        for transition in transitions {
            match self.matches_transition(transition) {
                TransitionCheckResult::Pass => matches.passes.push(*transition),
                TransitionCheckResult::NeedsOriginal => matches.needs_original.push(*transition),
                TransitionCheckResult::Fail => {}
            }
        }

        matches
    }

    /// Returns true when the transition satisfies this subscription filter.
    pub fn matches_transition(&self, state_transition: &StateTransition) -> TransitionCheckResult {
        match (self, state_transition) {
            (DriveSubscriptionFilter::Contract(filter), _) => {
                filter.matches_state_transition(state_transition)
            }
            (
                DriveSubscriptionFilter::Document(filter),
                StateTransition::Batch(batch_transition),
            ) => {
                let mut saw_needs_original = false;
                for transition_ref in batch_transition.transitions_iter() {
                    if let BatchedTransitionRef::Document(document_transition) = transition_ref {
                        match filter.matches_document_transition(document_transition, None) {
                            TransitionCheckResult::Pass => return TransitionCheckResult::Pass,
                            TransitionCheckResult::NeedsOriginal => saw_needs_original = true,
                            TransitionCheckResult::Fail => {}
                        }
                    }
                }

                if saw_needs_original {
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
            (DriveSubscriptionFilter::Token(filter), StateTransition::Batch(batch_transition)) => {
                let mut saw_needs_original = false;
                for transition_ref in batch_transition.transitions_iter() {
                    if let BatchedTransitionRef::Token(token_transition) = transition_ref {
                        match filter.matches_token_transition(token_transition) {
                            TransitionCheckResult::Pass => return TransitionCheckResult::Pass,
                            TransitionCheckResult::NeedsOriginal => saw_needs_original = true,
                            TransitionCheckResult::Fail => {}
                        }
                    }
                }

                if saw_needs_original {
                    TransitionCheckResult::NeedsOriginal
                } else {
                    TransitionCheckResult::Fail
                }
            }
            _ => TransitionCheckResult::Fail,
        }
    }

    /// Returns `true` when the provided document transition satisfies this filter.
    pub fn matches_document_transition(
        &self,
        transition: &DocumentTransition,
        batch_owner_value: Option<&Value>,
    ) -> bool {
        match self {
            DriveSubscriptionFilter::Contract(_) => false,
            DriveSubscriptionFilter::Document(filter) => {
                filter.matches_document_transition(transition, batch_owner_value)
                    != TransitionCheckResult::Fail
            }
            DriveSubscriptionFilter::Token(_) => false,
        }
    }

    /// Returns `true` when the provided token transition satisfies this filter.
    pub fn matches_token_transition(&self, transition: &TokenTransition) -> bool {
        match self {
            DriveSubscriptionFilter::Contract(_) => false,
            DriveSubscriptionFilter::Token(filter) => {
                filter.matches_token_transition(transition) != TransitionCheckResult::Fail
            }
            DriveSubscriptionFilter::Document(_) => false,
        }
    }

    /// Returns `true` when the provided contract update transition satisfies this filter after
    /// evaluating original contract dependent clauses.
    pub fn matches_contract_update_transition_original_contract(
        &self,
        original_contract: &DataContract,
    ) -> bool {
        match self {
            DriveSubscriptionFilter::Contract(filter) => filter.matches_original_contract(
                original_contract,
            ),
            DriveSubscriptionFilter::Document(_) | DriveSubscriptionFilter::Token(_) => false,
        }
    }
}
