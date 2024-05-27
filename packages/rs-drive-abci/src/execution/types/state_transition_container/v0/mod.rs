use dpp::consensus::ConsensusError;
use dpp::state_transition::StateTransition;
use dpp::ProtocolError;
use std::time::Duration;

/// Decoded state transition result
#[derive(Debug)]
pub enum DecodedStateTransition<'a> {
    SuccessfullyDecoded(SuccessfullyDecodedStateTransition<'a>),
    InvalidEncoding(InvalidStateTransition<'a>),
    FailedToDecode(InvalidWithProtocolErrorStateTransition<'a>),
}

/// Invalid encoded state transition
#[derive(Debug)]
pub struct InvalidStateTransition<'a> {
    pub raw: &'a [u8],
    pub error: ConsensusError,
    pub elapsed_time: Duration,
}

/// State transition that failed to decode
#[derive(Debug)]
pub struct InvalidWithProtocolErrorStateTransition<'a> {
    pub raw: &'a [u8],
    pub error: ProtocolError,
    pub elapsed_time: Duration,
}

/// Successfully decoded state transition
#[derive(Debug)]
pub struct SuccessfullyDecodedStateTransition<'a> {
    pub decoded: StateTransition,
    pub raw: &'a [u8],
    pub elapsed_time: Duration,
}

/// This is a container that holds state transitions
#[derive(Debug)]
pub struct StateTransitionContainerV0<'a> {
    // We collect all decoding results in the same vector because we want to
    // keep the original input order when we process them and log results we can
    // easily match with txs in block
    state_transitions: Vec<DecodedStateTransition<'a>>,
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

#[allow(clippy::from_over_into)]
impl<'a> Into<Vec<DecodedStateTransition<'a>>> for StateTransitionContainerV0<'a> {
    fn into(self) -> Vec<DecodedStateTransition<'a>> {
        self.state_transitions
    }
}
