use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::basic::identity::{
    IdentityAssetLockTransactionOutPointAlreadyExistsError,
};
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::platform_value::Bytes36;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;

// TODO: Versioning
impl AssetLockProofVerifyIsNotSpent for ChainAssetLockProof {
    fn verify_is_not_spent<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        // Make sure that asset lock isn't spent yet

        let outpoint_bytes = self.out_point.try_into().map_err(|e| {
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
