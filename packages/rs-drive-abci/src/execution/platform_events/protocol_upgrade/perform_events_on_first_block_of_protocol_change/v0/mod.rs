use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::dashcore::hashes::Hash;
use dpp::data_contracts::SystemDataContract;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::reduced_platform_state::v0::ReducedBlockInfoV0;
use dpp::serialization::PlatformDeserializable;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use dpp::version::ProtocolVersion;
use dpp::voting::vote_polls::VotePoll;
use drive::drive::address_funds::queries::CLEAR_ADDRESS_POOL_U8;
use drive::drive::balances::TOTAL_TOKEN_SUPPLIES_STORAGE_KEY;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap, KeyRequestType,
};
use drive::drive::identity::withdrawals::paths::{
    get_withdrawal_root_path, WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
    WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY, WITHDRAWAL_TRANSACTIONS_BROADCASTED_KEY,
    WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
};
use drive::drive::prefunded_specialized_balances::prefunded_specialized_balances_for_voting_path_vec;
use drive::drive::saved_block_transactions::{
    ADDRESS_BALANCES_KEY_U8, COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8,
    COMPACTED_ADDRESS_BALANCES_KEY_U8,
};
use drive::drive::system::misc_path;
use drive::drive::tokens::paths::{
    token_distributions_root_path, token_timed_distributions_path, tokens_root_path,
    TOKEN_BALANCES_KEY, TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY, TOKEN_CONTRACT_INFO_KEY,
    TOKEN_DIRECT_SELL_PRICE_KEY, TOKEN_DISTRIBUTIONS_KEY, TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY,
    TOKEN_IDENTITY_INFO_KEY, TOKEN_MS_TIMED_DISTRIBUTIONS_KEY, TOKEN_PERPETUAL_DISTRIBUTIONS_KEY,
    TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, TOKEN_STATUS_INFO_KEY, TOKEN_TIMED_DISTRIBUTIONS_KEY,
};
use drive::drive::votes::paths::vote_end_date_queries_tree_path_vec;
use drive::drive::{Drive, RootTree};
use drive::grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, Transaction, TreeType};
use drive::grovedb_path::SubtreePath;
use drive::query::QueryResultType;
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

        if previous_protocol_version < 11 && platform_version.protocol_version >= 11 {
            self.transition_to_version_11(transaction, platform_version)?;
        }

        if previous_protocol_version < 12 && platform_version.protocol_version >= 12 {
            self.transition_to_version_12(transaction, platform_version)?;
        }

        if previous_protocol_version < 13 && platform_version.protocol_version >= 13 {
            self.transition_to_version_13(block_info, transaction, platform_version)?;
        }

        if previous_protocol_version < 14 && platform_version.protocol_version >= 14 {
            self.transition_to_version_14(block_info, transaction, platform_version)?;
        }

        if previous_protocol_version < 15 && platform_version.protocol_version >= 15 {
            self.transition_to_version_15(platform_state, transaction, platform_version)?;
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
                    .map_err(drive::error::Error::from)?;
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
            TreeType::NormalTree,
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

    /// We introduced in version 11 Addresses
    fn transition_to_version_11(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.drive.grove_insert_if_not_exists(
            SubtreePath::empty(),
            &[RootTree::AddressBalances as u8],
            Element::empty_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        let path = Drive::addresses_path();
        self.drive.grove_insert_if_not_exists(
            path.as_slice().into(),
            &[CLEAR_ADDRESS_POOL_U8],
            Element::empty_provable_count_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // SavedBlockTransactions for address-based transaction sync
        self.drive.grove_insert_if_not_exists(
            SubtreePath::empty(),
            &[RootTree::SavedBlockTransactions as u8],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // Address balances subtree under SavedBlockTransactions
        let saved_block_path = Drive::saved_block_transactions_path();
        self.drive.grove_insert_if_not_exists(
            saved_block_path.as_slice().into(),
            &[ADDRESS_BALANCES_KEY_U8],
            Element::empty_count_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            saved_block_path.as_slice().into(),
            &[COMPACTED_ADDRESS_BALANCES_KEY_U8],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        self.drive.grove_insert_if_not_exists(
            saved_block_path.as_slice().into(),
            &[COMPACTED_ADDRESSES_EXPIRATION_TIME_KEY_U8],
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        Ok(())
    }

    /// We introduced in version 12 Shielded Pools.
    ///
    /// Builds the same `[ShieldedBalances]` subtree as a fresh genesis-12 chain
    /// (`Drive::create_initial_state_structure_v3`): a top-level
    /// `ShieldedBalances` SumTree containing the main shielded credit pool at
    /// `MAIN_SHIELDED_CREDIT_POOL_KEY`, and all eight pool subtrees. The pool and
    /// its children are built by the shared, sequential
    /// `Drive::insert_shielded_pool_structure` helper that the genesis path also
    /// calls — this is what guarantees a state-synced genesis-v12 node and an
    /// in-place-upgraded v12 node produce a byte-identical subtree root hash.
    fn transition_to_version_12(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Top-level ShieldedBalances SumTree — separate from AddressBalances so
        // per-pool internal trees cannot contaminate the address-credit
        // aggregate via sum propagation. Inserted here (not in the shared
        // helper) so it matches the genesis path, which creates this top-level
        // tree as a standalone non-batch insert before filling in the pool.
        self.drive.grove_insert_if_not_exists(
            SubtreePath::empty(),
            &[RootTree::ShieldedBalances as u8],
            Element::empty_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // Main shielded credit pool + its eight child subtrees, built by the
        // SHARED sequential builder. CONSENSUS-CRITICAL: the genesis-v12 path
        // (`Drive::create_initial_state_structure_v3`) calls this exact same
        // helper, so both paths root the pool's parent Merk at
        // `SHIELDED_NOTES_KEY` (`[128]`) and produce a byte-identical
        // `[ShieldedBalances]` subtree.
        self.drive
            .insert_shielded_pool_structure(Some(transaction), platform_version)?;

        // Strip unknown top-level properties from all contract document type schemas.
        // The v1 document meta-schema enforces additionalProperties: false at the
        // document-type level.  Contracts created under the v0 meta-schema (which did
        // NOT forbid unknown keys) may carry stale properties that would fail
        // validation under v1.  We clean them up here so every stored contract
        // conforms to the v1 meta-schema going forward.
        self.drive
            .strip_unknown_document_schema_properties(transaction, &platform_version.drive)?;

        Ok(())
    }

    /// When transitioning to version 13 we register the document history
    /// contract, and re-store the DPNS contract whose v2 schema subscribes the
    /// `domain` document type to transfer, purchase and pricing history.
    fn transition_to_version_13(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let document_history_contract =
            load_system_data_contract(SystemDataContract::DocumentHistory, platform_version)?;

        self.drive.insert_contract(
            &document_history_contract,
            *block_info,
            true,
            Some(transaction),
            platform_version,
        )?;

        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS, platform_version)?;

        self.drive.apply_contract(
            &dpns_contract,
            *block_info,
            true,
            None,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }

    /// When transitioning to version 14 we re-store the DashPay contract whose
    /// v2 schema adds the optional public payment address fields to the
    /// `profile` document type (DIP-33).
    fn transition_to_version_14(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let dashpay_contract =
            load_system_data_contract(SystemDataContract::Dashpay, platform_version)?;

        self.drive.apply_contract(
            &dashpay_contract,
            *block_info,
            true,
            None,
            Some(transaction),
            platform_version,
        )?;

        // Total credits history under the withdrawals tree: the daily withdrawal limit becomes
        // a share of the total credits Platform held a day ago, recorded here every block.
        self.drive.grove_insert_if_not_exists(
            get_withdrawal_root_path().as_slice().into(),
            &WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY,
            Element::empty_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        // Credit inflows sum tree: every credit mint is recorded here so the daily withdrawal
        // limit counts net outflow instead of gross — credits that entered Platform within the
        // window may leave again without consuming the withdrawal budget of other users.
        self.drive.grove_insert_if_not_exists(
            get_withdrawal_root_path().as_slice().into(),
            &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
            Element::empty_sum_tree(),
            Some(transaction),
            None,
            &platform_version.drive,
        )?;

        Ok(())
    }

    /// When transitioning to version 15 we write the initial reduced platform state (built
    /// from the last committed platform state) under `Misc/reduced_saved_state`, so the key
    /// exists in the replicated state from the fork block onward. `run_block_proposal` v1
    /// overwrites it later in this same block with the state of the block being processed;
    /// this initial write guarantees no v15 block ever commits without the key, which is
    /// what makes every snapshot taken at or after activation restorable via state sync.
    fn transition_to_version_15(
        &self,
        platform_state: &PlatformState,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let last_committed_block_info =
            platform_state
                .last_committed_block_info()
                .as_ref()
                .map(|extended_block_info| ReducedBlockInfoV0 {
                    basic_info: *extended_block_info.basic_info(),
                    app_hash: Some((*extended_block_info.app_hash()).into()),
                    quorum_hash: (*extended_block_info.quorum_hash()).into(),
                    block_id_hash: Some((*extended_block_info.block_id_hash()).into()),
                    proposer_pro_tx_hash: (*extended_block_info.proposer_pro_tx_hash()).into(),
                    signature: Some(*extended_block_info.signature()),
                    round: extended_block_info.round(),
                });

        let reduced_platform_state = platform_state.to_reduced_platform_state(
            last_committed_block_info,
            platform_state.last_committed_core_height(),
        );

        self.store_reduced_platform_state(
            &reduced_platform_state,
            Some(transaction),
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use drive::drive::shielded::paths::{
        shielded_credit_pool_path, MAIN_SHIELDED_CREDIT_POOL_KEY_U8, SHIELDED_ANCHORS_IN_POOL_KEY,
        SHIELDED_NOTES_KEY, SHIELDED_NULLIFIERS_KEY,
    };

    /// Recursively compares the GroveDB subtree rooted at `root_path` between
    /// two platforms and returns a list of human-readable differences (empty ⇒
    /// the two subtrees are byte-identical at every node, including every
    /// element's carried `root_key`, sum/count, value bytes, and the full set of
    /// child keys at every depth).
    ///
    /// This is the rigorous subtree-equality used by the boundary equivalence
    /// tests. We deliberately compare a *named subtree* rather than the whole-DB
    /// root hash: the whole DB also carries the genesis epoch's recorded
    /// protocol-version field (`[Pools, epoch_0, "v"]`), which legitimately
    /// differs between a chain *born* at vN and a chain born at v(N-1) then
    /// upgraded — that field is unrelated to how the boundary subtrees are
    /// constructed and must not pollute the equivalence check.
    fn collect_subtree_diffs(
        platform_genesis: &crate::platform_types::platform::Platform<
            crate::rpc::core::MockCoreRPCLike,
        >,
        platform_upgraded: &crate::platform_types::platform::Platform<
            crate::rpc::core::MockCoreRPCLike,
        >,
        upgraded_txn: &drive::grovedb::Transaction,
        root_path: Vec<Vec<u8>>,
    ) -> Vec<String> {
        use drive::grovedb::{PathQuery, Query, SizedQuery};
        use drive::query::QueryResultType;

        fn read_level(
            platform: &crate::platform_types::platform::Platform<crate::rpc::core::MockCoreRPCLike>,
            txn: drive::grovedb::TransactionArg,
            path: &[Vec<u8>],
        ) -> std::collections::BTreeMap<Vec<u8>, Element> {
            let mut q = Query::new();
            q.insert_all();
            let pq = PathQuery::new(path.to_vec(), SizedQuery::new(q, None, None));
            // A *query error* (as opposed to a legitimately empty subtree, which
            // returns Ok with no items) must fail the equivalence guard loudly —
            // swallowing it into an empty map could let `collect_subtree_diffs`
            // report "no diffs" when one side was actually unreadable, producing
            // a false GREEN. An empty Ok result correctly yields an empty map.
            let (results, _) = platform
                .drive
                .grove_get_raw_path_query(
                    &pq,
                    txn,
                    QueryResultType::QueryKeyElementPairResultType,
                    &mut vec![],
                    &platform
                        .state
                        .load()
                        .current_platform_version()
                        .expect("platform version")
                        .drive,
                )
                .unwrap_or_else(|e| {
                    panic!(
                        "equivalence guard: subtree read at path {:?} must succeed, got: {e}",
                        path.iter().map(hex::encode).collect::<Vec<_>>()
                    )
                });
            results.to_key_elements().into_iter().collect()
        }

        fn walk(
            platform_genesis: &crate::platform_types::platform::Platform<
                crate::rpc::core::MockCoreRPCLike,
            >,
            platform_upgraded: &crate::platform_types::platform::Platform<
                crate::rpc::core::MockCoreRPCLike,
            >,
            upgraded_txn: &drive::grovedb::Transaction,
            path: Vec<Vec<u8>>,
            depth: usize,
            diffs: &mut Vec<String>,
        ) {
            // Shielded/address subtrees are shallow; 16 is a safe recursion cap
            // that still covers the deepest commitment-tree internals.
            if depth > 16 {
                return;
            }
            let a_map = read_level(platform_genesis, None, &path);
            let b_map = read_level(platform_upgraded, Some(upgraded_txn), &path);

            let path_hex: Vec<String> = path.iter().map(hex::encode).collect();
            for (k, ea) in &a_map {
                match b_map.get(k) {
                    Some(eb) if eb == ea => {}
                    other => diffs.push(format!(
                        "path={:?} key={}\n  genesis={:?}\n  upgrade={:?}",
                        path_hex,
                        hex::encode(k),
                        ea,
                        other
                    )),
                }
            }
            for k in b_map.keys() {
                if !a_map.contains_key(k) {
                    diffs.push(format!(
                        "ONLY IN upgraded: path={:?} key={}",
                        path_hex,
                        hex::encode(k)
                    ));
                }
            }
            // Recurse only into subtrees present in both with a matching element.
            for (k, ea) in &a_map {
                let is_tree = matches!(
                    ea,
                    Element::Tree(..)
                        | Element::SumTree(..)
                        | Element::BigSumTree(..)
                        | Element::CountTree(..)
                        | Element::CountSumTree(..)
                        | Element::ProvableCountTree(..)
                        | Element::ProvableCountSumTree(..)
                );
                if is_tree && b_map.get(k) == Some(ea) {
                    let mut child = path.clone();
                    child.push(k.clone());
                    walk(
                        platform_genesis,
                        platform_upgraded,
                        upgraded_txn,
                        child,
                        depth + 1,
                        diffs,
                    );
                }
            }
        }

        let mut diffs = Vec::new();
        walk(
            platform_genesis,
            platform_upgraded,
            upgraded_txn,
            root_path,
            0,
            &mut diffs,
        );
        diffs
    }

    #[test]
    fn test_same_version_transition_is_noop() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // When previous == current, no transition_to_version_* should be triggered
        let result = platform.perform_events_on_first_block_of_protocol_change_v0(
            &platform_state,
            &block_info,
            &transaction,
            platform_version.protocol_version,
            platform_version,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_transition_to_version_6_inserts_wallet_utils_contract() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // Transition from version 5 to current (which is >= 6)
        // should insert the wallet utils contract
        let result = platform.transition_to_version_6(&block_info, &transaction, platform_version);

        assert!(result.is_ok());

        // Verify the contract was inserted by loading it
        let wallet_utils_contract = dpp::system_data_contracts::load_system_data_contract(
            dpp::data_contracts::SystemDataContract::WalletUtils,
            platform_version,
        )
        .expect("expected to load wallet utils contract");

        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        let (_fee_result, contract_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *wallet_utils_contract.id().as_bytes(),
                None,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch contract");
        assert!(
            contract_fetch_info.is_some(),
            "WalletUtils contract should exist after transition_to_version_6"
        );
    }

    #[test]
    fn test_transition_to_version_13_inserts_document_history_contract_and_updates_dpns() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

        // A chain born at protocol version 12: DPNS v1 is stored and the
        // domain document type is not subscribed to history
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(12)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_version = PlatformVersion::get(13).expect("expected platform version 13");

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // The PV12 genesis must NOT contain the document history contract:
        // it only comes into existence through this transition
        let (_fee_result, pre_upgrade_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::DocumentHistory
                    .id()
                    .as_bytes(),
                None,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch contract");
        assert!(
            pre_upgrade_fetch_info.is_none(),
            "DocumentHistory contract must not exist before transition_to_version_13"
        );

        let result = platform.transition_to_version_13(&block_info, &transaction, platform_version);

        assert!(result.is_ok(), "transition failed: {:?}", result.err());

        // The document history contract must exist in the state
        let (_fee_result, contract_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::DocumentHistory
                    .id()
                    .as_bytes(),
                None,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch contract");
        assert!(
            contract_fetch_info.is_some(),
            "DocumentHistory contract should exist after transition_to_version_13"
        );

        // The stored DPNS contract must now be v2: the domain document type
        // subscribes to transfer, purchase and pricing history
        let (_fee_result, dpns_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::DPNS
                    .id()
                    .as_bytes(),
                None,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch DPNS contract");

        let dpns_fetch_info = dpns_fetch_info.expect("expected the DPNS contract to exist");

        let domain = dpns_fetch_info
            .contract
            .document_type_for_name("domain")
            .expect("expected the domain document type");

        assert!(domain.documents_keep_transfer_history());
        assert!(domain.documents_keep_purchase_history());
        assert!(domain.documents_keep_pricing_history());
    }

    #[test]
    fn test_transition_to_version_14_updates_dashpay_with_payment_address_fields() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

        // A chain born at protocol version 13: DashPay v1 is stored and the
        // profile document type has no payment address fields.
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_version = PlatformVersion::get(14).expect("expected platform version 14");

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // Before the transition the stored DashPay profile must be v1: no
        // corePaymentAddress / platformPaymentAddress properties.
        let (_fee_result, pre_upgrade_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::Dashpay
                    .id()
                    .as_bytes(),
                None,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch DashPay contract");
        let pre_profile = pre_upgrade_fetch_info
            .expect("expected the DashPay contract to exist at genesis")
            .contract
            .document_type_for_name("profile")
            .expect("expected the profile document type")
            .properties()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert!(
            !pre_profile.iter().any(|p| p == "corePaymentAddress"),
            "profile must not carry corePaymentAddress before transition_to_version_14"
        );
        assert!(
            !pre_profile.iter().any(|p| p == "platformPaymentAddress"),
            "profile must not carry platformPaymentAddress before transition_to_version_14"
        );

        let result = platform.transition_to_version_14(&block_info, &transaction, platform_version);
        assert!(result.is_ok(), "transition failed: {:?}", result.err());

        // After the transition the stored DashPay contract must be v2: the
        // profile document type gains the two optional payment address fields.
        let (_fee_result, dashpay_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::Dashpay
                    .id()
                    .as_bytes(),
                None,
                false,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch DashPay contract");

        let profile = dashpay_fetch_info
            .expect("expected the DashPay contract to exist")
            .contract
            .document_type_for_name("profile")
            .expect("expected the profile document type")
            .properties()
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        assert!(
            profile.iter().any(|p| p == "corePaymentAddress"),
            "profile must carry corePaymentAddress after transition_to_version_14"
        );
        assert!(
            profile.iter().any(|p| p == "platformPaymentAddress"),
            "profile must carry platformPaymentAddress after transition_to_version_14"
        );
    }

    /// The v13→v14 boundary through the production dispatcher
    /// (`perform_events_on_first_block_of_protocol_change`), with a v1 profile
    /// stored before the upgrade and the contract cache warmed: the dispatcher
    /// must select the v14 transition, the refreshed contract must carry both
    /// payment address fields, and the legacy profile bytes must stay readable
    /// through normal Drive queries against the refreshed contract.
    #[tokio::test]
    async fn test_protocol_change_v13_to_v14_upgrades_dashpay_and_keeps_v1_profiles_readable() {
        use crate::execution::validation::state_transition::tests::setup_identity;
        use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
        use assert_matches::assert_matches;
        use dpp::dash_to_credits;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::document_type::random_document::{
            CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
        };
        use dpp::document::{DocumentV0Getters, DocumentV0Setters};
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::platform_value::Bytes32;
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
        use dpp::state_transition::batch_transition::BatchTransition;
        use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
        use drive::query::DriveDocumentQuery;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let platform_version_13 = PlatformVersion::get(13).expect("expected platform version 13");
        let platform_version_14 = PlatformVersion::get(14).expect("expected platform version 14");

        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();

        let platform_state = platform.state.load();
        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        // Store a v1 profile through the normal pipeline at protocol version 13
        let dashpay_v1 = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version_13)
            .expect("expected the dashpay system contract");
        let profile_v1 = dashpay_v1
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(438);
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = profile_v1
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version_13,
            )
            .expect("expected a random v1 profile document");
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        let stored_profile_id = document.id();

        let create_transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            profile_v1,
            entropy.0,
            &key,
            2,
            0,
            None,
            &signer,
            platform_version_13,
            None,
        )
        .await
        .expect("expect to create documents batch transition");
        let create_serialized = create_transition
            .serialize_to_bytes()
            .expect("expected serialized transition");

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![create_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version_13,
                false,
                None,
            )
            .expect("expected to process state transition");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Warm the drive contract cache with the v1 contract and confirm the
        // payment address fields are absent pre-upgrade
        let transaction = platform.drive.grove.start_transaction();
        let (_fee, pre_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::Dashpay
                    .id()
                    .as_bytes(),
                None,
                true,
                Some(&transaction),
                platform_version_13,
            )
            .expect("expected to fetch DashPay contract");
        let pre_profile_properties = pre_fetch_info
            .expect("expected the DashPay contract pre-upgrade")
            .contract
            .document_type_for_name("profile")
            .expect("expected the profile document type")
            .properties()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for field in ["corePaymentAddress", "platformPaymentAddress"] {
            assert!(
                !pre_profile_properties.iter().any(|p| p == field),
                "profile must not carry {field} before the upgrade"
            );
        }

        // Run the v13→v14 boundary through the production dispatcher
        let block_info = BlockInfo {
            time_ms: 2_000_000,
            height: 200,
            core_height: 200,
            epoch: Epoch::new(1).expect("expected epoch"),
        };
        platform
            .perform_events_on_first_block_of_protocol_change(
                &platform_state,
                &block_info,
                &transaction,
                13,
                platform_version_14,
            )
            .expect("expected the protocol change events to succeed");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the upgrade");

        // The refreshed contract must be v2 with both payment address fields
        let (_fee, post_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *dpp::data_contracts::SystemDataContract::Dashpay
                    .id()
                    .as_bytes(),
                None,
                true,
                None,
                platform_version_14,
            )
            .expect("expected to fetch DashPay contract post-upgrade");
        let dashpay_v2_fetch_info = post_fetch_info.expect("expected the DashPay contract");
        let post_profile_properties = dashpay_v2_fetch_info
            .contract
            .document_type_for_name("profile")
            .expect("expected the profile document type")
            .properties()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for field in ["corePaymentAddress", "platformPaymentAddress"] {
            assert!(
                post_profile_properties.iter().any(|p| p == field),
                "profile must carry {field} after the upgrade"
            );
        }

        // The pre-upgrade profile must remain readable through normal Drive
        // queries against the refreshed v2 contract
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from profile",
            &dashpay_v2_fetch_info.contract,
            Some(&platform.config.drive),
            PlatformVersion::get(14).expect("expected platform version 14"),
        )
        .expect("expected a document query");
        let query_results = platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query documents");
        let documents = query_results.documents();
        assert_eq!(
            documents.len(),
            1,
            "the v1 profile must survive the upgrade"
        );
        assert_eq!(
            documents.first().expect("expected a document").id(),
            stored_profile_id,
            "the surviving profile must be the pre-upgrade document"
        );
    }

    // test_transition_to_version_9 removed: requires prior state from versions 4-8

    #[test]
    fn test_transition_to_version_11_creates_address_trees() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let result = platform.transition_to_version_11(&transaction, platform_version);

        assert!(result.is_ok());

        // Verify that the AddressBalances root tree was created
        use drive::drive::RootTree;
        use drive::grovedb::Element;
        use drive::grovedb_path::SubtreePath;
        let element = platform.drive.grove.get(
            SubtreePath::empty(),
            &[RootTree::AddressBalances as u8],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            element.value.is_ok(),
            "AddressBalances root tree should exist"
        );
    }

    #[test]
    fn test_transition_to_version_14_creates_total_credits_history_tree() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        use drive::grovedb_path::SubtreePath;

        // Not there on a v13 genesis state
        for key in [
            &WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY,
            &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
        ] {
            assert!(platform
                .drive
                .grove
                .get(
                    SubtreePath::from(&get_withdrawal_root_path()),
                    key,
                    Some(&transaction),
                    &platform_version.drive.grove_version,
                )
                .value
                .is_err());
        }

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 10,
            epoch: Epoch::default(),
        };
        platform
            .transition_to_version_14(&block_info, &transaction, platform_version)
            .expect("expected the transition to succeed");

        let element = platform
            .drive
            .grove
            .get(
                SubtreePath::from(&get_withdrawal_root_path()),
                &WITHDRAWAL_TOTAL_CREDITS_HISTORY_KEY,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .value
            .expect("total credits history tree should exist after the v14 transition");
        assert!(element.is_any_tree());

        let element = platform
            .drive
            .grove
            .get(
                SubtreePath::from(&get_withdrawal_root_path()),
                &WITHDRAWAL_CREDIT_INFLOWS_SUM_TREE_KEY,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .value
            .expect("credit inflows sum tree should exist after the v14 transition");
        assert!(element.is_sum_tree());

        // Running it again is harmless and the tree stays usable
        platform
            .transition_to_version_14(&block_info, &transaction, platform_version)
            .expect("expected the transition to be idempotent");
        assert_eq!(
            platform
                .drive
                .fetch_total_credits_in_platform_a_day_ago(
                    block_info.time_ms,
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected to read an empty history"),
            None
        );
    }

    #[test]
    fn test_transition_to_version_12_creates_shielded_pool_trees() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Version 12 depends on version 11 trees existing
        platform
            .transition_to_version_11(&transaction, platform_version)
            .expect("expected version 11 transition to succeed");

        platform
            .transition_to_version_12(&transaction, platform_version)
            .expect("expected version 12 transition to succeed");

        // Verify shielded credit pool tree was created under ShieldedBalances
        let shielded_pool_element = platform.drive.grove.get(
            SubtreePath::from(&[&[RootTree::ShieldedBalances as u8] as &[u8]]),
            &[MAIN_SHIELDED_CREDIT_POOL_KEY_U8],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            shielded_pool_element.value.is_ok(),
            "Shielded credit pool tree should exist after transition_to_version_12"
        );

        // Verify notes tree was created inside the shielded pool
        let shielded_pool_path = shielded_credit_pool_path();
        let notes_element = platform.drive.grove.get(
            SubtreePath::from(shielded_pool_path.as_ref()),
            &[SHIELDED_NOTES_KEY],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            notes_element.value.is_ok(),
            "Shielded notes tree should exist after transition_to_version_12"
        );

        // Verify nullifiers tree was created
        let nullifiers_element = platform.drive.grove.get(
            SubtreePath::from(shielded_pool_path.as_ref()),
            &[SHIELDED_NULLIFIERS_KEY],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            nullifiers_element.value.is_ok(),
            "Shielded nullifiers tree should exist after transition_to_version_12"
        );

        // Verify anchors tree was created
        let anchors_element = platform.drive.grove.get(
            SubtreePath::from(shielded_pool_path.as_ref()),
            &[SHIELDED_ANCHORS_IN_POOL_KEY],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            anchors_element.value.is_ok(),
            "Shielded anchors tree should exist after transition_to_version_12"
        );
    }

    // test_full_transition_from_version_3_to_latest and test_transition_from_version_5
    // removed: multi-version transitions require cumulative state from each prior version

    #[test]
    fn test_transition_from_version_10_triggers_11_and_12() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let platform_state = platform.state.load();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // From version 10, versions 11 and 12 should trigger
        platform
            .perform_events_on_first_block_of_protocol_change_v0(
                &platform_state,
                &block_info,
                &transaction,
                10,
                platform_version,
            )
            .expect("expected transition from version 10 to succeed");

        // Verify version 11 artifacts: AddressBalances root tree should exist
        use drive::drive::RootTree;
        use drive::grovedb_path::SubtreePath;
        let address_balances = platform.drive.grove.get(
            SubtreePath::empty(),
            &[RootTree::AddressBalances as u8],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            address_balances.value.is_ok(),
            "AddressBalances root tree should exist (from version 11 transition)"
        );

        // Verify version 12 artifacts: shielded credit pool tree should exist
        let shielded_pool = platform.drive.grove.get(
            SubtreePath::from(&[&[RootTree::ShieldedBalances as u8] as &[u8]]),
            &[MAIN_SHIELDED_CREDIT_POOL_KEY_U8],
            Some(&transaction),
            &platform_version.drive.grove_version,
        );
        assert!(
            shielded_pool.value.is_ok(),
            "Shielded credit pool tree should exist (from version 12 transition)"
        );
    }

    #[test]
    fn test_idempotent_transition_to_version_6() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1_000_000,
            height: 100,
            core_height: 100,
            epoch: Epoch::new(1).expect("expected epoch"),
        };

        // First call should succeed
        platform
            .transition_to_version_6(&block_info, &transaction, platform_version)
            .expect("first transition should succeed");

        // Second call should also succeed (idempotent due to insert_contract)
        let result = platform.transition_to_version_6(&block_info, &transaction, platform_version);
        assert!(result.is_ok());
    }

    #[test]
    fn test_v12_migration_strips_unknown_document_schema_properties() {
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::platform_value::Value as PlatformValue;
        use dpp::serialization::PlatformSerializableWithPlatformVersion;
        use dpp::tests::json_document::json_document_to_contract_with_ids;
        use platform_version::TryFromPlatformVersioned;

        let platform_version_11 = PlatformVersion::get(11).expect("expected v11");
        let platform_version_12 = PlatformVersion::latest();

        // 1. Set up platform at v11
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // 2. Create an identity to own the contract
        let identity = Identity::random_identity(3, Some(42), platform_version_11)
            .expect("expected a random identity");

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version_11,
            )
            .expect("expected to add identity");

        // 3. Create a contract and manually inject unknown properties into the schema
        let mut data_contract = json_document_to_contract_with_ids(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            None,
            None,
            false, // no validation — we want to inject unknown props
            platform_version_11,
        )
        .expect("expected to get contract");

        data_contract.set_owner_id(identity.id());

        // Get the raw serialized format and inject unknown properties
        let mut serialization_format =
            DataContractInSerializationFormat::try_from_platform_versioned(
                data_contract.clone(),
                platform_version_11,
            )
            .expect("expected to convert to serialization format");

        // Inject unknown properties into the "person" document schema. These
        // include both an arbitrary unknown key and the v12-introduced flags
        // (`documentsCountable` / `rangeCountable`) — the latter must also be
        // stripped from pre-v12 contracts so the v2 parser cannot revive them
        // and reinterpret a NormalTree contract as a count tree post-upgrade.
        for (_doc_type_name, schema_value) in serialization_format.document_schemas_mut().iter_mut()
        {
            if let Some(map) = schema_value.as_map_mut() {
                map.push((
                    PlatformValue::Text("unknownSmuggled".to_string()),
                    PlatformValue::Bool(true),
                ));
                map.push((
                    PlatformValue::Text("documentsCountable".to_string()),
                    PlatformValue::Bool(true),
                ));
                map.push((
                    PlatformValue::Text("rangeCountable".to_string()),
                    PlatformValue::Bool(true),
                ));
            }
        }

        // Serialize the modified contract and insert directly into Drive
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let contract_bytes = bincode::encode_to_vec(&serialization_format, bincode_config)
            .expect("expected to serialize");

        let contract_id = data_contract.id();
        let contract_path =
            drive::drive::contract::paths::contract_root_path(contract_id.as_bytes());

        // Insert the contract storage tree
        platform
            .drive
            .grove_insert_if_not_exists(
                SubtreePath::from(&[&[RootTree::DataContractDocuments as u8] as &[u8]][..]),
                contract_id.as_bytes(),
                Element::empty_tree(),
                Some(&transaction),
                None,
                &platform_version_11.drive,
            )
            .expect("insert contract tree");

        // Insert the contract data at key [0]
        platform
            .drive
            .grove_insert(
                (&contract_path).into(),
                &[0],
                Element::Item(contract_bytes, None),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("insert contract data");

        // 4. Verify the unknown property is there before migration
        let raw_before = platform
            .drive
            .grove_get_raw(
                (&contract_path).into(),
                &[0],
                drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("get raw")
            .expect("element should exist");

        let bytes_before = match &raw_before {
            Element::Item(bytes, _) => bytes.clone(),
            _ => panic!("expected Item"),
        };

        let format_before: DataContractInSerializationFormat = bincode::borrow_decode_from_slice(
            &bytes_before,
            bincode::config::standard()
                .with_big_endian()
                .with_no_limit(),
        )
        .expect("deserialize")
        .0;

        let has_unknown_before = format_before.document_schemas().values().any(|schema| {
            schema
                .as_map()
                .map(|map| {
                    map.iter()
                        .any(|(k, _)| k.as_text() == Some("unknownSmuggled"))
                })
                .unwrap_or(false)
        });
        assert!(
            has_unknown_before,
            "Contract should have unknownSmuggled property before migration"
        );

        // 5. Run the v12 migration
        // First need v11 trees
        platform
            .transition_to_version_11(&transaction, platform_version_12)
            .expect("v11 transition");

        platform
            .transition_to_version_12(&transaction, platform_version_12)
            .expect("v12 transition should succeed and strip unknown properties");

        // 6. Verify the unknown property is gone from disk
        let raw_after = platform
            .drive
            .grove_get_raw(
                (&contract_path).into(),
                &[0],
                drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version_12.drive,
            )
            .expect("get raw")
            .expect("element should exist");

        let bytes_after = match &raw_after {
            Element::Item(bytes, _) => bytes.clone(),
            _ => panic!("expected Item"),
        };

        let format_after: DataContractInSerializationFormat = bincode::borrow_decode_from_slice(
            &bytes_after,
            bincode::config::standard()
                .with_big_endian()
                .with_no_limit(),
        )
        .expect("deserialize")
        .0;

        let schema_has_key = |format: &DataContractInSerializationFormat, key: &str| -> bool {
            format.document_schemas().values().any(|schema| {
                schema
                    .as_map()
                    .map(|map| map.iter().any(|(k, _)| k.as_text() == Some(key)))
                    .unwrap_or(false)
            })
        };

        assert!(
            !schema_has_key(&format_after, "unknownSmuggled"),
            "Contract should NOT have unknownSmuggled property after v12 migration"
        );
        assert!(
            !schema_has_key(&format_after, "documentsCountable"),
            "Contract should NOT have smuggled documentsCountable after v12 migration"
        );
        assert!(
            !schema_has_key(&format_after, "rangeCountable"),
            "Contract should NOT have smuggled rangeCountable after v12 migration"
        );

        // 7. Verify known properties are still present
        let has_type = format_after.document_schemas().values().any(|schema| {
            schema
                .as_map()
                .map(|map| map.iter().any(|(k, _)| k.as_text() == Some("type")))
                .unwrap_or(false)
        });
        assert!(
            has_type,
            "Contract should still have 'type' property after migration"
        );

        // 8. Verify the cache was cleared by fetching the contract through the
        //    Drive API. The cache was populated implicitly during migration (or
        //    could have been from prior block processing). After the migration
        //    clears the global cache, a fresh fetch should reload from disk and
        //    return the cleaned contract.
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");

        let (_fee, fetched): (
            _,
            Option<std::sync::Arc<drive::drive::contract::DataContractFetchInfo>>,
        ) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                *contract_id.as_bytes(),
                None,
                false,
                None, // no transaction — reads committed state
                platform_version_12,
            )
            .expect("fetch from cache/disk");

        let fetch_info = fetched.expect("contract should exist");
        let fetched_contract = &fetch_info.contract;

        // Check the fetched contract's raw serialization doesn't have the unknown property
        // (This verifies both cache invalidation and disk update)
        let refetched_format = DataContractInSerializationFormat::try_from_platform_versioned(
            fetched_contract.clone(),
            platform_version_12,
        )
        .expect("convert to serialization format");

        let schema_has_key_refetched =
            |format: &DataContractInSerializationFormat, key: &str| -> bool {
                format.document_schemas().values().any(|schema| {
                    schema
                        .as_map()
                        .map(|map| map.iter().any(|(k, _)| k.as_text() == Some(key)))
                        .unwrap_or(false)
                })
            };
        assert!(
            !schema_has_key_refetched(&refetched_format, "unknownSmuggled"),
            "Contract fetched through Drive API after migration should not have unknownSmuggled"
        );
        assert!(
            !schema_has_key_refetched(&refetched_format, "documentsCountable"),
            "Contract fetched through Drive API after migration should not have smuggled documentsCountable"
        );
        assert!(
            !schema_has_key_refetched(&refetched_format, "rangeCountable"),
            "Contract fetched through Drive API after migration should not have smuggled rangeCountable"
        );
    }

    #[test]
    fn test_v12_migration_strips_unknown_properties_from_historical_contract() {
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::platform_value::Value as PlatformValue;
        use dpp::tests::json_document::json_document_to_contract_with_ids;
        use drive::grovedb::reference_path::ReferencePathType;
        use platform_version::TryFromPlatformVersioned;

        let platform_version_11 = PlatformVersion::get(11).expect("expected v11");
        let platform_version_12 = PlatformVersion::latest();

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Create identity
        let identity = Identity::random_identity(3, Some(99), platform_version_11)
            .expect("expected a random identity");
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version_11,
            )
            .expect("expected to add identity");

        // Create contract
        let mut data_contract = json_document_to_contract_with_ids(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            None,
            None,
            false,
            platform_version_11,
        )
        .expect("expected to get contract");
        data_contract.set_owner_id(identity.id());

        // Serialize and inject unknown property
        let mut serialization_format =
            DataContractInSerializationFormat::try_from_platform_versioned(
                data_contract.clone(),
                platform_version_11,
            )
            .expect("convert to serialization format");

        for (_name, schema_value) in serialization_format.document_schemas_mut().iter_mut() {
            if let Some(map) = schema_value.as_map_mut() {
                map.push((
                    PlatformValue::Text("historicalSmuggled".to_string()),
                    PlatformValue::Bool(true),
                ));
            }
        }

        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let contract_bytes =
            bincode::encode_to_vec(&serialization_format, bincode_config).expect("serialize");

        let contract_id = data_contract.id();

        // Build historical contract storage layout:
        // [DataContractDocuments, contract_id] = tree (contract root)
        // [DataContractDocuments, contract_id, 0] = tree (history root)
        // [DataContractDocuments, contract_id, 0, encoded_time] = Item(contract_bytes)
        // [DataContractDocuments, contract_id, 0, 0] = Reference(SiblingReference(encoded_time))

        let root_tree_key = &[RootTree::DataContractDocuments as u8];
        let encoded_time = drive::util::common::encode::encode_u64(1000000);

        // Insert contract root tree
        platform
            .drive
            .grove_insert_if_not_exists(
                SubtreePath::from(&[root_tree_key as &[u8]][..]),
                contract_id.as_bytes(),
                Element::empty_tree(),
                Some(&transaction),
                None,
                &platform_version_11.drive,
            )
            .expect("insert contract root");

        // Insert history root tree at key [0]
        let contract_root_path =
            drive::drive::contract::paths::contract_root_path(contract_id.as_bytes());
        platform
            .drive
            .grove_insert(
                (&contract_root_path).into(),
                &[0],
                Element::empty_tree(),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("insert history tree");

        // Insert contract data at the timestamp key
        let history_path = drive::drive::contract::paths::contract_keeping_history_root_path(
            contract_id.as_bytes(),
        );
        platform
            .drive
            .grove_insert(
                (&history_path).into(),
                encoded_time.as_slice(),
                Element::Item(contract_bytes, None),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("insert contract at timestamp");

        // Insert reference at key [0] pointing to the timestamp key
        platform
            .drive
            .grove_insert(
                (&history_path).into(),
                &[0],
                Element::Reference(
                    ReferencePathType::SiblingReference(encoded_time.clone()),
                    Some(1),
                    None,
                ),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("insert reference");

        // Verify unknown property exists before migration
        let raw_element = platform
            .drive
            .grove_get_raw(
                (&history_path).into(),
                encoded_time.as_slice(),
                drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("get raw")
            .expect("element");

        let bytes_before = match &raw_element {
            Element::Item(bytes, _) => bytes.clone(),
            _ => panic!("expected Item"),
        };
        let format_before: DataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(&bytes_before, bincode_config)
                .expect("deserialize")
                .0;

        let has_smuggled = format_before.document_schemas().values().any(|s| {
            s.as_map()
                .map(|m| {
                    m.iter()
                        .any(|(k, _)| k.as_text() == Some("historicalSmuggled"))
                })
                .unwrap_or(false)
        });
        assert!(
            has_smuggled,
            "historical contract should have smuggled property before migration"
        );

        // Run v12 migration
        platform
            .transition_to_version_11(&transaction, platform_version_12)
            .expect("v11 transition");
        platform
            .transition_to_version_12(&transaction, platform_version_12)
            .expect("v12 transition");

        // Verify property is stripped from disk (at the timestamp key)
        let raw_after = platform
            .drive
            .grove_get_raw(
                (&history_path).into(),
                encoded_time.as_slice(),
                drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version_12.drive,
            )
            .expect("get raw")
            .expect("element");

        let bytes_after = match &raw_after {
            Element::Item(bytes, _) => bytes.clone(),
            _ => panic!("expected Item"),
        };
        let format_after: DataContractInSerializationFormat =
            bincode::borrow_decode_from_slice(&bytes_after, bincode_config)
                .expect("deserialize")
                .0;

        let has_smuggled_after = format_after.document_schemas().values().any(|s| {
            s.as_map()
                .map(|m| {
                    m.iter()
                        .any(|(k, _)| k.as_text() == Some("historicalSmuggled"))
                })
                .unwrap_or(false)
        });
        assert!(
            !has_smuggled_after,
            "historical contract should NOT have smuggled property after v12 migration"
        );
    }

    #[test]
    fn test_v12_migration_strips_unknown_properties_from_all_historical_revisions() {
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::platform_value::Value as PlatformValue;
        use drive::grovedb::reference_path::ReferencePathType;
        use platform_version::TryFromPlatformVersioned;

        let platform_version_11 = PlatformVersion::get(11).expect("expected v11");
        let platform_version_12 = PlatformVersion::latest();

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();

        // Create identity
        let identity = Identity::random_identity(3, Some(101), platform_version_11)
            .expect("expected a random identity");
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version_11,
            )
            .expect("expected to add identity");

        // Create base contract
        let mut data_contract = dpp::tests::json_document::json_document_to_contract_with_ids(
            "../rs-drive/tests/supporting_files/contract/family/family-contract.json",
            None,
            None,
            false,
            platform_version_11,
        )
        .expect("expected to get contract");
        data_contract.set_owner_id(identity.id());

        let contract_id = data_contract.id();
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        // Build 3 revisions, each with a different unknown property
        let timestamps: Vec<u64> = vec![1_000_000, 2_000_000, 3_000_000];
        let smuggled_keys: Vec<&str> = vec!["smuggled_v1", "smuggled_v2", "smuggled_v3"];

        let mut revision_bytes = Vec::new();
        for (i, smuggled_key) in smuggled_keys.iter().enumerate() {
            let mut serialization_format =
                DataContractInSerializationFormat::try_from_platform_versioned(
                    data_contract.clone(),
                    platform_version_11,
                )
                .expect("convert to serialization format");

            for (_name, schema_value) in serialization_format.document_schemas_mut().iter_mut() {
                if let Some(map) = schema_value.as_map_mut() {
                    map.push((
                        PlatformValue::Text(smuggled_key.to_string()),
                        PlatformValue::U32(i as u32),
                    ));
                }
            }

            let bytes =
                bincode::encode_to_vec(&serialization_format, bincode_config).expect("serialize");
            revision_bytes.push(bytes);
        }

        // Set up the historical contract storage layout
        let root_tree_key = &[RootTree::DataContractDocuments as u8];

        // Contract root tree
        platform
            .drive
            .grove_insert_if_not_exists(
                SubtreePath::from(&[root_tree_key as &[u8]][..]),
                contract_id.as_bytes(),
                Element::empty_tree(),
                Some(&transaction),
                None,
                &platform_version_11.drive,
            )
            .expect("insert contract root");

        // History root tree at key [0]
        let contract_root_path =
            drive::drive::contract::paths::contract_root_path(contract_id.as_bytes());
        platform
            .drive
            .grove_insert(
                (&contract_root_path).into(),
                &[0],
                Element::empty_tree(),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("insert history tree");

        let history_path = drive::drive::contract::paths::contract_keeping_history_root_path(
            contract_id.as_bytes(),
        );

        // Insert all 3 revisions at their timestamp keys, each with distinct flags
        let revision_flags: Vec<Option<Vec<u8>>> =
            vec![Some(vec![1, 10]), Some(vec![2, 20]), Some(vec![3, 30])];
        let mut encoded_times = Vec::new();
        for (i, ts) in timestamps.iter().enumerate() {
            let encoded_time = drive::util::common::encode::encode_u64(*ts);
            platform
                .drive
                .grove_insert(
                    (&history_path).into(),
                    encoded_time.as_slice(),
                    Element::Item(revision_bytes[i].clone(), revision_flags[i].clone()),
                    Some(&transaction),
                    None,
                    &mut vec![],
                    &platform_version_11.drive,
                )
                .expect("insert revision");
            encoded_times.push(encoded_time);
        }

        // Reference at [0] pointing to the latest (3rd) revision
        platform
            .drive
            .grove_insert(
                (&history_path).into(),
                &[0],
                Element::Reference(
                    ReferencePathType::SiblingReference(encoded_times[2].clone()),
                    Some(1),
                    None,
                ),
                Some(&transaction),
                None,
                &mut vec![],
                &platform_version_11.drive,
            )
            .expect("insert reference");

        // Verify all 3 revisions have their smuggled property before migration
        for (i, encoded_time) in encoded_times.iter().enumerate() {
            let raw = platform
                .drive
                .grove_get_raw(
                    (&history_path).into(),
                    encoded_time.as_slice(),
                    drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                    Some(&transaction),
                    &mut vec![],
                    &platform_version_11.drive,
                )
                .expect("get raw")
                .expect("element");

            let bytes = match &raw {
                Element::Item(bytes, _) => bytes.clone(),
                _ => panic!("expected Item"),
            };
            let format: DataContractInSerializationFormat =
                bincode::borrow_decode_from_slice(&bytes, bincode_config)
                    .expect("deserialize")
                    .0;

            let has_smuggled = format.document_schemas().values().any(|s| {
                s.as_map()
                    .map(|m| m.iter().any(|(k, _)| k.as_text() == Some(smuggled_keys[i])))
                    .unwrap_or(false)
            });
            assert!(
                has_smuggled,
                "revision {} should have '{}' before migration",
                i, smuggled_keys[i]
            );
        }

        // Run v12 migration
        platform
            .transition_to_version_11(&transaction, platform_version_12)
            .expect("v11 transition");
        platform
            .transition_to_version_12(&transaction, platform_version_12)
            .expect("v12 transition");

        // Verify ALL 3 revisions are cleaned
        for (i, encoded_time) in encoded_times.iter().enumerate() {
            let raw = platform
                .drive
                .grove_get_raw(
                    (&history_path).into(),
                    encoded_time.as_slice(),
                    drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                    Some(&transaction),
                    &mut vec![],
                    &platform_version_12.drive,
                )
                .expect("get raw")
                .expect("element");

            let (bytes, flags) = match &raw {
                Element::Item(bytes, flags) => (bytes.clone(), flags.clone()),
                _ => panic!("expected Item"),
            };
            let format: DataContractInSerializationFormat =
                bincode::borrow_decode_from_slice(&bytes, bincode_config)
                    .expect("deserialize")
                    .0;

            let has_any_smuggled = format.document_schemas().values().any(|s| {
                s.as_map()
                    .map(|m| {
                        m.iter().any(|(k, _)| {
                            k.as_text()
                                .map(|t| t.starts_with("smuggled_"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            });
            assert!(
                !has_any_smuggled,
                "revision {} should NOT have any smuggled properties after migration",
                i
            );

            // Verify element flags are preserved
            assert_eq!(
                flags, revision_flags[i],
                "revision {} should preserve its original element flags after migration",
                i
            );
        }

        // Verify the reference at [0] still exists and is intact
        let ref_element = platform
            .drive
            .grove_get_raw(
                (&history_path).into(),
                &[0],
                drive::util::grove_operations::DirectQueryType::StatefulDirectQuery,
                Some(&transaction),
                &mut vec![],
                &platform_version_12.drive,
            )
            .expect("get raw")
            .expect("reference should still exist");

        assert!(
            matches!(ref_element, Element::Reference(..)),
            "reference element should still be a Reference after migration"
        );
    }

    /// CONSENSUS-CRITICAL equivalence guard for the v11→v12 boundary.
    ///
    /// The `[ShieldedBalances]` subtree is built two ways that MUST be
    /// byte-identical:
    ///
    ///  * GENESIS path — a node that state-syncs a fresh v12 chain runs the real
    ///    `Drive::create_initial_state_structure_v3` (protocol-v12 genesis).
    ///  * UPGRADE path — a node already on v11 runs the real
    ///    `Platform::transition_to_version_12` at the activation block.
    ///
    /// Before the fix these diverged: genesis built the pool via a sorted
    /// `GroveDbOpBatch`, which roots the parent Merk at the batch's median key
    /// `[160]`; the upgrade built it with sequential breadth-first inserts, which
    /// root it at `[128]` (the intended NOTES-at-root layout). Two different
    /// subtree shapes ⇒ a state-synced v12 node and an in-place-upgraded v12 node
    /// would compute different app hashes at the boundary block and fork.
    ///
    /// This test drives the REAL production functions (not a hand-rebuilt batch)
    /// — Platform A is a genuine genesis-v12, Platform B is a genuine genesis-v11
    /// then the real `transition_to_version_12`. It asserts that the ENTIRE
    /// `[ShieldedBalances]` subtree (the main pool element with its carried
    /// `root_key`, all eight children, and every node below them) is byte-
    /// identical via `collect_subtree_diffs`.
    ///
    /// We compare the named subtree rather than the whole-DB root hash on
    /// purpose: the whole DB also carries the genesis epoch's recorded protocol-
    /// version field (`[Pools, epoch_0, "v"]` = 12 for Platform A, 11 for
    /// Platform B), which legitimately differs between a chain *born* at v12 and
    /// a chain born at v11 then upgraded. That field has nothing to do with how
    /// the shielded pool is constructed and would pollute a whole-DB comparison.
    /// The diagnostic that localized the original bug confirmed the shielded
    /// subtree was the ONLY construction-driven divergence.
    ///
    /// RED before the fix (genesis pool root `[160]` ≠ upgrade pool root `[128]`,
    /// plus a cascade of differing child hashes), GREEN after (both `[128]`,
    /// because both paths now call the shared
    /// `Drive::insert_shielded_pool_structure`).
    #[test]
    fn test_genesis_v12_and_upgrade_to_v12_build_identical_shielded_pool() {
        let platform_version_12 = PlatformVersion::get(12).expect("expected v12");

        // ---- Platform A: REAL fresh genesis at protocol v12. -----------------
        // Runs the production create_initial_state_structure_v3 (the genesis /
        // batch path). Genesis is already committed (no open transaction).
        let platform_a = TestPlatformBuilder::new()
            .with_initial_protocol_version(12)
            .build_with_mock_rpc()
            .set_genesis_state();

        // ---- Platform B: REAL v11 genesis, then REAL transition_to_version_12.
        let platform_b = TestPlatformBuilder::new()
            .with_initial_protocol_version(11)
            .build_with_mock_rpc()
            .set_genesis_state();

        // Sanity: a genuine v11 genesis must NOT contain ShieldedBalances yet.
        {
            let txn = platform_b.drive.grove.start_transaction();
            let shielded_root_pre = platform_b.drive.grove.get(
                SubtreePath::empty(),
                &[RootTree::ShieldedBalances as u8],
                Some(&txn),
                &platform_version_12.drive.grove_version,
            );
            assert!(
                shielded_root_pre.value.is_err(),
                "v11 genesis must not contain ShieldedBalances before the upgrade; got {:?}",
                shielded_root_pre.value
            );
        }

        let txn_b = platform_b.drive.grove.start_transaction();
        platform_b
            .transition_to_version_12(&txn_b, platform_version_12)
            .expect("upgrade: transition_to_version_12 should succeed");

        // Localizing diagnostic: the main pool element's carried root_key is the
        // [128] (fixed) vs [160] (buggy) AVL-shape discriminator.
        let sb_path: [&[u8]; 1] = [&[RootTree::ShieldedBalances as u8]];
        let gv = &platform_version_12.drive.grove_version;
        let m_a = platform_a
            .drive
            .grove
            .get(
                SubtreePath::from(&sb_path[..]),
                &[MAIN_SHIELDED_CREDIT_POOL_KEY_U8],
                None,
                gv,
            )
            .unwrap()
            .expect("genesis: [ShieldedBalances, M] element");
        let m_b = platform_b
            .drive
            .grove
            .get(
                SubtreePath::from(&sb_path[..]),
                &[MAIN_SHIELDED_CREDIT_POOL_KEY_U8],
                Some(&txn_b),
                gv,
            )
            .unwrap()
            .expect("upgrade: [ShieldedBalances, M] element");
        println!("[ShieldedBalances, M] GENESIS(v12): {:?}", m_a);
        println!("[ShieldedBalances, M] UPGRADE(v12): {:?}", m_b);

        // Rigorous assertion: the ENTIRE [ShieldedBalances] subtree must be
        // byte-identical between the two paths, at every depth.
        let diffs = collect_subtree_diffs(
            &platform_a,
            &platform_b,
            &txn_b,
            vec![vec![RootTree::ShieldedBalances as u8]],
        );
        assert!(
            diffs.is_empty(),
            "CONSENSUS FORK: the [ShieldedBalances] subtree differs between a fresh genesis-v12 \
             node and an in-place-upgraded v12 node. A state-synced v12 node and an upgraded v12 \
             node would disagree on the app hash at the v11→v12 boundary block.\n{}",
            diffs.join("\n"),
        );
    }

    /// READ-ONLY v11 boundary equivalence check — DO NOT use this to justify
    /// changing v11 construction.
    ///
    /// The same batch-vs-sequential pattern exists at the v10→v11 boundary:
    ///
    ///  * GENESIS path — `Drive::create_initial_state_structure_v2` builds the
    ///    AddressBalances + SavedBlockTransactions subtrees via a `GroveDbOpBatch`.
    ///  * UPGRADE path — `Platform::transition_to_version_11` builds the same
    ///    subtrees with sequential `grove_insert_if_not_exists` inserts.
    ///
    /// Protocol v11 is ALREADY ACTIVATED on mainnet and testnet, so changing how
    /// v11 builds these trees would itself fork the live network. This test only
    /// CONFIRMS that the two v11 paths coincidentally produce the same state
    /// (expected GREEN as-is) — it is a tripwire. If it ever comes out RED, STOP:
    /// the live v11 network is already committed to whatever the genesis/batch
    /// path produced, and the discrepancy must be analyzed, not "fixed" by
    /// editing v11 code.
    #[test]
    fn test_genesis_v11_and_upgrade_to_v11_build_identical_address_trees() {
        let platform_version_11 = PlatformVersion::get(11).expect("expected v11");

        // ---- Platform A: REAL fresh genesis at protocol v11 (batch path). ----
        let platform_a = TestPlatformBuilder::new()
            .with_initial_protocol_version(11)
            .build_with_mock_rpc()
            .set_genesis_state();

        // ---- Platform B: REAL v10 genesis, then REAL transition_to_version_11.
        let platform_b = TestPlatformBuilder::new()
            .with_initial_protocol_version(10)
            .build_with_mock_rpc()
            .set_genesis_state();

        // Sanity: a genuine v10 genesis must NOT contain AddressBalances /
        // SavedBlockTransactions yet (structure v1 predates them).
        {
            let txn = platform_b.drive.grove.start_transaction();
            let addr_pre = platform_b.drive.grove.get(
                SubtreePath::empty(),
                &[RootTree::AddressBalances as u8],
                Some(&txn),
                &platform_version_11.drive.grove_version,
            );
            assert!(
                addr_pre.value.is_err(),
                "v10 genesis must not contain AddressBalances before the v11 upgrade; got {:?}",
                addr_pre.value
            );
            let sbt_pre = platform_b.drive.grove.get(
                SubtreePath::empty(),
                &[RootTree::SavedBlockTransactions as u8],
                Some(&txn),
                &platform_version_11.drive.grove_version,
            );
            assert!(
                sbt_pre.value.is_err(),
                "v10 genesis must not contain SavedBlockTransactions before the v11 upgrade; got {:?}",
                sbt_pre.value
            );
        }

        let txn_b = platform_b.drive.grove.start_transaction();
        platform_b
            .transition_to_version_11(&txn_b, platform_version_11)
            .expect("upgrade: transition_to_version_11 should succeed");

        // Rigorous assertion: the ENTIRE SavedBlockTransactions and
        // AddressBalances subtrees (the trees the v11 transition adds) must be
        // byte-identical between the batch (genesis) and sequential (upgrade)
        // paths, at every depth. SavedBlockTransactions has three children, so
        // it is exactly where a batch-vs-sequential AVL-shape divergence could
        // surface — the same failure mode that affected v12.
        //
        // As with the v12 test we compare the named subtrees, NOT the whole-DB
        // root hash: the whole DB also carries the genesis epoch's recorded
        // protocol-version field (`[Pools, epoch_0, "v"]` = 11 for the
        // genesis-v11 node, 10 for the v10→v11 upgraded node), a legitimate
        // genesis-baseline difference unrelated to address-tree construction.
        let mut diffs = collect_subtree_diffs(
            &platform_a,
            &platform_b,
            &txn_b,
            vec![vec![RootTree::SavedBlockTransactions as u8]],
        );
        diffs.extend(collect_subtree_diffs(
            &platform_a,
            &platform_b,
            &txn_b,
            vec![vec![RootTree::AddressBalances as u8]],
        ));

        assert!(
            diffs.is_empty(),
            "v11 TRIPWIRE (RED is a STOP signal): the SavedBlockTransactions / AddressBalances \
             subtrees differ between a fresh genesis-v11 node (batch path) and a v10→v11 upgraded \
             node (sequential path). v11 is ALREADY ACTIVATED on the live network — do NOT change \
             v11 construction to make this pass; surface and analyze the discrepancy.\n{}",
            diffs.join("\n"),
        );
    }

    #[test]
    fn test_transition_to_version_15_writes_initial_reduced_platform_state() {
        use dpp::reduced_platform_state::ReducedPlatformState;

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_version = PlatformVersion::latest();

        let transaction = platform.drive.grove.start_transaction();

        let platform_state = platform.state.load();

        // Before the transition, the replicated state must not carry the reduced state key.
        let pre_transition = platform
            .fetch_reduced_platform_state(Some(&transaction), platform_version)
            .expect("fetching an absent reduced platform state should not error");
        assert!(
            pre_transition.is_none(),
            "reduced platform state must not exist before transition_to_version_15"
        );

        let result =
            platform.transition_to_version_15(&platform_state, &transaction, platform_version);
        assert!(result.is_ok(), "transition failed: {:?}", result.err());

        let reduced = platform
            .fetch_reduced_platform_state(Some(&transaction), platform_version)
            .expect("expected to fetch reduced platform state")
            .expect("reduced platform state must exist after transition_to_version_15");

        let ReducedPlatformState::V0(reduced) = reduced;
        assert_eq!(
            reduced.current_protocol_version_in_consensus,
            platform_state.current_protocol_version_in_consensus()
        );
        assert_eq!(
            reduced.next_epoch_protocol_version,
            platform_state.next_epoch_protocol_version()
        );
        assert_eq!(
            reduced.quorum_positions.len(),
            platform_state.validator_sets().len(),
            "quorum positions must mirror the validator set order"
        );
        assert_eq!(
            reduced.proposed_core_chain_locked_height,
            platform_state.last_committed_core_height()
        );
    }
}
