mod chain;
mod instant;

use crate::error::Error;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::fee::Credits;
use dpp::prelude::AssetLockProof;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

/// A trait for validating state transitions within a blockchain.
pub trait AssetLockProofValidation {
    /// Validates the state transition by analyzing the changes in the platform state after applying the transaction.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform containing the state data.
    /// * `tx` - The transaction argument to be applied.
    ///
    /// # Type Parameters
    ///
    /// * `C: CoreRPCLike` - A type constraint indicating that C should implement `CoreRPCLike`.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        required_balance: Credits,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error>;
}

impl AssetLockProofValidation for AssetLockProof {
    fn validate<C: CoreRPCLike>(
        &self,
        platform_ref: &PlatformRef<C>,
        required_balance: Credits,
        validation_mode: ValidationMode,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<AssetLockValue>, Error> {
        match self {
            AssetLockProof::Instant(proof) => proof.validate(
                platform_ref,
                required_balance,
                validation_mode,
                transaction,
                platform_version,
            ),
            AssetLockProof::Chain(proof) => proof.validate(
                platform_ref,
                required_balance,
                validation_mode,
                transaction,
                platform_version,
            ),
        }
    }
}
