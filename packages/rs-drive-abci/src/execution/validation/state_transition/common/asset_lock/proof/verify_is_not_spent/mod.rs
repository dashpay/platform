mod chain;
mod instant;

use crate::error::Error;
use crate::platform_types::platform::PlatformRef;

use dpp::prelude::AssetLockProof;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

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
    fn verify_is_not_spent<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl AssetLockProofVerifyIsNotSpent for AssetLockProof {
    fn verify_is_not_spent<C>(
        &self,
        platform_ref: &PlatformRef<C>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match self {
            AssetLockProof::Instant(proof) => {
                proof.verify_is_not_spent(platform_ref, transaction, platform_version)
            }
            AssetLockProof::Chain(proof) => {
                proof.verify_is_not_spent(platform_ref, transaction, platform_version)
            }
        }
    }
}
