use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::fee::Credits;
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::AssetLockProofVerifyIsNotSpent;

use super::verify_asset_lock_is_not_spent_and_has_enough_balance;

// TODO: Versioning
impl AssetLockProofVerifyIsNotSpent for ChainAssetLockProof {
    fn verify_is_not_spent_and_has_enough_balance<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        signable_bytes_hasher: &mut SignableBytesHasher,
        required_balance: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
        verify_asset_lock_is_not_spent_and_has_enough_balance(
            platform_ref,
            signable_bytes_hasher,
            self.out_point,
            required_balance,
            transaction,
            platform_version,
        )
    }
}
