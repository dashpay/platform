use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::core_script::CoreScript;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Output script must be either p2pkh or p2sh")]
#[platform_serialize(unversioned)]
pub struct InvalidCreditWithdrawalTransitionOutputScriptError {
    output_script: CoreScript,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl InvalidCreditWithdrawalTransitionOutputScriptError {
    pub fn new(output_script: CoreScript) -> Self {
        Self { output_script }
    }

    pub fn output_script(&self) -> CoreScript {
        // TODO: We shouldn't clone in getter 🤦
        self.output_script.clone()
    }
}
impl From<InvalidCreditWithdrawalTransitionOutputScriptError> for ConsensusError {
    fn from(err: InvalidCreditWithdrawalTransitionOutputScriptError) -> Self {
        Self::BasicError(BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(err))
    }
}
