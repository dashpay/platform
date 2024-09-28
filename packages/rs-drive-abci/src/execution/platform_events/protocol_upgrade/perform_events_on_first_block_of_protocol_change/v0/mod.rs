use crate::error::Error;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use drive::drive::RootTree;
use drive::grovedb::{Element, Transaction};

impl<C> Platform<C> {
    /// checks for a network upgrade and resets activation window
    /// this should only be called on epoch change
    pub(super) fn perform_events_on_first_block_of_protocol_change_v0(
        &self,
        epoch_info: &EpochInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if previous_protocol_version < 4 && platform_version.protocol_version >= 4 {
            let path = get_withdrawal_root_path();
            self.drive.grove_insert_if_not_exists(
                (&path).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                Element::empty_sum_tree(),
                Some(transaction),
                None,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
