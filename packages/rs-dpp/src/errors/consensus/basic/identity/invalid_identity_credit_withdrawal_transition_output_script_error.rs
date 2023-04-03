use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::basic::BasicError;
use crate::{consensus::ConsensusError};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Output script must be either p2pkh or p2sh")]
pub struct InvalidIdentityCreditWithdrawalTransitionOutputScriptError {}

impl InvalidIdentityCreditWithdrawalTransitionOutputScriptError {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionOutputScriptError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionOutputScriptError) -> Self {
        Self::BasicError(
            BasicError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(err),
        )
    }
}
