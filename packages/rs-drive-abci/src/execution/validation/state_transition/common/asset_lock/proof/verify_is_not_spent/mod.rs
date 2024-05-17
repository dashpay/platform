mod chain;
mod instant;
mod v0;

use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::dashcore::OutPoint;
use dpp::fee::Credits;

use dpp::prelude::AssetLockProof;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::common::asset_lock::proof::verify_is_not_spent::v0::verify_asset_lock_is_not_spent_and_has_enough_balance_v0;

/// A trait for validating that an asset lock is not spent
pub trait AssetLockProofVerifyIsNotSpent {
    /// Validates that the asset lock was not spent
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `transaction` - The database transaction to check on, can be None.
    /// * `platform_version` - The platform version that we are using
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn verify_is_not_spent_and_has_enough_balance<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        signable_bytes_hasher: &mut SignableBytesHasher,
        required_balance: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error>;
}

impl AssetLockProofVerifyIsNotSpent for AssetLockProof {
    fn verify_is_not_spent_and_has_enough_balance<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        signable_bytes_hasher: &mut SignableBytesHasher,
        required_balance: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
        match self {
            AssetLockProof::Instant(proof) => proof.verify_is_not_spent_and_has_enough_balance(
                platform_ref,
                signable_bytes_hasher,
                required_balance,
                transaction,
                platform_version,
            ),
            AssetLockProof::Chain(proof) => proof.verify_is_not_spent_and_has_enough_balance(
                platform_ref,
                signable_bytes_hasher,
                required_balance,
                transaction,
                platform_version,
            ),
        }
    }
}

#[inline(always)]
fn verify_asset_lock_is_not_spent_and_has_enough_balance<C>(
    platform_ref: &PlatformRef<C>,
    signable_bytes_hasher: &mut SignableBytesHasher,
    out_point: OutPoint,
    required_balance: Credits,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .common_validation_methods
        .asset_locks
        .verify_asset_lock_is_not_spent_and_has_enough_balance
    {
        0 => verify_asset_lock_is_not_spent_and_has_enough_balance_v0(
            platform_ref,
            signable_bytes_hasher,
            out_point,
            required_balance,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "verify_asset_lock_is_not_spent_and_has_enough_balance".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
