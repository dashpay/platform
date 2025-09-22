use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, StateTransitionContainerV0,
};
use derive_more::From;
use dpp::state_transition::StateTransition;
use drive::drive::subscriptions::{DriveSubscriptionFilter, TransitionMatchSet};
use std::collections::BTreeMap;

/// Aggregated filter usage information across decoded state transitions.
pub struct FilterUsage<'b> {
    /// Transitions that passed without fetching originals and the filters that matched them.
    pub passing: TransitionFilterMap<'b>,
    /// Transitions that require original documents and the filters that requested them.
    pub requiring_original_to_know: TransitionFilterMap<'b>,
}

pub type TransitionFilterMap<'b> = BTreeMap<usize, Vec<&'b DriveSubscriptionFilter>>;

pub(crate) mod v0;

#[derive(Debug, From)]
pub enum StateTransitionContainer<'a> {
    V0(StateTransitionContainerV0<'a>),
}

impl<'a> IntoIterator for &'a StateTransitionContainer<'a> {
    type Item = &'a DecodedStateTransition<'a>;
    type IntoIter = std::slice::Iter<'a, DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            StateTransitionContainer::V0(v0) => v0.into_iter(),
        }
    }
}

impl<'a> IntoIterator for StateTransitionContainer<'a> {
    type Item = DecodedStateTransition<'a>;
    type IntoIter = std::vec::IntoIter<DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            StateTransitionContainer::V0(v0) => v0.into_iter(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl<'a> Into<Vec<DecodedStateTransition<'a>>> for StateTransitionContainer<'a> {
    fn into(self) -> Vec<DecodedStateTransition<'a>> {
        match self {
            StateTransitionContainer::V0(v0) => v0.into(),
        }
    }
}

impl StateTransitionContainer<'_> {
    /// This function narrows the filters to filters that could match in the block
    pub fn find_used_filters<'b>(&self, filters: &'b [DriveSubscriptionFilter]) -> FilterUsage<'b> {
        let state_transitions = self.successfully_decoded_state_transitions();
        let mut transition_indices = BTreeMap::new();
        for (index, transition) in state_transitions.iter().enumerate() {
            transition_indices.insert((*transition) as *const StateTransition, index);
        }

        let mut passing = TransitionFilterMap::new();
        let mut requiring_original = TransitionFilterMap::new();

        for filter in filters {
            let TransitionMatchSet {
                passes,
                needs_original,
            } = filter.matches_any_transition(&state_transitions);

            if passes.is_empty() && needs_original.is_empty() {
                continue;
            }

            for transition in passes {
                if let Some(&index) = transition_indices.get(&(transition as *const _)) {
                    push_filter_for_transition(&mut passing, index, filter);
                }
            }

            for transition in needs_original {
                if let Some(&index) = transition_indices.get(&(transition as *const _)) {
                    push_filter_for_transition(&mut requiring_original, index, filter);
                }
            }
        }

        FilterUsage {
            passing,
            requiring_original_to_know: requiring_original,
        }
    }

    /// Returns references to each successfully decoded state transition.
    pub fn successfully_decoded_state_transitions(&self) -> Vec<&StateTransition> {
        match self {
            StateTransitionContainer::V0(container) => {
                container.successfully_decoded_state_transitions()
            }
        }
    }
}

fn push_filter_for_transition<'b>(
    collection: &mut TransitionFilterMap<'b>,
    index: usize,
    filter: &'b DriveSubscriptionFilter,
) {
    collection.entry(index).or_default().push(filter);
}
