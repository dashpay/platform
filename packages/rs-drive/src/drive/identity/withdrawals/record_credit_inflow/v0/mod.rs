use crate::drive::identity::withdrawals::paths::get_withdrawal_credit_inflows_sum_tree_path_vec;
use crate::drive::identity::withdrawals::DAY_AND_A_HOUR_IN_MS;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::grove_operations::BatchInsertApplyType;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::{Credits, SignedCredits};
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn record_credit_inflow_v0(
        &self,
        amount: Credits,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if amount == 0 {
            return Ok(());
        }

        let mut drive_operations = vec![];

        // Keyed by when the inflow stops counting toward the limit, on the same 25 hour
        // schedule the withdrawal reservations expire on, and pruned by the same per-block
        // cleanup. Saturating: a block time near the end of u64 must not panic, and an entry
        // that never expires only ever raises the limit toward the gross cap. Adding to an
        // existing entry keeps one entry per block time even if a caller records twice.
        let expiration_date = block_info.time_ms.saturating_add(DAY_AND_A_HOUR_IN_MS);

        self.batch_insert_sum_item_or_add_to_if_already_exists(
            PathKeyElementInfo::PathKeyElement::<0>((
                get_withdrawal_credit_inflows_sum_tree_path_vec(),
                expiration_date.to_be_bytes().to_vec(),
                Element::SumItem(amount as SignedCredits, None),
            )),
            BatchInsertApplyType::StatefulBatchInsert,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

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
