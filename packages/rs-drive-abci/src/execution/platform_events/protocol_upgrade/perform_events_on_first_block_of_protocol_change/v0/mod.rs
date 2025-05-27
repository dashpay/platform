use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::data_contracts::SystemDataContract;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::serialization::PlatformDeserializable;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::VotePoll;
use drive::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use drive::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_voting_path_vec;
use drive::drive::system::misc_path;
use drive::drive::tokens::paths::{
    token_distributions_root_path, token_timed_distributions_path, tokens_root_path,
    TOKEN_BALANCES_KEY, TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY, TOKEN_CONTRACT_INFO_KEY,
    TOKEN_DIRECT_SELL_PRICE_KEY, TOKEN_DISTRIBUTIONS_KEY, TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY,
    TOKEN_IDENTITY_INFO_KEY, TOKEN_MS_TIMED_DISTRIBUTIONS_KEY, TOKEN_PERPETUAL_DISTRIBUTIONS_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, TOKEN_STATUS_INFO_KEY, TOKEN_TIMED_DISTRIBUTIONS_KEY,
};
use drive::drive::votes::paths::vote_end_date_queries_tree_path_vec;
use drive::drive::RootTree;
use drive::grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, Transaction};
use drive::grovedb_path::SubtreePath;
use drive::query::QueryResultType;
use platform_version::version::ProtocolVersion;
use std::collections::HashSet;
use std::ops::RangeFull;

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
        self.drive
            .cache
            .system_data_contracts
            .reload_system_contracts(platform_version)?;
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

        if previous_protocol_version < 8 && platform_version.protocol_version >= 8 {
            self.transition_to_version_8(block_info, transaction, platform_version)
                .or_else(|e| {
                    tracing::error!(
                        error = ?e,
                        "Error while transitioning to version 8: {e}"
                    );

                    // We ignore this transition errors because it's not changing the state structure
                    // and not critical for the system
                    Ok::<(), Error>(())
                })?;
        }

        if previous_protocol_version < 9 && platform_version.protocol_version >= 9 {
            self.transition_to_version_9(block_info, transaction, platform_version)?;
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

    /// When transitioning to version 8 we need to empty some specialized balances
    fn transition_to_version_8(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Let's start by getting all the specialized balances that exist
        let path_holding_specialized_balances =
            prefunded_specialized_balances_for_voting_path_vec();
        let path_query = PathQuery::new_single_query_item(
            path_holding_specialized_balances,
            QueryItem::RangeFull(RangeFull),
        );
        let all_specialized_balances_still_around: HashSet<_> = self
            .drive
            .grove_get_path_query(
                &path_query,
                Some(transaction),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )?
            .0
            .to_keys()
            .into_iter()
            .map(Identifier::try_from)
            .collect::<Result<HashSet<_>, dpp::platform_value::Error>>()?;

        let path = vote_end_date_queries_tree_path_vec();

        let mut query = Query::new_with_direction(true);

        query.insert_all();

        let mut sub_query = Query::new();

        sub_query.insert_all();

        query.default_subquery_branch.subquery = Some(sub_query.into());

        let current_votes_path_query = PathQuery {
            path,
            query: SizedQuery {
                query,
                limit: Some(30000), //Just a high number that shouldn't break the system
                offset: None,
            },
        };

        let (query_result_elements, _) = self.drive.grove_get_path_query(
            &current_votes_path_query,
            Some(transaction),
            QueryResultType::QueryElementResultType,
            &mut vec![],
            &platform_version.drive,
        )?;

        let active_specialized_balances = query_result_elements
            .to_elements()
            .into_iter()
            .map(|element| {
                let contested_document_resource_vote_poll_bytes = element
                    .into_item_bytes()
                    .map_err(drive::error::Error::GroveDB)?;
                let vote_poll =
                    VotePoll::deserialize_from_bytes(&contested_document_resource_vote_poll_bytes)?;
                match vote_poll {
                    VotePoll::ContestedDocumentResourceVotePoll(contested) => {
                        contested.specialized_balance_id().map_err(Error::Protocol)
                    }
                }
            })
            .collect::<Result<HashSet<Identifier>, Error>>()?;

        // let's get the non-active ones
        let non_active_specialized_balances =
            all_specialized_balances_still_around.difference(&active_specialized_balances);

        let mut total_credits_to_add_to_processing: Credits = 0;

        let mut operations = vec![];

        for specialized_balance_id in non_active_specialized_balances {
            let (credits, mut empty_specialized_balance_operation) =
                self.drive.empty_prefunded_specialized_balance_operations(
                    *specialized_balance_id,
                    false,
                    &mut None,
                    Some(transaction),
                    platform_version,
                )?;
            operations.append(&mut empty_specialized_balance_operation);
            total_credits_to_add_to_processing = total_credits_to_add_to_processing
                .checked_add(credits)
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "Credits from specialized balances are overflowing",
                )))?;
        }

        if total_credits_to_add_to_processing > 0 {
            operations.push(
                self.drive
                    .add_epoch_processing_credits_for_distribution_operation(
                        &block_info.epoch,
                        total_credits_to_add_to_processing,
                        Some(transaction),
                        platform_version,
                    )?,
            );
        }

        if !operations.is_empty() {
            self.drive.apply_batch_low_level_drive_operations(
                None,
                Some(transaction),
                operations,
                &mut vec![],
                &platform_version.drive,
            )?;
        }

        Ok(())
    }

    /// Adds all trees needed for tokens, also adds the token history system data contract
    ///
    /// This function is called during the transition from protocol version 5 to protocol version 6
    /// and higher to set up the wallet contract in the platform.
    fn transition_to_version_9(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.grove_insert_empty_tree(
            SubtreePath::empty(),
            &[RootTree::GroupActions as u8],
            Some(transaction),
            None,
            &mut vec![],
            &platform_version.drive,
        )?;

        // The root token trees

        let path = tokens_root_path();
        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_BALANCES_KEY],
            Element::empty_big_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_IDENTITY_INFO_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_STATUS_INFO_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_DIRECT_SELL_PRICE_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            &[TOKEN_CONTRACT_INFO_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // The token distribution trees

        let token_distributions_path = token_distributions_root_path();

        self.drive.grove_insert_if_not_exists(
            (&token_distributions_path).into(),
            &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&token_distributions_path).into(),
            &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&token_distributions_path).into(),
            &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // The token time distribution trees
        let timed_distributions_path = token_timed_distributions_path();

        self.drive.grove_insert_if_not_exists(
            (&timed_distributions_path).into(),
            &[TOKEN_MS_TIMED_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&timed_distributions_path).into(),
            &[TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            (&timed_distributions_path).into(),
            &[TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // The token total supply

        let path = misc_path();
        self.drive.grove_insert_if_not_exists(
            (&path).into(),
            TOTAL_TOKEN_SUPPLIES_STORAGE_KEY.as_slice(),
            Element::empty_big_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        let token_history_contract =
            load_system_data_contract(SystemDataContract::TokenHistory, platform_version)?;

        self.drive.insert_contract(
            &token_history_contract,
            *block_info,
            true,
            Some(transaction),
            platform_version,
        )?;

        let search_contract =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)?;

        self.drive.insert_contract(
            &search_contract,
            *block_info,
            true,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}
