use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::dashcore::signer;
use dpp::dashcore::signer::double_sha;
use dpp::identity::state_transition::AssetLockProved;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransitionLike;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::StateTransitionAction;
//  TODO: Move to DPP
pub trait OneTimeSignedStateTransitionV0: StateTransitionLike + AssetLockProved {
    /// Validate the signature of the state transition with it's asset lock proof
    fn verify_one_time_signature_v0(
        &self,
        action: &StateTransitionAction,
        signable_bytes: Vec<u8>,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let output = match action {
            StateTransitionAction::IdentityCreateAction(action) => action.asset_lock_output(),
            StateTransitionAction::IdentityTopUpAction(action) => action.asset_lock_output(),
            _ => {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "unexpected action type",
                )))
            }
        };

        let public_key_hash = &output
            .script_pubkey
            .p2pkh_public_key_hash_bytes()
            .ok_or_else(|| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "output must be a valid p2pkh already",
                ))
            })?;

        let data_hash = double_sha(signable_bytes);

        match signer::verify_hash_signature(
            &data_hash,
            self.signature().as_slice(),
            public_key_hash,
        ) {
            Ok(_) => Ok(ConsensusValidationResult::new()),
            Err(e) => Ok(SimpleConsensusValidationResult::new_with_error(
                SignatureError::BasicECDSAError(BasicECDSAError::new(e.to_string())).into(),
            )),
        }
    }
}
