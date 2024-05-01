use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::rpc::signature::CoreSignatureVerification;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
use dpp::fee::Credits;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

// TODO: Versioning
impl AssetLockProofValidation for InstantAssetLockProof {
    fn validate<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        signable_bytes_hasher: &mut SignableBytesHasher,
        required_balance: Credits,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
        // Verify instant lock signature with Core

        let validation_result = self.verify_is_not_spent_and_has_enough_balance(
            platform_ref,
            signable_bytes_hasher,
            required_balance,
            transaction,
            platform_version,
        )?;

        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // If we have a partially spent asset lock then we do not need to verify the signature of the instant lock
        // As we know this outpoint was already considered final and locked.

        if validation_mode != ValidationMode::RecheckTx && !validation_result.has_data() {
            let is_instant_lock_signature_valid = self.instant_lock().verify_signature(
                platform_ref.core_rpc,
                platform_ref.state.last_committed_core_height(),
            )?;

            if !is_instant_lock_signature_valid {
                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidInstantAssetLockProofSignatureError::new().into(),
                ));
            }
        }

        Ok(validation_result)
    }
}
