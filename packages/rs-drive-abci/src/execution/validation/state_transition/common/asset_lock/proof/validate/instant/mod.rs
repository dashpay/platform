use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::rpc::signature::CoreSignatureVerification;
use dpp::consensus::basic::identity::InvalidInstantAssetLockProofSignatureError;
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;

// TODO: Versioning
impl AssetLockProofValidation for InstantAssetLockProof {
    fn validate<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        // Verify instant lock signature with Core

        let is_instant_lock_signature_valid = self
            .instant_lock()
            .verify_signature(platform_ref.core_rpc, platform_ref.block_info.core_height)?;

        if !is_instant_lock_signature_valid {
            result.add_error(InvalidInstantAssetLockProofSignatureError::new());

            return Ok(result);
        }

        self.verify_is_not_spent(platform_ref, transaction, platform_version)
    }
}
