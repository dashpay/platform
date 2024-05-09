use crate::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::system::misc_path;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::balances::total_credits_balance::TotalCreditsBalance;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;
use grovedb_path::SubtreePath;

impl Drive {
    /// Verify that the sum tree identity credits + pool credits + refunds are equal to the
    /// Total credits in the system
    #[inline(always)]
    pub(super) fn calculate_total_credits_balance_v0(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<TotalCreditsBalance, Error> {
        let mut drive_operations = vec![];
        let path_holding_total_credits = misc_path();
        let total_credits_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&path_holding_total_credits).into(),
                TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut drive_operations,
                drive_version,
            )?
            .ok_or(Error::Drive(DriveError::CriticalCorruptedState(
                "Credits not found in Platform",
            )))?;

        let total_identity_balances = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::Balances),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        let total_in_pools = self.grove_get_sum_tree_total_value(
            SubtreePath::empty(),
            Into::<&[u8; 1]>::into(RootTree::Pools),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(TotalCreditsBalance {
            total_credits_in_platform,
            total_in_pools,
            total_identity_balances,
        })
    }
}
