use crate::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use crate::drive::system::misc_path;
use crate::drive::tokens::paths::{tokens_root_path, TOKEN_BALANCES_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::total_tokens_balance::TotalTokensBalance;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verify that the sum tree identity credits + pool credits + refunds are equal to the
    /// Total credits in the system
    #[inline(always)]
    pub(super) fn calculate_total_tokens_balance_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<TotalTokensBalance, Error> {
        let mut drive_operations = vec![];
        let path_holding_total_credits = misc_path();
        let total_tokens_in_platform = self.grove_get_big_sum_tree_total_value(
            (&path_holding_total_credits).into(),
            TOTAL_TOKEN_SUPPLIES_STORAGE_KEY,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let tokens_root_path = tokens_root_path();

        let total_identity_token_balances = self.grove_get_big_sum_tree_total_value(
            (&tokens_root_path).into(),
            &[TOKEN_BALANCES_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(TotalTokensBalance {
            total_tokens_in_platform,
            total_identity_token_balances,
        })
    }
}
