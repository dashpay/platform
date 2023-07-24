// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

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
use drive::drive::query::QueryDocumentsOutcome;
use drive::grovedb::TransactionArg;
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};

use crate::platform_types::contracts::reward_shares::fetch_reward_shares_list_for_masternode::MN_REWARD_SHARES_DOCUMENT_TYPE;

use std::collections::BTreeMap;
use dpp::data_contract::base::DataContractBaseMethodsV0;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcome;

impl<C> Platform<C> {
    /// A function to retrieve a list of the masternode reward shares documents for a list of masternode IDs.
    pub(crate) fn fetch_reward_shares_list_for_masternode_v0(
        &self,
        masternode_owner_id: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let document_type = self
            .drive
            .system_contracts
            .masternode_rewards
            .document_type_for_name(MN_REWARD_SHARES_DOCUMENT_TYPE)?;

        let drive_query = DriveQuery {
            contract: &self.drive.system_contracts.masternode_rewards,
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

        let QueryDocumentsOutcome { documents, .. } =
            self.drive
                .query_documents(drive_query, None, false, transaction, Some(platform_version.protocol_version))?;

        Ok(documents)
    }
}
