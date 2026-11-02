use crate::drive::identity::withdrawals::paths::get_withdrawal_core_credit_pool_balances_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::util::object_size_info::PathKeyElementInfo;
use dpp::fee::Credits;
use grovedb::{Element, TransactionArg};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn record_core_credit_pool_blocks_v0(
        &self,
        balances: &[(u32, Credits)],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if balances.is_empty() {
            return Ok(());
        }

        let mut drive_operations = vec![];

        for (core_height, credit_pool_balance) in balances {
            self.batch_insert(
                PathKeyElementInfo::PathKeyElement::<0>((
                    get_withdrawal_core_credit_pool_balances_path_vec(),
                    core_height.to_be_bytes().to_vec(),
                    Element::new_item(credit_pool_balance.to_be_bytes().to_vec()),
                )),
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
