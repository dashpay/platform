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
