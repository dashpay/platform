use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::ordering::OrderClause;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::tokens::token_order::TokenOrder;
use dpp::tokens::token_order_side::TokenOrderSide;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use indexmap::IndexMap;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_frozen_token_listings_v0(
        &self,
        token_id: Identifier,
        owner_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<TokenOrder>, u64), Error> {
        let token_market_contract = self
            .cache
            .system_data_contracts
            .load_token_market(platform_version)?;
        let document_type = token_market_contract
            .document_type_for_name(TokenOrderSide::Ask.document_type_name())?;
        let equal = |field: &str, value: Identifier| {
            (
                field.to_string(),
                WhereClause {
                    field: field.to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(value.to_buffer()),
                },
            )
        };
        let query = DriveDocumentQuery {
            contract: &token_market_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: BTreeMap::from([
                    equal("$ownerId", owner_id),
                    equal("tokenId", token_id),
                ]),
            },
            offset: None,
            limit: Some(
                platform_version
                    .system_limits
                    .max_frozen_listings_destroyed_per_transition
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "the frozen listings are read only where their limit is set",
                    )))?,
            ),
            order_by: IndexMap::from([(
                "$createdAt".to_string(),
                OrderClause {
                    field: "$createdAt".to_string(),
                    ascending: true,
                },
            )]),
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
        let listings = outcome
            .documents_owned()
            .iter()
            .map(|document| TokenOrder::from_document(document, TokenOrderSide::Ask))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((listings, cost))
    }
}
