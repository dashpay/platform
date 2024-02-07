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
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};

use dpp::version::PlatformVersion;
use drive::drive::document::query::{QueryDocumentsOutcomeV0Methods};
use std::collections::BTreeMap;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::SystemDataContract;
use crate::execution::platform_events::fee_pool_outwards_distribution::fetch_reward_shares_list_for_masternode::MN_REWARD_SHARES_DOCUMENT_TYPE;

impl<C> Platform<C> {
    /// A function to retrieve a list of the masternode reward shares documents for a list of masternode IDs.
    pub(super) fn fetch_reward_shares_list_for_masternode_v0(
        &self,
        masternode_owner_id: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let cache = self.drive.cache.read().unwrap();

        let masternode_rewards_contract = &cache.system_data_contracts.masternode_reward_shares;

        let document_type =
            masternode_rewards_contract.document_type_for_name(MN_REWARD_SHARES_DOCUMENT_TYPE)?;

        let drive_query = DriveQuery {
            contract: masternode_rewards_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
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
