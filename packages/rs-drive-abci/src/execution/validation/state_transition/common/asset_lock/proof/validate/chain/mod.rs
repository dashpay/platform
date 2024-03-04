use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::identity::{
    InvalidAssetLockProofCoreChainHeightError,
};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;

// TODO: Versioning
impl AssetLockProofValidation for ChainAssetLockProof {
    fn validate<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        if platform_ref.last_committed_block_info.core_height < self.core_chain_locked_height {
            result.add_error(InvalidAssetLockProofCoreChainHeightError::new(
                self.core_chain_locked_height,
                platform_ref.last_committed_block_info.core_height,
            ));

            return Ok(result);
        }

        self.verify_is_not_spent(platform_ref, transaction, platform_version)
    }
}
