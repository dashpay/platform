use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_core_credit_pool_balances_path_vec,
    get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec,
    get_withdrawal_pending_asset_lock_inflows_path,
};
use crate::drive::identity::withdrawals::{
    core_dated_credit_inflow_key, PendingAssetLockCreditInflow,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::{BatchDeleteApplyType, BatchInsertApplyType, DirectQueryType};
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::{Credits, SignedCredits};
use grovedb::{Element, MaybeTree, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn record_core_credit_pool_block_v0(
        &self,
        core_height: u32,
        credit_pool_balance: Credits,
        asset_lock_txids: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let window_min_blocks = platform_version
            .system_limits
            .core_credit_pool_window_min_blocks
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "record_core_credit_pool_block v0 requires system_limits.core_credit_pool_window_min_blocks",
            )))?;

        let mut drive_operations = vec![];

        self.batch_insert(
            PathKeyElementInfo::PathKeyElement::<0>((
                get_withdrawal_core_credit_pool_balances_path_vec(),
                core_height.to_be_bytes().to_vec(),
                Element::new_item(credit_pool_balance.to_be_bytes().to_vec()),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Core counts an asset lock in full for its window after the block that mined it, so
        // the credits of one Platform consumed early count from this block, like those of one
        // Core had mined already; past the window they no longer count at all.
        let expires_at_core_height = core_height.saturating_add(window_min_blocks);
        let still_counts = expires_at_core_height > block_info.core_height;

        let pending_path = get_withdrawal_pending_asset_lock_inflows_path();

        // Summed per key first: two asset locks minted in one Platform block and mined in one
        // Core block share a key, and the add-or-insert below reads the stored value, not the
        // operations queued in this batch.
        let mut dated_inflows: BTreeMap<Vec<u8>, Credits> = BTreeMap::new();

        for txid in asset_lock_txids {
            let Some(element) = self.grove_get_raw_optional(
                (&pending_path).into(),
                txid,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?
            else {
                // Not consumed by Platform yet, or consumed after Core mined it (then it was
                // dated when it was consumed).
                continue;
            };

            let Element::Item(value, _) = element else {
                return Err(Error::Drive(DriveError::CorruptedElementType(
                    "pending asset lock credit inflow is not an item",
                )));
            };
            let pending = PendingAssetLockCreditInflow::from_bytes(&value)?;

            self.batch_delete(
                (&pending_path).into(),
                txid,
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;

            if still_counts {
                let total = dated_inflows
                    .entry(core_dated_credit_inflow_key(
                        expires_at_core_height,
                        pending.recorded_at_time_ms,
                    ))
                    .or_default();
                *total = total.checked_add(pending.amount).ok_or(Error::Drive(
                    DriveError::CriticalCorruptedState("core-dated credit inflows overflow"),
                ))?;
            }
        }

        let dated_path = get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec();
        for (key, amount) in dated_inflows {
            let amount = SignedCredits::try_from(amount).map_err(|_| {
                Error::Drive(DriveError::CriticalCorruptedState(
                    "core-dated credit inflow does not fit a sum item",
                ))
            })?;
            self.batch_insert_sum_item_or_add_to_if_already_exists(
                PathKeyElementInfo::PathKeyElement::<0>((
                    dated_path.clone(),
                    key,
                    Element::SumItem(amount, None),
                )),
                BatchInsertApplyType::StatefulBatchInsert,
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }

        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            drive_operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }
}
