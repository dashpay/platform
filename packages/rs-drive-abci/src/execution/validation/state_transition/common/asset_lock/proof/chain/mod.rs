use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::common::asset_lock::proof::AssetLockProofStateValidation;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
    InvalidAssetLockProofCoreChainHeightError,
};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::platform_value::Bytes36;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

// TODO: Versioning
impl AssetLockProofStateValidation for ChainAssetLockProof {
    fn validate_state<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        if platform_ref.block_info.core_height < self.core_chain_locked_height {
            result.add_error(InvalidAssetLockProofCoreChainHeightError::new(
                self.core_chain_locked_height,
                platform_ref.block_info.core_height,
            ));

            return Ok(result);
        }

        // Make sure that asset lock isn't spent yet

        let outpoint_bytes = self.out_point.try_into().map_err(|_e| {
            Error::Execution(ExecutionError::Conversion(String::from(
                "can't convert output to bytes",
            )))
        })?;

        let is_already_spent = platform_ref.drive.has_asset_lock_outpoint(
            &Bytes36::new(outpoint_bytes),
            transaction,
            &platform_version.drive,
        )?;

        if is_already_spent {
            result.add_error(IdentityAssetLockTransactionOutPointAlreadyExistsError::new(
                self.out_point.txid,
                self.out_point.vout as usize,
            ))
        }

        Ok(result)
    }
}
