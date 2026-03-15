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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;

    #[test]
    fn test_dispatcher_unknown_version_returns_error() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .tokens_processing
            .validate_token_aggregated_balance = 255;

        let result = platform.validate_token_aggregated_balance(&transaction, &modified_version);

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            })) => {
                assert_eq!(method, "validate_token_aggregated_balance");
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 255);
            }
            _ => panic!("expected UnknownVersionMismatch error"),
        }
    }
}
