use crate::execution::types::state_transition_container::v0::{
    StateTransitionContainerGettersV0, StateTransitionContainerV0,
};
use derive_more::From;
use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;

pub(crate) mod v0;

#[derive(Debug, From)]
pub enum StateTransitionContainer<'a> {
    V0(StateTransitionContainerV0<'a>),
}
impl<'a> StateTransitionContainerGettersV0<'a> for StateTransitionContainer<'a> {
    fn valid_state_transitions(&'a self) -> &'a [(&'a Vec<u8>, StateTransition)] {
        match self {
            StateTransitionContainer::V0(container) => &container.valid_state_transitions,
        }
    }

    fn invalid_state_transitions(&'a self) -> &'a [(&'a Vec<u8>, ConsensusError)] {
        match self {
            StateTransitionContainer::V0(container) => &container.invalid_state_transitions,
        }
    }

    fn invalid_state_transitions_with_protocol_error(
        &'a self,
    ) -> &'a [(&'a Vec<u8>, ProtocolError)] {
        match self {
            StateTransitionContainer::V0(container) => {
                &container.invalid_state_transitions_with_protocol_error
            }
        }
    }

    // The destructure method's signature and return type need to be adjusted to match the trait.
    fn destructure(
        self,
    ) -> (
        Vec<(&'a Vec<u8>, StateTransition)>,
        Vec<(&'a Vec<u8>, ConsensusError)>,
        Vec<(&'a Vec<u8>, ProtocolError)>,
    ) {
        match self {
            StateTransitionContainer::V0(container) => (
                container.valid_state_transitions,
                container.invalid_state_transitions,
                container.invalid_state_transitions_with_protocol_error,
            ),
        }
    }
}
