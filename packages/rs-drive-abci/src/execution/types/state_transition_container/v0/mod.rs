use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;

/// This is a container that holds state transitions

#[derive(Debug, Default)]
pub struct StateTransitionContainerV0<'a> {
    /// The asset lock state transitions
    pub(super) valid_state_transitions: Vec<(&'a Vec<u8>, StateTransition)>,
    /// Deserialization errors
    pub(super) invalid_state_transitions: Vec<(&'a Vec<u8>, ConsensusError)>,
    /// Deserialization errors that broke platform, these should not exist, but are still handled
    pub(super) invalid_state_transitions_with_protocol_error: Vec<(&'a Vec<u8>, ProtocolError)>,
}

pub trait StateTransitionContainerGettersV0<'a> {
    fn valid_state_transitions(&'a self) -> &'a [(&'a Vec<u8>, StateTransition)];
    fn invalid_state_transitions(&'a self) -> &'a [(&'a Vec<u8>, ConsensusError)];
    fn invalid_state_transitions_with_protocol_error(
        &'a self,
    ) -> &'a [(&'a Vec<u8>, ProtocolError)];

    fn destructure(
        self,
    ) -> (
        Vec<(&'a Vec<u8>, StateTransition)>,
        Vec<(&'a Vec<u8>, ConsensusError)>,
        Vec<(&'a Vec<u8>, ProtocolError)>,
    );
}

impl<'a> StateTransitionContainerV0<'a> {
    pub fn push_valid_state_transition(
        &mut self,
        raw_state_transition: &'a Vec<u8>,
        valid_state_transition: StateTransition,
    ) {
        self.valid_state_transitions
            .push((raw_state_transition, valid_state_transition))
    }

    pub fn push_invalid_raw_state_transition(
        &mut self,
        invalid_raw_state_transition: &'a Vec<u8>,
        error: ConsensusError,
    ) {
        self.invalid_state_transitions
            .push((invalid_raw_state_transition, error))
    }

    pub fn push_invalid_raw_state_transition_with_protocol_error(
        &mut self,
        invalid_raw_state_transition: &'a Vec<u8>,
        error: ProtocolError,
    ) {
        self.invalid_state_transitions_with_protocol_error
            .push((invalid_raw_state_transition, error))
    }
}
