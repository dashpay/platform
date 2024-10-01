use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use drive::grovedb::{Element, Transaction};

impl<C> Platform<C> {
    /// Executes protocol-specific events on the first block after a protocol version change.
    ///
    /// This function is triggered when there is a protocol version upgrade detected in the network.
    /// It checks if the current protocol version has transitioned from an earlier version to version 4,
    /// and if so, performs the necessary setup or migration tasks associated with version 4.
    ///
    /// Currently, the function handles the transition to version 4 by initializing new structures
    /// or states required for the new protocol version.
    ///
    /// # Parameters
    ///
    /// * `transaction`: A reference to the transaction context in which the changes should be applied.
    /// * `previous_protocol_version`: The protocol version prior to the upgrade.
    /// * `platform_version`: The current platform version containing the updated protocol version and relevant configuration details.
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If all events related to the protocol change were successfully executed.
    /// * `Err(Error)`: If there was an issue executing the protocol-specific events.
    pub(super) fn perform_events_on_first_block_of_protocol_change_v0(
        &self,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if previous_protocol_version < 4 && platform_version.protocol_version >= 4 {
            self.transition_to_version_4(transaction, platform_version)?;
        }

        Ok(())
    }

    /// Initializes an empty sum tree for withdrawal transactions required for protocol version 4.
    ///
    /// This function is called during the transition to protocol version 4 to set up
    /// an empty sum tree at the specified path if it does not already exist.
    ///
    /// # Parameters
    ///
    /// * `transaction`: A reference to the transaction context in which the changes should be applied.
    /// * `platform_version`: The current platform version containing the updated protocol version and relevant configuration details.
    ///
    /// # Returns
    ///
    /// * `Ok(())`: If the transition to version 4 was successful.
    /// * `Err(Error)`: If there was an issue creating or updating the necessary data structures.
    fn transition_to_version_4(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path = get_withdrawal_root_path();
        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
            Element::empty_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;
        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;
        Ok(())
    }
}
