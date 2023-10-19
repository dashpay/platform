use crate::error::execution::ExecutionError;
use crate::error::Error;
use anyhow::bail;
use dpp::consensus::signature::{BasicECDSAError, SignatureError};
use dpp::dashcore::key::Secp256k1;
use dpp::dashcore::secp256k1::ecdsa::RecoverableSignature;
use dpp::dashcore::secp256k1::Message;
use dpp::dashcore::signer::{double_sha, ripemd160_sha256, CompactSignature};
use dpp::identity::state_transition::AssetLockProved;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::StateTransitionLike;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::StateTransitionAction;

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

        match verify_hash_signature(&data_hash, self.signature().as_slice(), public_key_hash) {
            Ok(_) => Ok(ConsensusValidationResult::new()),
            Err(e) => Ok(SimpleConsensusValidationResult::new_with_error(
                SignatureError::BasicECDSAError(BasicECDSAError::new(e.to_string())).into(),
            )),
        }
    }
}

pub fn verify_hash_signature(
    data_hash: &[u8],
    data_signature: &[u8],
    public_key_id: &[u8],
) -> Result<(), anyhow::Error> {
    let signature: RecoverableSignature =
        RecoverableSignature::from_compact_signature(data_signature)?;

    let secp = Secp256k1::new();
    let msg = Message::from_slice(data_hash).map_err(anyhow::Error::msg)?;
    let recovered_public_key = secp
        .recover_ecdsa(&msg, &signature)
        .map_err(anyhow::Error::msg)?;

    let recovered_compressed_public_key = recovered_public_key.serialize();
    let hash_recovered_key = ripemd160_sha256(&recovered_compressed_public_key);

    let are_equal = public_key_id == hash_recovered_key;

    if are_equal {
        Ok(())
    } else {
        bail!("the signature isn't valid")
    }
}
