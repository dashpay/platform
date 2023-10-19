use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::common::asset_lock::proof::AssetLockProofStateValidation;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::rpc::signature::CoreSignatureVerification;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    InvalidInstantAssetLockProofSignatureError,
};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

// TODO: Versioning
impl AssetLockProofStateValidation for InstantAssetLockProof {
    fn validate_state<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        // Make sure that asset lock isn't spent yet

        let Some(asset_lock_outpoint) = self.out_point() else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "asset lock outpoint must be present",
            )));
        };

        let is_already_spent = platform_ref.drive.has_asset_lock_outpoint(
            &asset_lock_outpoint,
            transaction,
            &platform_version.drive,
        )?;

        if is_already_spent {
            result.add_error(IdentityAssetLockTransactionOutPointAlreadyExistsError::new(
                asset_lock_outpoint.txid,
                asset_lock_outpoint.vout as usize,
            ))
        }

        // Verify instant lock signature with Core

        let is_instant_lock_signature_valid = self
            .instant_lock()
            .verify_signature(platform_ref.core_rpc, platform_ref.block_info.core_height)?;

        if !is_instant_lock_signature_valid {
            result.add_error(InvalidInstantAssetLockProofSignatureError::new());

            return Ok(result);
        }

        Ok(result)
    }
}
