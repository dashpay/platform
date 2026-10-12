use crate::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use crate::drive::system::misc_path;
use crate::drive::tokens::paths::{tokens_root_path, TOKEN_BALANCES_KEY, TOKEN_SHIELDED_POOLS_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::total_tokens_balance::TotalTokensBalance;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Version 1 (protocol version 14): the token shielded pools BigSumTree joins the identity
    /// balances on the balance side, so `identity balances + shielded pool balances` must equal
    /// the total supplies. Shielding moves tokens between the two terms without touching supply.
    #[inline(always)]
    pub(super) fn calculate_total_tokens_balance_v1(
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

        let total_token_shielded_pool_balances = self.grove_get_big_sum_tree_total_value(
            (&tokens_root_path).into(),
            &[TOKEN_SHIELDED_POOLS_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(TotalTokensBalance {
            total_tokens_in_platform,
            total_identity_token_balances,
            total_token_shielded_pool_balances,
        })
    }
}
