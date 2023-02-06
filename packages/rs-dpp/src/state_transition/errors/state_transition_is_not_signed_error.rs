use thiserror::Error;

use crate::state_transition::StateTransition;
use crate::ProtocolError;

#[derive(Error, Debug, Clone)]
#[error("State Transition is not signed")]
pub struct StateTransitionIsNotSignedError {
    state_transition: StateTransition,
}

impl StateTransitionIsNotSignedError {
    pub fn new(state_transition: StateTransition) -> Self {
        Self { state_transition }
    }

    pub fn state_transition(&self) -> StateTransition {
        self.state_transition.clone()
    }
}

impl From<StateTransitionIsNotSignedError> for ProtocolError {
    fn from(err: StateTransitionIsNotSignedError) -> Self {
        Self::StateTransitionIsNotSignedError(err)
    }
}
