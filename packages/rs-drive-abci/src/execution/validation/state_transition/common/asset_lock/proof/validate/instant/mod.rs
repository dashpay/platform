use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
use dpp::fee::Credits;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::platform_events::core_instant_send_lock::verify_recent_signature_locally::VerifyInstantLockSignature;
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;
use crate::execution::validation::state_transition::ValidationMode;

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
            // We should be able to disable instant lock versification for integration tests
            #[cfg(feature = "testing-config")]
            if platform_ref
                .config
                .testing_configs
                .disable_instant_lock_signature_verification
            {
                return Ok(validation_result);
            }

            // TODO: Shouldn't we add an operation for fees?

            // This is a limited verification and will work properly only for recently signed instant locks.
            // Even valid instant locks that was signed some time ago will be considered invalid due to limited
            // quorum information in the platform state. In turn, this verification doesn't use Core RPC or any other
            // IO. This is done to prevent DoS attacks on slow verify instant lock signature Core RPC method.
            // In case of failed signature verification (or any knowing the fact that signing quorum is old),
            // we expect clients to use ChainAssetLockProof.
            let is_valid = self
                .instant_lock()
                .verify_recent_signature_locally(platform_ref.state, platform_version)?;

            if !is_valid {
                return Ok(ConsensusValidationResult::new_with_error(
                    InvalidInstantAssetLockProofSignatureError::new().into(),
                ));
            }
        }

        Ok(validation_result)
    }
}
