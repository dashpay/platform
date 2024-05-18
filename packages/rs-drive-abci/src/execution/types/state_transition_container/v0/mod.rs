use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use std::time::Duration;

#[derive(Debug)]
pub enum DecodedStateTransition<'a> {
    SuccessfullyDecoded(SuccessfullyDecodedStateTransition<'a>),
    InvalidEncoding(InvalidEncodedStateTransition<'a>),
    FailedToDecode(FaultyStateTransition<'a>),
}

#[derive(Debug)]
pub struct InvalidEncodedStateTransition<'a> {
    pub raw: &'a [u8],
    pub error: ConsensusError,
    pub elapsed_time: Duration,
}

#[derive(Debug)]
pub struct FaultyStateTransition<'a> {
    pub raw: &'a [u8],
    pub error: ProtocolError,
    pub elapsed_time: Duration,
}

#[derive(Debug)]
pub struct SuccessfullyDecodedStateTransition<'a> {
    pub decoded: StateTransition,
    pub raw: &'a [u8],
    pub elapsed_time: Duration,
}

/// This is a container that holds state transitions

#[derive(Debug)]
pub struct StateTransitionContainerV0<'a> {
    state_transitions: Vec<DecodedStateTransition<'a>>,
}

pub trait StateTransitionContainerGettersV0<'a> {
    fn into_vec(self) -> Vec<DecodedStateTransition<'a>>;
}

impl<'a> StateTransitionContainerV0<'a> {
    pub fn new(state_transitions: Vec<DecodedStateTransition<'a>>) -> Self {
        Self { state_transitions }
    }
}

impl<'a> IntoIterator for &'a StateTransitionContainerV0<'a> {
    type Item = &'a DecodedStateTransition<'a>;
    type IntoIter = std::slice::Iter<'a, DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.state_transitions.iter()
    }
}

impl<'a> IntoIterator for StateTransitionContainerV0<'a> {
    type Item = DecodedStateTransition<'a>;
    type IntoIter = std::vec::IntoIter<DecodedStateTransition<'a>>;

    fn into_iter(self) -> Self::IntoIter {
        self.state_transitions.into_iter()
    }
}

impl<'a> StateTransitionContainerGettersV0<'a> for StateTransitionContainerV0<'a> {
    fn into_vec(self) -> Vec<DecodedStateTransition<'a>> {
        self.state_transitions
    }
}
