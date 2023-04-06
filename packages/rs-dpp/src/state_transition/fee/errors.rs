use crate::state_transition::errors::StateTransitionError;
use crate::ProtocolError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeeError {
    #[error("not enough balance")]
    InsufficientBalance,
}

impl Into<ProtocolError> for FeeError {
    fn into(self) -> ProtocolError {
        ProtocolError::StateTransitionError(StateTransitionError::FeeError(self))
    }
}
