use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::data_contracts::SystemDataContract;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
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
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        previous_protocol_version: ProtocolVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if previous_protocol_version < 4 && platform_version.protocol_version >= 4 {
            self.transition_to_version_4(
                platform_state,
                block_info,
                transaction,
                platform_version,
            )?;
        }

        if previous_protocol_version < 6 && platform_version.protocol_version >= 6 {
            self.transition_to_version_6(block_info, transaction, platform_version)?;
        }

        Ok(())
    }

    /// Initializes the wallet contract that supports mobile wallets with additional
    /// functionality
    ///
    /// This function is called during the transition from protocol version 5 to protocol version 6
    /// and higher to set up the wallet contract in the platform.
    fn transition_to_version_6(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let contract =
            load_system_data_contract(SystemDataContract::WalletUtils, platform_version)?;

        self.drive.insert_contract(
            &contract,
            *block_info,
            true,
            Some(transaction),
            platform_version,
        )?;

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
        platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // We are adding the withdrawal transactions sum amount tree
        let path = get_withdrawal_root_path();
        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
            Element::empty_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;
        // We are adding a tree to store broadcasted transactions that might expire
        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;
        // We need to add all masternode owner keys
        // This is because owner identities only had a withdrawal key
        // But no owner key
        for masternode in platform_state.full_masternode_list().values() {
            let masternode_id = masternode.pro_tx_hash.to_byte_array();
            let key_request = IdentityKeysRequest {
                identity_id: masternode_id,
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let old_owner_identity_keys = self
                .drive
                .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                    key_request,
                    Some(transaction),
                    platform_version,
                )?;

            if old_owner_identity_keys.is_empty() {
                continue;
            }

            let last_key_id = *old_owner_identity_keys
                .keys()
                .max()
                .expect("there must be keys, we already checked");

            let new_owner_key = Self::get_owner_identity_owner_key(
                masternode.state.owner_address,
                last_key_id + 1,
                platform_version,
            )?;

            tracing::trace!(
                identity_id = ?masternode_id,
                withdrawal_key = ?new_owner_key,
                method = "transition_to_version_4",
                "add new owner key to owner identity"
            );

            self.drive.add_new_non_unique_keys_to_identity(
                masternode_id,
                vec![new_owner_key],
                block_info,
                true,
                Some(transaction),
                platform_version,
            )?;
        }
        Ok(())
    }
}
