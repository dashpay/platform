use crate::drive::contract::DataContractFetchInfo;
use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use grovedb_costs::OperationCost;
use std::sync::Arc;

impl Drive {
    /// The token market contract materialized for `platform_version`, wrapped for the document
    /// operations of the order transitions. A system contract is stored without storage flags,
    /// and the order transitions never bill reading it: it is held by the system contract cache.
    pub fn token_market_contract_fetch_info(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Arc<DataContractFetchInfo>, Error> {
        let contract = self
            .cache
            .system_data_contracts
            .load_token_market(platform_version)?;
        Ok(Arc::new(DataContractFetchInfo {
            contract: (*contract).clone(),
            storage_flags: None,
            cost: OperationCost::default(),
            fee: None,
        }))
    }

    /// The open order `order_id` of `side` in the token market contract, with the processing
    /// cost of the read, billed at `epoch`. `None` when no open order of that side has the id:
    /// it never existed, was filled entirely or was cancelled.
    pub fn fetch_token_order_document(
        &self,
        token_market_contract: &DataContract,
        side: TokenOrderSide,
        order_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Document>, u64), Error> {
        let document_type =
            token_market_contract.document_type_for_name(side.document_type_name())?;
        let query = DriveDocumentQuery {
            contract: token_market_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: Some(WhereClause {
                    field: "$id".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(order_id.to_buffer()),
                }),
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: Default::default(),
            },
            offset: None,
            limit: Some(1),
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
            sub_queries: vec![],
        };
        let outcome = self.query_documents(
            query,
            Some(epoch),
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;
        let cost = outcome.cost();
        Ok((outcome.documents_owned().into_iter().next(), cost))
    }
}
