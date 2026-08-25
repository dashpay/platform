//! Masternode reward shares.
//!
//! This module defines structs and functions related to masternode reward shares.
//!
//! Masternode reward shares are shares of the masternode reward that a masternode owner
//! would like to automatically distribute to addresses other than that of the masternode itself.
//!
//! For example, the address of someone who manages the masternode for the owner.
//!

use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::platform_value::Value;

use drive::dpp::document::Document;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use std::collections::BTreeMap;

use crate::execution::platform_events::fee_pool_outwards_distribution::fetch_reward_shares_list_for_masternode::MN_REWARD_SHARES_DOCUMENT_TYPE;

impl<C> Platform<C> {
    /// A function to retrieve a list of the masternode reward shares documents for a list of masternode IDs.
    pub(super) fn fetch_reward_shares_list_for_masternode_v0(
        &self,
        masternode_owner_id: &[u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let masternode_rewards_contract = self
            .drive
            .cache
            .system_data_contracts
            .load_masternode_reward_shares(platform_version)?;

        let document_type =
            masternode_rewards_contract.document_type_for_name(MN_REWARD_SHARES_DOCUMENT_TYPE)?;

        let drive_query = DriveDocumentQuery {
            contract: &masternode_rewards_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: BTreeMap::from([(
                    "$ownerId".to_string(),
                    WhereClause {
                        field: "$ownerId".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Bytes(masternode_owner_id.to_vec()),
                    },
                )]),
            },
            offset: None,
            limit: Some(1),
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        // todo: deal with cost of this operation
        let query_documents_outcome = self.drive.query_documents(
            drive_query,
            None,
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        Ok(query_documents_outcome.documents().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    /// On a genesis-state platform, no reward share documents exist for any
    /// owner, so the v0 query must return an empty `Vec`. This is the canonical
    /// "masternode has no explicit reward shares" case.
    #[test]
    fn v0_returns_empty_for_unknown_masternode_owner() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        let docs = platform
            .fetch_reward_shares_list_for_masternode_v0(
                &[0x11u8; 32],
                Some(&transaction),
                platform_version,
            )
            .expect("must succeed on a genesis state without reward shares");

        assert!(docs.is_empty(), "unknown owner must yield empty list");
    }

    /// Across distinct owner ids on a fresh (empty) state, the query must
    /// consistently return an empty `Vec`. This does not prove that owner
    /// bytes flow into the query key — genesis has no reward-share documents,
    /// so every owner is indistinguishable at the result level — but it does
    /// exercise the empty-list branch for several different inputs and
    /// guarantees the function is callable per-owner without internal state
    /// leaking between calls.
    #[test]
    fn v0_empty_list_branch_holds_across_distinct_owners_on_fresh_state() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let transaction = platform.drive.grove.start_transaction();
        for owner in [[0u8; 32], [1u8; 32], [0xffu8; 32]] {
            let docs = platform
                .fetch_reward_shares_list_for_masternode_v0(
                    &owner,
                    Some(&transaction),
                    platform_version,
                )
                .expect("must succeed for any owner");
            assert!(docs.is_empty(), "empty state must yield empty list");
        }
    }

    /// Without a transaction handle, the query must still succeed: the query
    /// itself is read-only and does not require a transaction.
    #[test]
    fn v0_works_without_transaction() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let docs = platform
            .fetch_reward_shares_list_for_masternode_v0(&[7u8; 32], None, platform_version)
            .expect("must succeed without a transaction");
        assert!(docs.is_empty());
    }
}
