use drive::grovedb::Transaction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    #[inline(always)]
    pub(super) fn validate_token_aggregated_balance_v0(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if self.config.execution.verify_token_sum_trees {
            // Verify sum trees
            let token_balance = self
                .drive
                .calculate_total_tokens_balance(Some(transaction), platform_version)
                .map_err(Error::Drive)?;

            if !token_balance.ok()? {
                return Err(Error::Execution(
                    ExecutionError::CorruptedTokensNotBalanced(format!(
                        "tokens are not balanced after block execution {:?} off by {}",
                        token_balance,
                        token_balance
                            .total_identity_token_balances
                            .abs_diff(token_balance.total_tokens_in_platform)
                    )),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_validate_token_aggregated_balance_with_verification_enabled() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        // The default config should have verify_token_sum_trees enabled
        // On a fresh genesis state with no token operations, balances should be balanced
        let transaction = platform.drive.grove.start_transaction();

        let result = platform.validate_token_aggregated_balance_v0(&transaction, platform_version);

        assert!(
            result.is_ok(),
            "expected token balance validation to pass on fresh genesis state: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_token_aggregated_balance_with_verification_disabled() {
        let platform_version = PlatformVersion::latest();
        let mut config =
            crate::config::PlatformConfig::default_for_network(dpp::dashcore::Network::Regtest);
        config.execution.verify_token_sum_trees = false;

        let platform = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // With verification disabled, this should always succeed
        let result = platform.validate_token_aggregated_balance_v0(&transaction, platform_version);

        assert!(result.is_ok());
    }
}
