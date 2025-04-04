mod v0;

use dpp::version::PlatformVersion;

use drive::grovedb::Transaction;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

impl<C> Platform<C> {
    /// Validates the aggregated token balance for the platform.
    ///
    /// This function verifies that the total token balances in the platform are consistent
    /// and correctly aggregated. It delegates the validation to a version-specific implementation
    /// based on the `PlatformVersion` provided.
    ///
    /// # Arguments
    ///
    /// * `transaction` - A reference to the current transaction.
    /// * `platform_version` - The platform version specifying the implementation to use.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the token balances are validated successfully.
    /// * `Err(Error)` if the validation fails due to inconsistencies or an unknown version.
    ///
    /// # Errors
    ///
    /// Returns an `ExecutionError::CorruptedCreditsNotBalanced` if the token sum trees
    /// are not balanced.
    ///
    /// Returns an `ExecutionError::UnknownVersionMismatch` if the platform version is not recognized.
    pub fn validate_token_aggregated_balance(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .tokens_processing
            .validate_token_aggregated_balance
        {
            0 => self.validate_token_aggregated_balance_v0(transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "validate_token_aggregated_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
