use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::identity::{
    InvalidAssetLockProofCoreChainHeightError,
};
use dpp::fee::Credits;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::validate::AssetLockProofValidation;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

// TODO: Versioning
impl AssetLockProofValidation for ChainAssetLockProof {
    fn validate<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        required_balance: Credits,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
        if validation_mode != ValidationMode::RecheckTx
            && platform_ref.state.last_committed_core_height() < self.core_chain_locked_height
        {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidAssetLockProofCoreChainHeightError::new(
                    self.core_chain_locked_height,
                    platform_ref.state.last_committed_core_height(),
                )
                .into(),
            ));
        }

        self.verify_is_not_spent_and_has_enough_balance(
            platform_ref,
            required_balance,
            transaction,
            platform_version,
        )
    }
}
