use crate::drive::identity::withdrawals::paths::{
    get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec,
    get_withdrawal_pending_asset_lock_inflows_path,
    get_withdrawal_pending_asset_lock_inflows_path_vec,
};
use crate::drive::identity::withdrawals::{
    core_dated_credit_inflow_key, PendingAssetLockCreditInflow,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::grove_operations::{BatchInsertApplyType, DirectQueryType};
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::{Credits, SignedCredits};
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn record_asset_lock_credit_inflow_v0(
        &self,
        asset_lock_txid: [u8; 32],
        amount: Credits,
        mined_at_core_height: Option<u32>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if amount == 0 {
            return Ok(());
        }

        let mut drive_operations = vec![];

        match mined_at_core_height {
            Some(mined_at_core_height) => {
                let window_min_blocks = platform_version
                    .system_limits
                    .core_credit_pool_window_min_blocks
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "record_asset_lock_credit_inflow v0 requires system_limits.core_credit_pool_window_min_blocks",
                    )))?;

                let expires_at_core_height = mined_at_core_height.saturating_add(window_min_blocks);
                if expires_at_core_height <= block_info.core_height {
                    // Core mined it a whole window ago: its credits sit in the balance Core's
                    // limit starts from, and adding them again would grant budget Core does not.
                    return Ok(());
                }

                let amount = SignedCredits::try_from(amount).map_err(|_| {
                    Error::Drive(DriveError::CriticalCorruptedState(
                        "core-dated credit inflow does not fit a sum item",
                    ))
                })?;

                self.batch_insert_sum_item_or_add_to_if_already_exists(
                    PathKeyElementInfo::PathKeyElement::<0>((
                        get_withdrawal_core_dated_credit_inflows_sum_tree_path_vec(),
                        core_dated_credit_inflow_key(expires_at_core_height, block_info.time_ms),
                        Element::SumItem(amount, None),
                    )),
                    BatchInsertApplyType::StatefulBatchInsert,
                    transaction,
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
            }
            None => {
                let pending_path = get_withdrawal_pending_asset_lock_inflows_path();

                // Another mint of the same asset lock may already wait (a partly used asset
                // lock): add to it, keeping when it was first recorded.
                let pending = match self.grove_get_raw_optional(
                    (&pending_path).into(),
                    &asset_lock_txid,
                    DirectQueryType::StatefulDirectQuery,
                    transaction,
                    &mut vec![],
                    &platform_version.drive,
                )? {
                    Some(Element::Item(value, _)) => {
                        let mut pending = PendingAssetLockCreditInflow::from_bytes(&value)?;
                        pending.amount = pending.amount.checked_add(amount).ok_or(Error::Drive(
                            DriveError::CriticalCorruptedState(
                                "pending asset lock credit inflow overflow",
                            ),
                        ))?;
                        pending
                    }
                    Some(_) => {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "pending asset lock credit inflow is not an item",
                        )))
                    }
                    None => PendingAssetLockCreditInflow {
                        amount,
                        recorded_at_time_ms: block_info.time_ms,
                        recorded_at_core_height: block_info.core_height,
                    },
                };

                self.batch_insert(
                    PathKeyElementInfo::PathKeyElement::<0>((
                        get_withdrawal_pending_asset_lock_inflows_path_vec(),
                        asset_lock_txid.to_vec(),
                        Element::new_item(pending.to_bytes()),
                    )),
                    &mut drive_operations,
                    &platform_version.drive,
                )?;
            }
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

#[cfg(test)]
mod tests {
    use crate::drive::identity::withdrawals::paths::get_withdrawal_pending_asset_lock_inflows_path;
    use crate::drive::identity::withdrawals::PendingAssetLockCreditInflow;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn should_add_a_second_mint_to_a_pending_asset_lock_and_keep_when_it_was_first_recorded() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        for (time_ms, core_height, amount) in
            [(1_000u64, 50u32, 300u64), (2_000, 51, 200), (3_000, 52, 0)]
        {
            drive
                .record_asset_lock_credit_inflow(
                    [5; 32],
                    amount,
                    None,
                    &BlockInfo {
                        time_ms,
                        core_height,
                        ..Default::default()
                    },
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to record the inflow");
        }

        let element = drive
            .grove_get_raw_optional(
                (&get_withdrawal_pending_asset_lock_inflows_path()).into(),
                &[5; 32],
                DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to read")
            .expect("expected the pending entry");
        let Element::Item(value, _) = element else {
            panic!("expected an item");
        };
        assert_eq!(
            PendingAssetLockCreditInflow::from_bytes(&value).expect("expected to decode"),
            PendingAssetLockCreditInflow {
                amount: 500,
                recorded_at_time_ms: 1_000,
                recorded_at_core_height: 50,
            }
        );
    }

    #[test]
    fn should_round_trip_a_pending_entry_and_refuse_other_lengths() {
        let pending = PendingAssetLockCreditInflow {
            amount: u64::MAX,
            recorded_at_time_ms: 7,
            recorded_at_core_height: u32::MAX,
        };
        assert_eq!(
            PendingAssetLockCreditInflow::from_bytes(&pending.to_bytes()).expect("decodes"),
            pending
        );
        assert!(PendingAssetLockCreditInflow::from_bytes(&[0; 19]).is_err());
    }
}
