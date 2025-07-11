use std::sync::Arc;

#[cfg(any(feature = "server", feature = "verify"))]
pub use {
    conditions::{WhereClause, WhereOperator},
    grovedb::{PathQuery, Query, QueryItem, SizedQuery},
    ordering::OrderClause,
    single_document_drive_query::SingleDocumentDriveQuery,
    single_document_drive_query::SingleDocumentDriveQueryContestedStatus,
    vote_polls_by_end_date_query::VotePollsByEndDateDriveQuery,
    vote_query::IdentityBasedVoteDriveQuery,
};
// Imports available when either "server" or "verify" features are enabled
#[cfg(any(feature = "server", feature = "verify"))]
use {
    crate::{
        drive::contract::paths::DataContractPaths,
        error::{drive::DriveError, query::QuerySyntaxError, Error},
    },
    dpp::{
        data_contract::{
            accessors::v0::DataContractV0Getters,
            document_type::{accessors::DocumentTypeV0Getters, methods::DocumentTypeV0Methods},
            document_type::{DocumentTypeRef, Index, IndexProperty},
            DataContract,
        },
        document::{
            document_methods::DocumentMethodsV0,
            serialization_traits::DocumentPlatformConversionMethodsV0, Document, DocumentV0Getters,
        },
        platform_value::{btreemap_extensions::BTreeValueRemoveFromMapHelper, Value},
        version::PlatformVersion,
        ProtocolError,
    },
    indexmap::IndexMap,
    sqlparser::{
        ast::{self, OrderByExpr, Select, Statement, TableFactor::Table, Value::Number},
        dialect::MySqlDialect,
        parser::Parser,
    },
    std::{collections::BTreeMap, ops::BitXor},
};

#[cfg(feature = "verify")]
use crate::verify::RootHash;

#[cfg(feature = "server")]
pub use grovedb::{
    query_result_type::{QueryResultElements, QueryResultType},
    Element, Error as GroveError, TransactionArg,
};

use dpp::document;
use dpp::prelude::Identifier;
#[cfg(feature = "server")]
use {
    crate::{drive::Drive, fees::op::LowLevelDriveOperation},
    dpp::block::block_info::BlockInfo,
};
// Crate-local unconditional imports
use crate::config::DriveConfig;
// Crate-local unconditional imports
use crate::util::common::encode::encode_u64;
#[cfg(feature = "server")]
use crate::util::grove_operations::QueryType::StatefulQuery;

// Module declarations that are conditional on either "server" or "verify" features
#[cfg(any(feature = "server", feature = "verify"))]
pub mod conditions;
#[cfg(any(feature = "server", feature = "verify"))]
mod defaults;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod ordering;
#[cfg(any(feature = "server", feature = "verify"))]
mod single_document_drive_query;

// Module declarations exclusively for "server" feature
#[cfg(feature = "server")]
mod test_index;

#[cfg(any(feature = "server", feature = "verify"))]
/// Vote poll vote state query module
pub mod vote_poll_vote_state_query;
#[cfg(any(feature = "server", feature = "verify"))]
/// Vote Query module
pub mod vote_query;

#[cfg(any(feature = "server", feature = "verify"))]
/// Vote poll contestant votes query module
pub mod vote_poll_contestant_votes_query;

#[cfg(any(feature = "server", feature = "verify"))]
/// Vote polls by end date query
pub mod vote_polls_by_end_date_query;

#[cfg(any(feature = "server", feature = "verify"))]
/// Vote polls by document type query
pub mod vote_polls_by_document_type_query;

/// Function type for looking up a contract by identifier
///
/// This function is used to look up a contract by its identifier.
/// It should be implemented by the caller in order to provide data
/// contract required for operations like proof verification.
#[cfg(any(feature = "server", feature = "verify"))]
pub type ContractLookupFn<'a> = dyn Fn(&dpp::identifier::Identifier) -> Result<Option<Arc<DataContract>>, crate::error::Error>
    + 'a;

/// Creates a [ContractLookupFn] function that returns provided data contract when requested.
///
/// # Arguments
///
/// * `data_contract` - [Arc<DataContract>](DataContract) to return
///
/// # Returns
///
/// [ContractLookupFn] that will return the `data_contract`, or `None` if
/// the requested contract is not the same as the provided one.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn contract_lookup_fn_for_contract<'a>(
    data_contract: Arc<DataContract>,
) -> Box<ContractLookupFn<'a>> {
    let func = move
        |id: &dpp::identifier::Identifier| -> Result<Option<Arc<DataContract>>, crate::error::Error> {
            if data_contract.id().ne(id) {
                return Ok(None);
            }
            Ok(Some(Arc::clone(&data_contract)))
        };
    Box::new(func)
}

/// A query to get the votes given out by an identity
#[cfg(any(feature = "server", feature = "verify"))]
pub mod contested_resource_votes_given_by_identity_query;
/// A query to get contested documents before they have been awarded
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive_contested_document_query;

/// A query to get the block counts of proposers in an epoch
#[cfg(any(feature = "server", feature = "verify"))]
pub mod proposer_block_count_query;

/// A query to get the identity's token balance
#[cfg(any(feature = "server", feature = "verify"))]
pub mod identity_token_balance_drive_query;
/// A query to get the identity's token info
#[cfg(any(feature = "server", feature = "verify"))]
pub mod identity_token_info_drive_query;

/// A query to get the token's status
#[cfg(any(feature = "server", feature = "verify"))]
pub mod token_status_drive_query;

#[cfg(any(feature = "server", feature = "verify"))]
/// Represents a starting point for a query based on a specific document.
///
/// This struct encapsulates all the necessary details to define the starting
/// conditions for a query, including the document to start from, its type,
/// associated index property, and whether the document itself should be included
/// in the query results.
#[derive(Debug, Clone)]
pub struct StartAtDocument<'a> {
    /// The document that serves as the starting point for the query.
    pub document: Document,

    /// The type of the document, providing metadata about its schema and structure.
    pub document_type: DocumentTypeRef<'a>,

    /// Indicates whether the starting document itself should be included in the query results.
    /// - `true`: The document is included in the results.
    /// - `false`: The document is excluded, and the query starts from the next matching document.
    pub included: bool,
}
#[cfg(any(feature = "server", feature = "verify"))]
/// Internal clauses struct
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InternalClauses {
    /// Primary key in clause
    pub primary_key_in_clause: Option<WhereClause>,
    /// Primary key equal clause
    pub primary_key_equal_clause: Option<WhereClause>,
    /// In clause
    pub in_clause: Option<WhereClause>,
    /// Range clause
    pub range_clause: Option<WhereClause>,
    /// Equal clause
    pub equal_clauses: BTreeMap<String, WhereClause>,
}

impl InternalClauses {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns true if the clause is a valid format.
    pub fn verify(&self) -> bool {
        // There can only be 1 primary key clause, or many other clauses
        if self
            .primary_key_in_clause
            .is_some()
            .bitxor(self.primary_key_equal_clause.is_some())
        {
            // One is set, all rest must be empty
            !(self.in_clause.is_some()
                || self.range_clause.is_some()
                || !self.equal_clauses.is_empty())
        } else {
            !(self.primary_key_in_clause.is_some() && self.primary_key_equal_clause.is_some())
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns true if the query clause is for primary keys.
    pub fn is_for_primary_key(&self) -> bool {
        self.primary_key_in_clause.is_some() || self.primary_key_equal_clause.is_some()
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns true if self is empty.
    pub fn is_empty(&self) -> bool {
        self.in_clause.is_none()
            && self.range_clause.is_none()
            && self.equal_clauses.is_empty()
            && self.primary_key_in_clause.is_none()
            && self.primary_key_equal_clause.is_none()
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Extracts the `WhereClause`s and returns them as type `InternalClauses`.
    pub fn extract_from_clauses(all_where_clauses: Vec<WhereClause>) -> Result<Self, Error> {
        let primary_key_equal_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                WhereOperator::Equal => match where_clause.is_identifier() {
                    true => Some(where_clause.clone()),
                    false => None,
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let primary_key_in_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                WhereOperator::In => match where_clause.is_identifier() {
                    true => Some(where_clause.clone()),
                    false => None,
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let (equal_clauses, range_clause, in_clause) =
            WhereClause::group_clauses(&all_where_clauses)?;

        let primary_key_equal_clause = match primary_key_equal_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_equal_clauses_array
                    .first()
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::Query(
                QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                    "There should only be one equal clause for the primary key",
                ),
            )),
        }?;

        let primary_key_in_clause = match primary_key_in_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_in_clauses_array
                    .first()
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::Query(
                QuerySyntaxError::DuplicateNonGroupableClauseSameField(
                    "There should only be one in clause for the primary key",
                ),
            )),
        }?;

        let internal_clauses = InternalClauses {
            primary_key_equal_clause,
            primary_key_in_clause,
            in_clause,
            range_clause,
            equal_clauses,
        };

        match internal_clauses.verify() {
            true => Ok(internal_clauses),
            false => Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents("Query has invalid where clauses"),
            )),
        }
    }
}

impl From<InternalClauses> for Vec<WhereClause> {
    fn from(clauses: InternalClauses) -> Self {
        let mut result: Self = clauses.equal_clauses.into_values().collect();

        if let Some(clause) = clauses.in_clause {
            result.push(clause);
        };
        if let Some(clause) = clauses.primary_key_equal_clause {
            result.push(clause);
        };
        if let Some(clause) = clauses.primary_key_in_clause {
            result.push(clause);
        };
        if let Some(clause) = clauses.range_clause {
            result.push(clause);
        };

        result
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct DriveDocumentQuery<'a> {
    ///DataContract
    pub contract: &'a DataContract,
    /// Document type
    pub document_type: DocumentTypeRef<'a>,
    /// Internal clauses
    pub internal_clauses: InternalClauses,
    /// Offset
    pub offset: Option<u16>,
    /// Limit
    pub limit: Option<u16>,
    /// Order by
    pub order_by: IndexMap<String, OrderClause>,
    /// Start at document id
    pub start_at: Option<[u8; 32]>,
    /// Start at included
    pub start_at_included: bool,
    /// Block time
    pub block_time_ms: Option<u64>,
}

impl<'a> DriveDocumentQuery<'a> {
    /// Gets a document by their primary key
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn new_primary_key_single_item_query(
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        id: Identifier,
    ) -> Self {
        DriveDocumentQuery {
            contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: Some(WhereClause {
                    field: document::property_names::ID.to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(id.to_buffer()),
                }),
                in_clause: None,
                range_clause: None,
                equal_clauses: Default::default(),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        }
    }

    #[cfg(feature = "server")]
    /// Returns any item
    pub fn any_item_query(contract: &'a DataContract, document_type: DocumentTypeRef<'a>) -> Self {
        DriveDocumentQuery {
            contract,
            document_type,
            internal_clauses: Default::default(),
            offset: None,
            limit: Some(1),
            order_by: Default::default(),
            start_at: None,
            start_at_included: true,
            block_time_ms: None,
        }
    }

    #[cfg(feature = "server")]
    /// Returns all items
    pub fn all_items_query(
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        limit: Option<u16>,
    ) -> Self {
        DriveDocumentQuery {
            contract,
            document_type,
            internal_clauses: Default::default(),
            offset: None,
            limit,
            order_by: Default::default(),
            start_at: None,
            start_at_included: true,
            block_time_ms: None,
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns true if the query clause if for primary keys.
    pub fn is_for_primary_key(&self) -> bool {
        self.internal_clauses.is_for_primary_key()
            || (self.internal_clauses.is_empty()
                && (self.order_by.is_empty()
                    || (self.order_by.len() == 1
                        && self
                            .order_by
                            .keys()
                            .collect::<Vec<&String>>()
                            .first()
                            .unwrap()
                            .as_str()
                            == "$id")))
    }

    #[cfg(feature = "cbor_query")]
    /// Converts a query CBOR to a `DriveQuery`.
    pub fn from_cbor(
        query_cbor: &[u8],
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
    ) -> Result<Self, Error> {
        let query_document_value: Value = ciborium::de::from_reader(query_cbor).map_err(|_| {
            Error::Query(QuerySyntaxError::DeserializationError(
                "unable to decode query from cbor".to_string(),
            ))
        })?;
        Self::from_value(query_document_value, contract, document_type, config)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a query Value to a `DriveQuery`.
    pub fn from_value(
        query_value: Value,
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
    ) -> Result<Self, Error> {
        let query_document: BTreeMap<String, Value> = query_value.into_btree_string_map()?;
        Self::from_btree_map_value(query_document, contract, document_type, config)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a query Value to a `DriveQuery`.
    pub fn from_btree_map_value(
        mut query_document: BTreeMap<String, Value>,
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
    ) -> Result<Self, Error> {
        if let Some(contract_id) = query_document
            .remove_optional_identifier("contract_id")
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?
        {
            if contract.id() != contract_id {
                return Err(ProtocolError::IdentifierError(format!(
                    "data contract id mismatch, expected: {}, got: {}",
                    contract.id(),
                    contract_id
                ))
                .into());
            };
        }

        if let Some(document_type_name) = query_document
            .remove_optional_string("document_type_name")
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?
        {
            if document_type.name() != &document_type_name {
                return Err(ProtocolError::IdentifierError(format!(
                    "document type name mismatch, expected: {}, got: {}",
                    document_type.name(),
                    document_type_name
                ))
                .into());
            }
        }

        let maybe_limit: Option<u16> = query_document
            .remove_optional_integer("limit")
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

        let limit = maybe_limit
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0 || limit_value > config.default_query_limit {
                    None
                } else {
                    Some(limit_value)
                }
            })
            .ok_or(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "limit greater than max limit {}",
                config.max_query_limit
            ))))?;

        let offset: Option<u16> = query_document
            .remove_optional_integer("offset")
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

        let block_time_ms: Option<u64> = query_document
            .remove_optional_integer("blockTime")
            .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))?;

        let all_where_clauses: Vec<WhereClause> =
            query_document
                .remove("where")
                .map_or(Ok(vec![]), |id_cbor| {
                    if let Value::Array(clauses) = id_cbor {
                        clauses
                            .iter()
                            .map(|where_clause| {
                                if let Value::Array(clauses_components) = where_clause {
                                    WhereClause::from_components(clauses_components)
                                } else {
                                    Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                                        "where clause must be an array",
                                    )))
                                }
                            })
                            .collect::<Result<Vec<WhereClause>, Error>>()
                    } else {
                        Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                            "where clause must be an array",
                        )))
                    }
                })?;

        let internal_clauses = InternalClauses::extract_from_clauses(all_where_clauses)?;

        let start_at_option = query_document.remove("startAt");
        let start_after_option = query_document.remove("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::Query(QuerySyntaxError::DuplicateStartConditions(
                "only one of startAt or startAfter should be provided",
            )));
        }

        let mut start_at_included = true;

        let mut start_option: Option<Value> = None;

        if start_after_option.is_some() {
            start_option = start_after_option;
            start_at_included = false;
        } else if start_at_option.is_some() {
            start_option = start_at_option;
            start_at_included = true;
        }

        let start_at: Option<[u8; 32]> = start_option
            .map(|v| {
                v.into_identifier()
                    .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))
                    .map(|identifier| identifier.into_buffer())
            })
            .transpose()?;

        let order_by: IndexMap<String, OrderClause> =
            query_document
                .remove("orderBy")
                .map_or(Ok(IndexMap::new()), |id_cbor| {
                    if let Value::Array(clauses) = id_cbor {
                        clauses
                            .into_iter()
                            .filter_map(|order_clause| {
                                if let Value::Array(clauses_components) = order_clause {
                                    let order_clause =
                                        OrderClause::from_components(&clauses_components)
                                            .map_err(Error::GroveDB);
                                    match order_clause {
                                        Ok(order_clause) => {
                                            Some(Ok((order_clause.field.clone(), order_clause)))
                                        }
                                        Err(err) => Some(Err(err)),
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect::<Result<IndexMap<String, OrderClause>, Error>>()
                    } else {
                        Err(Error::Query(QuerySyntaxError::InvalidOrderByProperties(
                            "order clauses must be an array",
                        )))
                    }
                })?;

        if !query_document.is_empty() {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "unsupported syntax in where clause: {:?}",
                query_document
            ))));
        }

        Ok(DriveDocumentQuery {
            contract,
            document_type,
            internal_clauses,
            limit: Some(limit),
            offset,
            order_by,
            start_at,
            start_at_included,
            block_time_ms,
        })
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a query Value to a `DriveQuery`.
    #[allow(clippy::too_many_arguments)]
    pub fn from_decomposed_values(
        where_clause: Value,
        order_by: Option<Value>,
        maybe_limit: Option<u16>,
        start_at: Option<[u8; 32]>,
        start_at_included: bool,
        block_time_ms: Option<u64>,
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
    ) -> Result<Self, Error> {
        let limit = maybe_limit
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0 || limit_value > config.default_query_limit {
                    None
                } else {
                    Some(limit_value)
                }
            })
            .ok_or(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "limit greater than max limit {}",
                config.max_query_limit
            ))))?;

        let all_where_clauses: Vec<WhereClause> = match where_clause {
            Value::Null => Ok(vec![]),
            Value::Array(clauses) => clauses
                .iter()
                .map(|where_clause| {
                    if let Value::Array(clauses_components) = where_clause {
                        WhereClause::from_components(clauses_components)
                    } else {
                        Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                            "where clause must be an array",
                        )))
                    }
                })
                .collect::<Result<Vec<WhereClause>, Error>>(),
            _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                "where clause must be an array",
            ))),
        }?;

        let internal_clauses = InternalClauses::extract_from_clauses(all_where_clauses)?;

        let order_by: IndexMap<String, OrderClause> = order_by
            .map_or(vec![], |id_cbor| {
                if let Value::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .filter_map(|order_clause| {
                            if let Value::Array(clauses_components) = order_clause {
                                OrderClause::from_components(clauses_components).ok()
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            })
            .iter()
            .map(|order_clause| Ok((order_clause.field.clone(), order_clause.to_owned())))
            .collect::<Result<IndexMap<String, OrderClause>, Error>>()?;

        Ok(DriveDocumentQuery {
            contract,
            document_type,
            internal_clauses,
            offset: None,
            limit: Some(limit),
            order_by,
            start_at,
            start_at_included,
            block_time_ms,
        })
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a SQL expression to a `DriveQuery`.
    pub fn from_sql_expr(
        sql_string: &str,
        contract: &'a DataContract,
        config: Option<&DriveConfig>,
    ) -> Result<Self, Error> {
        let dialect: MySqlDialect = MySqlDialect {};
        let statements: Vec<Statement> = Parser::parse_sql(&dialect, sql_string)
            .map_err(|e| Error::Query(QuerySyntaxError::SQLParsingError(e)))?;

        // Should ideally iterate over each statement
        let first_statement =
            statements
                .first()
                .ok_or(Error::Query(QuerySyntaxError::InvalidSQL(
                    "Issue parsing sql getting first statement".to_string(),
                )))?;

        let query: &ast::Query = match first_statement {
            ast::Statement::Query(query_struct) => Some(query_struct),
            _ => None,
        }
        .ok_or(Error::Query(QuerySyntaxError::InvalidSQL(
            "Issue parsing sql: not a query".to_string(),
        )))?;

        let max_limit = config
            .map(|config| config.max_query_limit)
            .unwrap_or(DriveConfig::default().max_query_limit);

        let limit: u16 = if let Some(limit_expr) = &query.limit {
            match limit_expr {
                ast::Expr::Value(Number(num_string, _)) => {
                    let cast_num_string: &String = num_string;
                    let user_limit = cast_num_string.parse::<u16>().map_err(|e| {
                        Error::Query(QuerySyntaxError::InvalidLimit(format!(
                            "limit could not be parsed {}",
                            e
                        )))
                    })?;
                    if user_limit > max_limit {
                        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                            "limit {} greater than max limit {}",
                            user_limit, max_limit
                        ))));
                    }
                    user_limit
                }
                result => {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "expression not a limit {}",
                        result
                    ))));
                }
            }
        } else {
            config
                .map(|config| config.default_query_limit)
                .unwrap_or(DriveConfig::default().default_query_limit)
        };

        let order_by: IndexMap<String, OrderClause> = query
            .order_by
            .iter()
            .map(|order_exp: &OrderByExpr| {
                let ascending = order_exp.asc.is_none() || order_exp.asc.unwrap();
                let field = order_exp.expr.to_string();
                (field.clone(), OrderClause { field, ascending })
            })
            .collect::<IndexMap<String, OrderClause>>();

        // Grab the select section of the query
        let select: &Select = match &*query.body {
            ast::SetExpr::Select(select) => Some(select),
            _ => None,
        }
        .ok_or(Error::Query(QuerySyntaxError::InvalidSQL(
            "Issue parsing sql: Not a select".to_string(),
        )))?;

        // Get the document type from the 'from' section
        let document_type_name = match &select
            .from
            .first()
            .ok_or(Error::Query(QuerySyntaxError::InvalidSQL(
                "Invalid query: missing from section".to_string(),
            )))?
            .relation
        {
            Table { name, .. } => name.0.first().as_ref().map(|identifier| &identifier.value),
            _ => None,
        }
        .ok_or(Error::Query(QuerySyntaxError::InvalidSQL(
            "Issue parsing sql: invalid from value".to_string(),
        )))?;

        let document_type =
            contract
                .document_types()
                .get(document_type_name)
                .ok_or(Error::Query(QuerySyntaxError::DocumentTypeNotFound(
                    "document type not found in contract",
                )))?;

        // Restrictions
        // only binary where clauses are supported
        // i.e. [<fieldname>, <operator>, <value>]
        // [and] is used to separate where clauses
        // currently where clauses are either binary operations or list descriptions (in clauses)
        // hence once [and] is encountered [left] and [right] must be only one of the above
        // i.e other where clauses
        // e.g. firstname = wisdom and lastname = ogwu
        // if op is not [and] then [left] or [right] must not be a binary operation or list description
        let mut all_where_clauses: Vec<WhereClause> = Vec::new();
        let selection_tree = select.selection.as_ref();

        // Where clauses are optional
        if let Some(selection_tree) = selection_tree {
            WhereClause::build_where_clauses_from_operations(
                selection_tree,
                document_type,
                &mut all_where_clauses,
            )?;
        }

        let internal_clauses = InternalClauses::extract_from_clauses(all_where_clauses)?;

        let start_at_option = None; //todo
        let start_after_option = None; //todo
        let mut start_at_included = true;
        let mut start_option: Option<Value> = None;

        if start_after_option.is_some() {
            start_option = start_after_option;
            start_at_included = false;
        } else if start_at_option.is_some() {
            start_option = start_at_option;
            start_at_included = true;
        }

        let start_at: Option<[u8; 32]> = start_option
            .map(|v| {
                v.into_identifier()
                    .map_err(|e| Error::Protocol(ProtocolError::ValueError(e)))
                    .map(|identifier| identifier.into_buffer())
            })
            .transpose()?;

        Ok(DriveDocumentQuery {
            contract,
            document_type: document_type.as_ref(),
            internal_clauses,
            offset: None,
            limit: Some(limit),
            order_by,
            start_at,
            start_at_included,
            block_time_ms: None,
        })
    }

    /// Serialize drive query to CBOR format.
    ///
    /// FIXME: The data contract is only referred as ID, and document type as its name.
    /// This can change in the future to include full data contract and document type.
    #[cfg(feature = "cbor_query")]
    pub fn to_cbor(&self) -> Result<Vec<u8>, Error> {
        let data: BTreeMap<String, Value> = self.into();
        let cbor: BTreeMap<String, ciborium::Value> = Value::convert_to_cbor_map(data)?;
        let mut output = Vec::new();

        ciborium::ser::into_writer(&cbor, &mut output)
            .map_err(|e| ProtocolError::PlatformSerializationError(e.to_string()))?;
        Ok(output)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Operations to construct a path query.
    pub fn start_at_document_path_and_key(&self, starts_at: &[u8; 32]) -> (Vec<Vec<u8>>, Vec<u8>) {
        if self.document_type.documents_keep_history() {
            let document_holding_path = self.contract.documents_with_history_primary_key_path(
                self.document_type.name().as_str(),
                starts_at,
            );
            (
                document_holding_path
                    .into_iter()
                    .map(|key| key.to_vec())
                    .collect::<Vec<_>>(),
                vec![0],
            )
        } else {
            let document_holding_path = self
                .contract
                .documents_primary_key_path(self.document_type.name().as_str());
            (
                document_holding_path
                    .into_iter()
                    .map(|key| key.to_vec())
                    .collect::<Vec<_>>(),
                starts_at.to_vec(),
            )
        }
    }

    #[cfg(feature = "server")]
    /// Operations to construct a path query.
    pub fn construct_path_query_operations(
        &self,
        drive: &Drive,
        include_start_at_for_proof: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let drive_version = &platform_version.drive;
        // First we should get the overall document_type_path
        let document_type_path = self
            .contract
            .document_type_path(self.document_type.name().as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();

        let (starts_at_document, start_at_path_query) = match &self.start_at {
            None => Ok((None, None)),
            Some(starts_at) => {
                // First if we have a startAt or startsAfter we must get the element
                // from the backing store

                let (start_at_document_path, start_at_document_key) =
                    self.start_at_document_path_and_key(starts_at);
                let start_at_document = drive
                    .grove_get(
                        start_at_document_path.as_slice().into(),
                        &start_at_document_key,
                        StatefulQuery,
                        transaction,
                        drive_operations,
                        drive_version,
                    )
                    .map_err(|e| match e {
                        Error::GroveDB(GroveError::PathKeyNotFound(_))
                        | Error::GroveDB(GroveError::PathNotFound(_))
                        | Error::GroveDB(GroveError::PathParentLayerNotFound(_)) => {
                            let error_message = if self.start_at_included {
                                "startAt document not found"
                            } else {
                                "startAfter document not found"
                            };

                            Error::Query(QuerySyntaxError::StartDocumentNotFound(error_message))
                        }
                        _ => e,
                    })?
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected a value",
                    )))?;

                let path_query =
                    PathQuery::new_single_key(start_at_document_path, start_at_document_key);

                if let Element::Item(item, _) = start_at_document {
                    let document = Document::from_bytes(
                        item.as_slice(),
                        self.document_type,
                        platform_version,
                    )?;
                    Ok((Some((document, self.start_at_included)), Some(path_query)))
                } else {
                    Err(Error::Drive(DriveError::CorruptedDocumentPath(
                        "Holding paths should only have items",
                    )))
                }
            }
        }?;
        let mut main_path_query = if self.is_for_primary_key() {
            self.get_primary_key_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        } else {
            self.get_non_primary_key_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        }?;
        if !include_start_at_for_proof {
            return Ok(main_path_query);
        }

        if let Some(start_at_path_query) = start_at_path_query {
            let limit = main_path_query.query.limit.take();
            let mut merged = PathQuery::merge(
                vec![&start_at_path_query, &main_path_query],
                &platform_version.drive.grove_version,
            )
            .map_err(Error::GroveDB)?;
            merged.query.limit = limit.map(|a| a.saturating_add(1));
            Ok(merged)
        } else {
            Ok(main_path_query)
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Operations to construct a path query.
    pub fn construct_path_query(
        &self,
        starts_at_document: Option<Document>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // First we should get the overall document_type_path
        let document_type_path = self
            .contract
            .document_type_path(self.document_type.name().as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();
        let starts_at_document = starts_at_document
            .map(|starts_at_document| (starts_at_document, self.start_at_included));
        if self.is_for_primary_key() {
            self.get_primary_key_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        } else {
            self.get_non_primary_key_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a path query given a document type path and starting document.
    pub fn get_primary_key_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let mut path = document_type_path;

        // Add primary key ($id) subtree
        path.push(vec![0]);

        if let Some(primary_key_equal_clause) = &self.internal_clauses.primary_key_equal_clause {
            let mut query = Query::new();
            let key = self.document_type.serialize_value_for_key(
                "$id",
                &primary_key_equal_clause.value,
                platform_version,
            )?;
            query.insert_key(key);

            if self.document_type.documents_keep_history() {
                // if the documents keep history then we should insert a subquery
                if let Some(block_time) = self.block_time_ms {
                    let encoded_block_time = encode_u64(block_time);
                    let mut sub_query = Query::new_with_direction(false);
                    sub_query.insert_range_to_inclusive(..=encoded_block_time);
                    query.set_subquery(sub_query);
                } else {
                    query.set_subquery_key(vec![0]);
                }
            }

            Ok(PathQuery::new(path, SizedQuery::new(query, Some(1), None)))
        } else {
            // This is for a range
            let left_to_right = if self.order_by.keys().len() == 1 {
                if self.order_by.keys().next().unwrap() != "$id" {
                    return Err(Error::Query(QuerySyntaxError::InvalidOrderByProperties(
                        "order by should include $id only",
                    )));
                }

                let order_clause = self.order_by.get("$id").unwrap();

                order_clause.ascending
            } else {
                true
            };

            let mut query = Query::new_with_direction(left_to_right);
            // If there is a start_at_document, we need to get the value that it has for the
            // current field.
            let starts_at_key_option = match starts_at_document {
                None => None,
                Some((document, included)) => {
                    // if the key doesn't exist then we should ignore the starts at key
                    document
                        .get_raw_for_document_type(
                            "$id",
                            self.document_type,
                            None,
                            platform_version,
                        )?
                        .map(|raw_value_option| (raw_value_option, included))
                }
            };

            if let Some(primary_key_in_clause) = &self.internal_clauses.primary_key_in_clause {
                let in_values = primary_key_in_clause.in_values()?;

                match starts_at_key_option {
                    None => {
                        for value in in_values.iter() {
                            let key = self.document_type.serialize_value_for_key(
                                "$id",
                                value,
                                platform_version,
                            )?;
                            query.insert_key(key)
                        }
                    }
                    Some((starts_at_key, included)) => {
                        for value in in_values.iter() {
                            let key = self.document_type.serialize_value_for_key(
                                "$id",
                                value,
                                platform_version,
                            )?;

                            if (left_to_right && starts_at_key < key)
                                || (!left_to_right && starts_at_key > key)
                                || (included && starts_at_key == key)
                            {
                                query.insert_key(key);
                            }
                        }
                    }
                }

                if self.document_type.documents_keep_history() {
                    // if the documents keep history then we should insert a subquery
                    if let Some(_block_time) = self.block_time_ms {
                        //todo
                        return Err(Error::Query(QuerySyntaxError::Unsupported(
                            "Not yet implemented".to_string(),
                        )));
                        // in order to be able to do this we would need limited subqueries
                        // as we only want the first element before the block_time

                        // let encoded_block_time = encode_float(block_time)?;
                        // let mut sub_query = Query::new_with_direction(false);
                        // sub_query.insert_range_to_inclusive(..=encoded_block_time);
                        // query.set_subquery(sub_query);
                    } else {
                        query.set_subquery_key(vec![0]);
                    }
                }

                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, self.limit, self.offset),
                ))
            } else {
                // this is a range on all elements
                match starts_at_key_option {
                    None => {
                        query.insert_all();
                    }
                    Some((starts_at_key, included)) => match left_to_right {
                        true => match included {
                            true => query.insert_range_from(starts_at_key..),
                            false => query.insert_range_after(starts_at_key..),
                        },
                        false => match included {
                            true => query.insert_range_to_inclusive(..=starts_at_key),
                            false => query.insert_range_to(..starts_at_key),
                        },
                    },
                }

                if self.document_type.documents_keep_history() {
                    // if the documents keep history then we should insert a subquery
                    if let Some(_block_time) = self.block_time_ms {
                        return Err(Error::Query(QuerySyntaxError::Unsupported(
                            "this query is not supported".to_string(),
                        )));
                        // in order to be able to do this we would need limited subqueries
                        // as we only want the first element before the block_time

                        // let encoded_block_time = encode_float(block_time)?;
                        // let mut sub_query = Query::new_with_direction(false);
                        // sub_query.insert_range_to_inclusive(..=encoded_block_time);
                        // query.set_subquery(sub_query);
                    } else {
                        query.set_subquery_key(vec![0]);
                    }
                }

                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, self.limit, self.offset),
                ))
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Finds the best index for the query.
    pub fn find_best_index(&self, platform_version: &PlatformVersion) -> Result<&Index, Error> {
        let equal_fields = self
            .internal_clauses
            .equal_clauses
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();
        let in_field = self
            .internal_clauses
            .in_clause
            .as_ref()
            .map(|in_clause| in_clause.field.as_str());
        let range_field = self
            .internal_clauses
            .range_clause
            .as_ref()
            .map(|range_clause| range_clause.field.as_str());
        let mut fields = equal_fields;
        if let Some(range_field) = range_field {
            fields.push(range_field);
        }
        if let Some(in_field) = in_field {
            fields.push(in_field);
            //if there is an in_field, it always takes precedence
        }

        let order_by_keys: Vec<&str> = self
            .order_by
            .keys()
            .map(|key: &String| {
                let str = key.as_str();
                if !fields.contains(&str) {
                    fields.push(str);
                }
                str
            })
            .collect();

        let (index, difference) = self
            .document_type
            .index_for_types(
                fields.as_slice(),
                in_field,
                order_by_keys.as_slice(),
                platform_version,
            )?
            .ok_or(Error::Query(
                QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
                    "query must be for valid indexes, valid indexes are: {:?}",
                    self.document_type.indexes()
                )),
            ))?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Err(Error::Query(QuerySyntaxError::QueryTooFarFromIndex(
                "query must better match an existing index",
            )));
        }
        Ok(index)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `QueryItem` given a start key and query direction.
    pub fn query_item_for_starts_at_key(starts_at_key: Vec<u8>, left_to_right: bool) -> QueryItem {
        if left_to_right {
            QueryItem::RangeAfter(starts_at_key..)
        } else {
            QueryItem::RangeTo(..starts_at_key)
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given document ID if given.
    fn inner_query_from_starts_at_for_id(
        starts_at_document: Option<&StartAtDocument>,
        left_to_right: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);

        if let Some(StartAtDocument {
            document, included, ..
        }) = starts_at_document
        {
            let start_at_key = document.id().to_vec();
            if *included {
                inner_query.insert_range_from(start_at_key..)
            } else {
                inner_query.insert_range_after(start_at_key..)
            }
        } else {
            // No starts at document, take all NULL items
            inner_query.insert_all();
        }
        inner_query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given key.
    fn inner_query_starts_from_key(
        start_at_key: Option<Vec<u8>>,
        left_to_right: bool,
        included: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);

        if left_to_right {
            if let Some(start_at_key) = start_at_key {
                if included {
                    inner_query.insert_range_from(start_at_key..);
                } else {
                    inner_query.insert_range_after(start_at_key..);
                }
            } else {
                inner_query.insert_all();
            }
        } else if included {
            if let Some(start_at_key) = start_at_key {
                inner_query.insert_range_to_inclusive(..=start_at_key);
            } else {
                inner_query.insert_key(vec![]);
            }
        } else if let Some(start_at_key) = start_at_key {
            inner_query.insert_range_to(..start_at_key);
        } else {
            //todo: really not sure if this is correct
            // Should investigate more
            inner_query.insert_key(vec![]);
        }

        inner_query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given document if given.
    fn inner_query_from_starts_at(
        starts_at_document: Option<&StartAtDocument>,
        indexed_property: &IndexProperty,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Query, Error> {
        let mut inner_query = Query::new_with_direction(left_to_right);
        if let Some(StartAtDocument {
            document,
            document_type,
            included,
        }) = starts_at_document
        {
            // We only need items after the start at document
            let start_at_key = document.get_raw_for_document_type(
                indexed_property.name.as_str(),
                *document_type,
                None,
                platform_version,
            )?;
            // We want to get items starting at the start key
            if let Some(start_at_key) = start_at_key {
                if left_to_right {
                    if *included {
                        inner_query.insert_range_from(start_at_key..)
                    } else {
                        inner_query.insert_range_after(start_at_key..)
                    }
                } else if *included {
                    inner_query.insert_range_to_inclusive(..=start_at_key)
                } else {
                    inner_query.insert_range_to(..start_at_key)
                }
            } else if left_to_right {
                inner_query.insert_all();
            } else {
                inner_query.insert_key(vec![]);
            }
        } else {
            // No starts at document, take all NULL items
            inner_query.insert_all();
        }
        Ok(inner_query)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    fn recursive_create_query(
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: Option<&StartAtDocument>, //for key level, included
        indexed_property: &IndexProperty,
        order_by: Option<&IndexMap<String, OrderClause>>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Query>, Error> {
        match left_over_index_properties.split_first() {
            None => Ok(None),
            Some((first, left_over)) => {
                let left_to_right = if let Some(order_by) = order_by {
                    order_by
                        .get(first.name.as_str())
                        .map(|order_clause| order_clause.ascending)
                        .unwrap_or(first.ascending)
                } else {
                    first.ascending
                };

                let mut inner_query = Self::inner_query_from_starts_at(
                    starts_at_document,
                    indexed_property,
                    left_to_right,
                    platform_version,
                )?;
                DriveDocumentQuery::recursive_insert_on_query(
                    &mut inner_query,
                    left_over,
                    unique,
                    starts_at_document,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;
                Ok(Some(inner_query))
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Recursively queries as long as there are leftover index properties.
    /// The in_start_at_document_sub_path_needing_conditional is interesting.
    /// It indicates whether the start at document should be applied as a conditional
    /// For example if we have a tree
    /// Root
    /// ├── model
    /// │   ├── sedan
    /// │   │   ├── brand_name
    /// │   │   │   ├── Honda
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Accord
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── a47d2...
    /// │   │   │   │   │   │   │   ├── e19c8...
    /// │   │   │   │   │   │   │   └── f1a7b...
    /// │   │   │   │   │   └── Civic
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── b65a7...
    /// │   │   │   │   │       │   └── c43de...
    /// │   │   │   ├── Toyota
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Camry
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   └── 1a9d2...
    /// │   │   │   │   │   └── Corolla
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 3f7b4...
    /// │   │   │   │   │       │   ├── 4e8fa...
    /// │   │   │   │   │       │   └── 9b1c6...
    /// │   ├── suv
    /// │   │   ├── brand_name
    /// │   │   │   ├── Ford*
    /// │   │   │   │   ├── car_type*
    /// │   │   │   │   │   ├── Escape*
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 102bc...
    /// │   │   │   │   │   │   │   ├── 29f8e... <- Set After this document
    /// │   │   │   │   │   │   │   └── 6b1a3...
    /// │   │   │   │   │   └── Explorer
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── b2a9d...
    /// │   │   │   │   │       │   └── f4d5c...
    /// │   │   │   ├── Nissan
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Rogue
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 5a9c3...
    /// │   │   │   │   │   │   │   └── 7e4b9...
    /// │   │   │   │   │   └── Murano
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 8f6a2...
    /// │   │   │   │   │       │   └── 9c7d4...
    /// │   ├── truck
    /// │   │   ├── brand_name
    /// │   │   │   ├── Ford
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── F-150
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 72a3b...
    /// │   │   │   │   │   │   │   └── 94c8e...
    /// │   │   │   │   │   └── Ranger
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 3f4b1...
    /// │   │   │   │   │       │   ├── 6e7d2...
    /// │   │   │   │   │       │   └── 8a1f5...
    /// │   │   │   ├── Toyota
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Tundra
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 7c9a4...
    /// │   │   │   │   │   │   │   └── a5d1e...
    /// │   │   │   │   │   └── Tacoma
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 1e7f4...
    /// │   │   │   │   │       │   └── 6b9d3...
    ///
    /// let's say we are asking for suv's after 29f8e
    /// here the * denotes the area needing a conditional
    /// We need a conditional subquery on Ford to say only things after Ford (with Ford included)
    /// We need a conditional subquery on Escape to say only things after Escape (with Escape included)
    fn recursive_insert_on_query(
        query: &mut Query,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: Option<&StartAtDocument>, //for key level, included
        default_left_to_right: bool,
        order_by: Option<&IndexMap<String, OrderClause>>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Query>, Error> {
        match left_over_index_properties.split_first() {
            None => {
                match unique {
                    true => {
                        query.set_subquery_key(vec![0]);

                        // In the case things are NULL we allow to have multiple values
                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            starts_at_document,
                            true, //for ids we always go left to right
                        );
                        query.add_conditional_subquery(
                            QueryItem::Key(b"".to_vec()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                    false => {
                        query.set_subquery_key(vec![0]);
                        // we just get all by document id order ascending
                        let full_query =
                            Self::inner_query_from_starts_at_for_id(None, default_left_to_right);
                        query.set_subquery(full_query);

                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            starts_at_document,
                            default_left_to_right,
                        );

                        query.add_conditional_subquery(
                            QueryItem::Key(b"".to_vec()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                }
                Ok(None)
            }
            Some((first, left_over)) => {
                let left_to_right = if let Some(order_by) = order_by {
                    order_by
                        .get(first.name.as_str())
                        .map(|order_clause| order_clause.ascending)
                        .unwrap_or(first.ascending)
                } else {
                    first.ascending
                };

                if let Some(start_at_document_inner) = starts_at_document {
                    let StartAtDocument {
                        document,
                        document_type,
                        included,
                    } = start_at_document_inner;
                    let start_at_key = document
                        .get_raw_for_document_type(
                            first.name.as_str(),
                            *document_type,
                            None,
                            platform_version,
                        )
                        .ok()
                        .flatten();

                    // We should always include if we have left_over
                    let non_conditional_included =
                        !left_over.is_empty() || *included || start_at_key.is_none();

                    let mut non_conditional_query = Self::inner_query_starts_from_key(
                        start_at_key.clone(),
                        left_to_right,
                        non_conditional_included,
                    );

                    // We place None here on purpose, this has been well-thought-out
                    // and should not change. The reason is that the path of the start
                    // at document is used only on the conditional subquery and not on the
                    // main query
                    // for example in the following
                    // Our query will be with $ownerId == a3f9b81c4d7e6a9f5b1c3e8a2d9c4f7b
                    // With start after 8f2d5
                    // We want to get from 2024-11-17T12:45:00Z
                    // withdrawal
                    // ├── $ownerId
                    // │   ├── a3f9b81c4d7e6a9f5b1c3e8a2d9c4f7b
                    // │   │   ├── $updatedAt
                    // │   │   │   ├── 2024-11-17T12:45:00Z <- conditional subquery here
                    // │   │   │   │   ├── status
                    // │   │   │   │   │   ├── 0
                    // │   │   │   │   │   │   ├── 7a9f1...
                    // │   │   │   │   │   │   └── 4b8c3...
                    // │   │   │   │   │   ├── 1
                    // │   │   │   │   │   │   ├── 8f2d5... <- start after
                    // │   │   │   │   │   │   └── 5c1e4...
                    // │   │   │   │   │   ├── 2
                    // │   │   │   │   │   │   ├── 2e7a9...
                    // │   │   │   │   │   │   └── 1c8b3...
                    // │   │   │   ├── 2024-11-18T11:25:00Z <- we want all statuses here, so normal subquery, with None as start at document
                    // │   │   │   │   ├── status
                    // │   │   │   │   │   ├── 0
                    // │   │   │   │   │   │   └── 1a4f2...
                    // │   │   │   │   │   ├── 2
                    // │   │   │   │   │   │   ├── 3e7a9...
                    // │   │   │   │   │   │   └── 198b4...
                    // │   ├── b6d7e9c4a5f2b3d8e1a7c9f4b1e8a3f
                    // │   │   ├── $updatedAt
                    // │   │   │   ├── 2024-11-17T13:30:00Z
                    // │   │   │   │   ├── status
                    // │   │   │   │   │   ├── 0
                    // │   │   │   │   │   │   ├── 6d7e2...
                    // │   │   │   │   │   │   └── 9c7f5...
                    // │   │   │   │   │   ├── 3
                    // │   │   │   │   │   │   ├── 3a9b7...
                    // │   │   │   │   │   │   └── 8e5c4...
                    // │   │   │   │   │   ├── 4
                    // │   │   │   │   │   │   ├── 1f7a8...
                    // │   │   │   │   │   │   └── 2c9b3...
                    // println!("going to call recursive_insert_on_query on non_conditional_query {} with left_over {:?}", non_conditional_query, left_over);
                    DriveDocumentQuery::recursive_insert_on_query(
                        &mut non_conditional_query,
                        left_over,
                        unique,
                        None,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;

                    DriveDocumentQuery::recursive_conditional_insert_on_query(
                        &mut non_conditional_query,
                        start_at_key,
                        left_over,
                        unique,
                        start_at_document_inner,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;

                    query.set_subquery(non_conditional_query);
                } else {
                    let mut inner_query = Query::new_with_direction(first.ascending);
                    inner_query.insert_all();
                    DriveDocumentQuery::recursive_insert_on_query(
                        &mut inner_query,
                        left_over,
                        unique,
                        starts_at_document,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;
                    query.set_subquery(inner_query);
                }
                query.set_subquery_key(first.name.as_bytes().to_vec());
                Ok(None)
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    #[allow(clippy::too_many_arguments)]
    fn recursive_conditional_insert_on_query(
        query: &mut Query,
        conditional_value: Option<Vec<u8>>,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: &StartAtDocument,
        default_left_to_right: bool,
        order_by: Option<&IndexMap<String, OrderClause>>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match left_over_index_properties.split_first() {
            None => {
                match unique {
                    true => {
                        // In the case things are NULL we allow to have multiple values
                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            Some(starts_at_document),
                            true, //for ids we always go left to right
                        );
                        query.add_conditional_subquery(
                            QueryItem::Key(b"".to_vec()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                    false => {
                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            Some(starts_at_document),
                            default_left_to_right,
                        );

                        query.add_conditional_subquery(
                            QueryItem::Key(conditional_value.unwrap_or_default()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                }
            }
            Some((first, left_over)) => {
                let left_to_right = if let Some(order_by) = order_by {
                    order_by
                        .get(first.name.as_str())
                        .map(|order_clause| order_clause.ascending)
                        .unwrap_or(first.ascending)
                } else {
                    first.ascending
                };

                let StartAtDocument {
                    document,
                    document_type,
                    ..
                } = starts_at_document;

                let lower_start_at_key = document
                    .get_raw_for_document_type(
                        first.name.as_str(),
                        *document_type,
                        None,
                        platform_version,
                    )
                    .ok()
                    .flatten();

                // We include it if we are not unique,
                // or if we are unique but the value is empty
                let non_conditional_included = !unique || lower_start_at_key.is_none();

                let mut non_conditional_query = Self::inner_query_starts_from_key(
                    lower_start_at_key.clone(),
                    left_to_right,
                    non_conditional_included,
                );

                DriveDocumentQuery::recursive_insert_on_query(
                    &mut non_conditional_query,
                    left_over,
                    unique,
                    None,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;

                DriveDocumentQuery::recursive_conditional_insert_on_query(
                    &mut non_conditional_query,
                    lower_start_at_key,
                    left_over,
                    unique,
                    starts_at_document,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;

                query.add_conditional_subquery(
                    QueryItem::Key(conditional_value.unwrap_or_default()),
                    Some(vec![first.name.as_bytes().to_vec()]),
                    Some(non_conditional_query),
                );
            }
        }
        Ok(())
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a path query for non-primary keys given a document type path and starting document.
    pub fn get_non_primary_key_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let index = self.find_best_index(platform_version)?;
        let ordered_clauses: Vec<&WhereClause> = index
            .properties
            .iter()
            .filter_map(|field| self.internal_clauses.equal_clauses.get(field.name.as_str()))
            .collect();
        let (last_clause, last_clause_is_range, subquery_clause) =
            match &self.internal_clauses.in_clause {
                None => match &self.internal_clauses.range_clause {
                    None => (ordered_clauses.last().copied(), false, None),
                    Some(where_clause) => (Some(where_clause), true, None),
                },
                Some(in_clause) => match &self.internal_clauses.range_clause {
                    None => (Some(in_clause), true, None),
                    Some(range_clause) => (Some(in_clause), true, Some(range_clause)),
                },
            };

        // We need to get the terminal indexes unused by clauses.
        let left_over_index_properties = index
            .properties
            .iter()
            .filter(|field| {
                !(self
                    .internal_clauses
                    .equal_clauses
                    .contains_key(field.name.as_str())
                    || (last_clause.is_some() && last_clause.unwrap().field == field.name)
                    || (subquery_clause.is_some() && subquery_clause.unwrap().field == field.name))
            })
            .collect::<Vec<&IndexProperty>>();

        let intermediate_values = index
            .properties
            .iter()
            .filter_map(|field| {
                match self.internal_clauses.equal_clauses.get(field.name.as_str()) {
                    None => None,
                    Some(where_clause) => {
                        if !last_clause_is_range
                            && last_clause.is_some()
                            && last_clause.unwrap().field == field.name
                        {
                            //there is no need to give an intermediate value as the last clause is an equality
                            None
                        } else {
                            Some(self.document_type.serialize_value_for_key(
                                field.name.as_str(),
                                &where_clause.value,
                                platform_version,
                            ))
                        }
                    }
                }
            })
            .collect::<Result<Vec<Vec<u8>>, ProtocolError>>()
            .map_err(Error::Protocol)?;

        let final_query = match last_clause {
            None => {
                // There is no last_clause which means we are using an index most likely because of an order_by, however we have no
                // clauses, in this case we should use the first value of the index.
                let first_index = index.properties.first().ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("index must have properties".to_string()),
                ))?; // Index must have properties
                Self::recursive_create_query(
                    left_over_index_properties.as_slice(),
                    index.unique,
                    starts_at_document
                        .map(|(document, included)| StartAtDocument {
                            document,
                            document_type: self.document_type,
                            included,
                        })
                        .as_ref(),
                    first_index,
                    None,
                    platform_version,
                )?
                .expect("Index must have left over properties if no last clause")
            }
            Some(where_clause) => {
                let left_to_right = if where_clause.operator.is_range() {
                    let order_clause: &OrderClause = self
                        .order_by
                        .get(where_clause.field.as_str())
                        .ok_or(Error::Query(QuerySyntaxError::MissingOrderByForRange(
                            "query must have an orderBy field for each range element",
                        )))?;

                    order_clause.ascending
                } else {
                    true
                };

                // We should set the starts at document to be included for the query if there are
                // left over index properties.

                let query_starts_at_document = if left_over_index_properties.is_empty() {
                    &starts_at_document
                } else {
                    &None
                };

                let mut query = where_clause.to_path_query(
                    self.document_type,
                    query_starts_at_document,
                    left_to_right,
                    platform_version,
                )?;

                match subquery_clause {
                    None => {
                        Self::recursive_insert_on_query(
                            &mut query,
                            left_over_index_properties.as_slice(),
                            index.unique,
                            starts_at_document
                                .map(|(document, included)| StartAtDocument {
                                    document,
                                    document_type: self.document_type,
                                    included,
                                })
                                .as_ref(),
                            left_to_right,
                            Some(&self.order_by),
                            platform_version,
                        )?;
                    }
                    Some(subquery_where_clause) => {
                        let order_clause: &OrderClause = self
                            .order_by
                            .get(subquery_where_clause.field.as_str())
                            .ok_or(Error::Query(QuerySyntaxError::MissingOrderByForRange(
                                "query must have an orderBy field for each range element",
                            )))?;
                        let mut subquery = subquery_where_clause.to_path_query(
                            self.document_type,
                            &starts_at_document,
                            order_clause.ascending,
                            platform_version,
                        )?;
                        Self::recursive_insert_on_query(
                            &mut subquery,
                            left_over_index_properties.as_slice(),
                            index.unique,
                            starts_at_document
                                .map(|(document, included)| StartAtDocument {
                                    document,
                                    document_type: self.document_type,
                                    included,
                                })
                                .as_ref(),
                            left_to_right,
                            Some(&self.order_by),
                            platform_version,
                        )?;
                        let subindex = subquery_where_clause.field.as_bytes().to_vec();
                        query.set_subquery_key(subindex);
                        query.set_subquery(subquery);
                    }
                };

                query
            }
        };

        let (intermediate_indexes, last_indexes) =
            index.properties.split_at(intermediate_values.len());

        // Now we should construct the path
        let last_index = last_indexes.first().ok_or(Error::Query(
            QuerySyntaxError::QueryOnDocumentTypeWithNoIndexes(
                "document query has no index with fields",
            ),
        ))?;

        let mut path = document_type_path;

        for (intermediate_index, intermediate_value) in
            intermediate_indexes.iter().zip(intermediate_values.iter())
        {
            path.push(intermediate_index.name.as_bytes().to_vec());
            path.push(intermediate_value.as_slice().to_vec());
        }

        path.push(last_index.name.as_bytes().to_vec());

        Ok(PathQuery::new(
            path,
            SizedQuery::new(final_query, self.limit, self.offset),
        ))
    }

    #[cfg(feature = "server")]
    /// Executes a query with proof and returns the items and fee.
    pub fn execute_with_proof(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations = vec![];
        let items = self.execute_with_proof_internal(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with proof and returns the items.
    pub(crate) fn execute_with_proof_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            true,
            transaction,
            drive_operations,
            platform_version,
        )?;
        drive.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(all(feature = "server", feature = "verify"))]
    /// Executes a query with proof and returns the root hash, items, and fee.
    pub fn execute_with_proof_only_get_elements(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>, u64), Error> {
        let mut drive_operations = vec![];
        let (root_hash, items) = self.execute_with_proof_only_get_elements_internal(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((root_hash, items, cost))
    }

    #[cfg(all(feature = "server", feature = "verify"))]
    /// Executes an internal query with proof and returns the root hash and values.
    pub(crate) fn execute_with_proof_only_get_elements_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            true,
            transaction,
            drive_operations,
            platform_version,
        )?;

        let proof = drive.grove_get_proved_path_query(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        self.verify_proof_keep_serialized(proof.as_slice(), platform_version)
    }

    #[cfg(feature = "server")]
    /// Executes a query with no proof and returns the items, skipped items, and fee.
    pub fn execute_raw_results_no_proof(
        &self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations = vec![];
        let (items, skipped) = self.execute_raw_results_no_proof_internal(
            drive,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let cost = if let Some(block_info) = block_info {
            let fee_result = Drive::calculate_fee(
                None,
                Some(drive_operations),
                &block_info.epoch,
                drive.config.epochs_per_era,
                platform_version,
                None,
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, skipped, cost))
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_raw_results_no_proof_internal(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;

        let query_result = drive.grove_get_path_query_serialized_results(
            &path_query,
            transaction,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => Ok((Vec::new(), 0)),
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
            }
        }
    }

    #[cfg(feature = "server")]
    /// Executes an internal query with no proof and returns the values and skipped items.
    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &Drive,
        result_type: QueryResultType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(QueryResultElements, u16), Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            false,
            transaction,
            drive_operations,
            platform_version,
        )?;
        let query_result = drive.grove_get_path_query(
            &path_query,
            transaction,
            result_type,
            drive_operations,
            &platform_version.drive,
        );
        match query_result {
            Err(Error::GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathNotFound(_)))
            | Err(Error::GroveDB(GroveError::PathParentLayerNotFound(_))) => {
                Ok((QueryResultElements::new(), 0))
            }
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
            }
        }
    }
}

/// Convert DriveQuery to a BTreeMap of values
impl<'a> From<&DriveDocumentQuery<'a>> for BTreeMap<String, Value> {
    fn from(query: &DriveDocumentQuery<'a>) -> Self {
        let mut response = BTreeMap::<String, Value>::new();

        //  contract
        // TODO: once contract can be serialized, maybe put full contract here instead of id
        response.insert(
            "contract_id".to_string(),
            Value::Identifier(query.contract.id().to_buffer()),
        );

        // document_type
        // TODO: once DocumentType can be serialized, maybe put full DocumentType instead of name
        response.insert(
            "document_type_name".to_string(),
            Value::Text(query.document_type.name().to_string()),
        );

        // Internal clauses
        let all_where_clauses: Vec<WhereClause> = query.internal_clauses.clone().into();
        response.insert(
            "where".to_string(),
            Value::Array(all_where_clauses.into_iter().map(|v| v.into()).collect()),
        );

        // Offset
        if let Some(offset) = query.offset {
            response.insert("offset".to_string(), Value::U16(offset));
        };
        // Limit
        if let Some(limit) = query.limit {
            response.insert("limit".to_string(), Value::U16(limit));
        };
        // Order by
        let order_by = &query.order_by;
        let value: Vec<Value> = order_by
            .into_iter()
            .map(|(_k, v)| v.clone().into())
            .collect();
        response.insert("orderBy".to_string(), Value::Array(value));

        // start_at, start_at_included
        if let Some(start_at) = query.start_at {
            let v = Value::Identifier(start_at);
            if query.start_at_included {
                response.insert("startAt".to_string(), v);
            } else {
                response.insert("startAfter".to_string(), v);
            }
        };

        // block_time_ms
        if let Some(block_time_ms) = query.block_time_ms {
            response.insert("blockTime".to_string(), Value::U64(block_time_ms));
        };

        response
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {

    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    use dpp::prelude::Identifier;
    use grovedb::Query;
    use indexmap::IndexMap;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use serde_json::json;
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    use std::option::Option::None;
    use tempfile::TempDir;

    use crate::drive::Drive;
    use crate::query::{
        DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator,
    };
    use crate::util::storage_flags::StorageFlags;

    use dpp::data_contract::DataContract;

    use serde_json::Value::Null;

    use crate::config::DriveConfig;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::DocumentV0;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::Value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::tests::fixtures::{get_data_contract_fixture, get_dpns_data_contract_fixture};
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::util::cbor_serializer;
    use dpp::version::PlatformVersion;

    fn setup_family_contract() -> (Drive, DataContract) {
        let tmp_dir = TempDir::new().unwrap();

        let platform_version = PlatformVersion::latest();

        let (drive, _) = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/family/family-contract.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get document");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                storage_flags,
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_withdrawal_contract() -> (Drive, DataContract) {
        let tmp_dir = TempDir::new().unwrap();

        let platform_version = PlatformVersion::latest();

        let (drive, _) = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        // let's construct the grovedb structure for the dashpay data contract
        let contract = load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
            .expect("load system contact");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                storage_flags,
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_family_birthday_contract() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let contract_path =
            "tests/supporting_files/contract/family/family-contract-with-birthday.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path, false, platform_version)
            .expect("expected to get document");
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                storage_flags,
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[test]
    fn test_drive_query_from_to_cbor() {
        let config = DriveConfig::default();
        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");
        let start_after = Identifier::random();

        let query_value = json!({
            "contract_id": contract.id(),
            "document_type_name": document_type.name(),
            "where": [
                ["firstName", "<", "Gilligan"],
                ["lastName", "=", "Doe"]
            ],
            "limit": 100u16,
            "offset": 10u16,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "desc"],
            ],
            "startAfter": start_after,
            "blockTime": 13453432u64,
        });

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query =
            DriveDocumentQuery::from_cbor(where_cbor.as_slice(), &contract, document_type, &config)
                .expect("deserialize cbor shouldn't fail");

        let cbor = query.to_cbor().expect("should serialize cbor");

        let deserialized = DriveDocumentQuery::from_cbor(&cbor, &contract, document_type, &config)
            .expect("should deserialize cbor");

        assert_eq!(query, deserialized);

        assert_eq!(deserialized.start_at, Some(start_after.to_buffer()));
        assert!(!deserialized.start_at_included);
        assert_eq!(deserialized.block_time_ms, Some(13453432u64));
    }

    #[test]
    fn test_invalid_query_ranges_different_fields() {
        let query_value = json!({
            "where": [
                ["firstName", "<", "Gilligan"],
                ["lastName", "<", "Michelle"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("all ranges must be on same field");
    }

    #[test]
    fn test_invalid_query_extra_invalid_field() {
        let query_value = json!({
            "where": [
                ["firstName", "<", "Gilligan"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
            "invalid": 0,
        });
        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("fields of queries must of defined supported types (where, limit, orderBy...)");
    }

    #[test]
    fn test_invalid_query_conflicting_clauses() {
        let query_value = json!({
            "where": [
                ["firstName", "<", "Gilligan"],
                ["firstName", ">", "Gilligan"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });

        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("the query should not be created");
    }

    #[test]
    fn test_valid_query_groupable_meeting_clauses() {
        let query_value = json!({
            "where": [
                ["firstName", "<=", "Gilligan"],
                ["firstName", ">", "Gilligan"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });

        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("the query should be created");
    }

    #[test]
    fn test_valid_query_query_field_at_max_length() {
        let long_string = "t".repeat(255);
        let query_value = json!({
            "where": [
                ["firstName", "<", long_string],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });
        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("query should be fine for a 255 byte long string");
    }

    #[test]
    fn test_valid_query_drive_document_query() {
        let platform_version = PlatformVersion::latest();
        let mut rng = StdRng::seed_from_u64(5);
        let contract =
            get_dpns_data_contract_fixture(Some(Identifier::random_with_rng(&mut rng)), 0, 1)
                .data_contract_owned();
        let domain = contract
            .document_type_for_name("domain")
            .expect("expected to get domain");

        let query_asc = DriveDocumentQuery {
            contract: &contract,
            document_type: domain,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: Some(WhereClause {
                    field: "records.identity".to_string(),
                    operator: WhereOperator::LessThan,
                    value: Value::Identifier(
                        Identifier::from_string(
                            "AYN4srupPWDrp833iG5qtmaAsbapNvaV7svAdncLN5Rh",
                            Encoding::Base58,
                        )
                        .unwrap()
                        .to_buffer(),
                    ),
                }),
                equal_clauses: BTreeMap::new(),
            },
            offset: None,
            limit: Some(6),
            order_by: vec![(
                "records.identity".to_string(),
                OrderClause {
                    field: "records.identity".to_string(),
                    ascending: false,
                },
            )]
            .into_iter()
            .collect(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        let path_query = query_asc
            .construct_path_query(None, platform_version)
            .expect("expected to create path query");

        assert_eq!(path_query.to_string(), "PathQuery { path: [@, 0x1da29f488023e306ff9a680bc9837153fb0778c8ee9c934a87dc0de1d69abd3c, 0x01, domain, 0x7265636f7264732e6964656e74697479], query: SizedQuery { query: Query {\n  items: [\n    RangeTo(.. 8dc201fd7ad7905f8a84d66218e2b387daea7fe4739ae0e21e8c3ee755e6a2c0),\n  ],\n  default_subquery_branch: SubqueryBranch { subquery_path: [00], subquery: Query {\n  items: [\n    RangeFull,\n  ],\n  default_subquery_branch: SubqueryBranch { subquery_path: None subquery: None },\n  left_to_right: false,\n} },\n  conditional_subquery_branches: {\n    Key(): SubqueryBranch { subquery_path: [00], subquery: Query {\n  items: [\n    RangeFull,\n  ],\n  default_subquery_branch: SubqueryBranch { subquery_path: None subquery: None },\n  left_to_right: false,\n} },\n  },\n  left_to_right: false,\n}, limit: 6 } }");

        // Serialize the PathQuery to a Vec<u8>
        let encoded = bincode::encode_to_vec(&path_query, bincode::config::standard())
            .expect("Failed to serialize PathQuery");

        // Convert the encoded bytes to a hex string
        let hex_string = hex::encode(encoded);

        assert_eq!(hex_string, "050140201da29f488023e306ff9a680bc9837153fb0778c8ee9c934a87dc0de1d69abd3c010106646f6d61696e107265636f7264732e6964656e746974790105208dc201fd7ad7905f8a84d66218e2b387daea7fe4739ae0e21e8c3ee755e6a2c0010101000101030000000001010000010101000101030000000000010600");
    }

    #[test]
    fn test_invalid_query_field_too_long() {
        let (drive, contract) = setup_family_contract();

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let too_long_string = "t".repeat(256);
        let query_value = json!({
            "where": [
                ["firstName", "<", too_long_string],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("fields of queries length must be under 256 bytes long");
        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("fields of queries length must be under 256 bytes long");
    }

    // TODO: Eventually we want to error with weird Null values
    // #[test]
    // fn test_invalid_query_scalar_field_with_null_value() {
    //     let (drive, contract) = setup_family_contract();
    //
    //     let document_type = contract
    //         .document_type("person")
    //         .expect("expected to get a document type");
    //
    //     let query_value = json!({
    //         "where": [
    //             ["age", "<", Null],
    //         ],
    //         "limit": 100,
    //         "orderBy": [
    //             ["age", "asc"],
    //         ],
    //     });
    //
    //     let where_cbor = serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");
    //     let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, document_type, &DriveConfig::default())
    //         .expect("The query itself should be valid for a null type");
    //     query
    //         .execute_no_proof(&drive, None, None)
    //         .expect_err("a Null value doesn't make sense for an integer");
    // }

    // TODO: Eventually we want to error with weird Null values
    //
    // #[test]
    // fn test_invalid_query_timestamp_field_with_null_value() {
    //     let (drive, contract) = setup_family_birthday_contract();
    //
    //     let document_type = contract
    //         .document_type("person")
    //         .expect("expected to get a document type");
    //
    //     let query_value = json!({
    //         "where": [
    //             ["birthday", "<", Null],
    //         ],
    //         "limit": 100,
    //         "orderBy": [
    //             ["birthday", "asc"],
    //         ],
    //     });
    //
    //     let where_cbor = serializer::value_to_cbor(query_value, None).expect("expected to serialize to cbor");
    //     let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, document_type, &DriveConfig::default())
    //         .expect("The query itself should be valid for a null type");
    //     query
    //         .execute_no_proof(&drive, None, None)
    //         .expect_err("the value can not be less than Null");
    // }

    #[test]
    fn test_valid_query_timestamp_field_with_null_value() {
        let (drive, contract) = setup_family_birthday_contract();

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let query_value = json!({
            "where": [
                ["birthday", ">=", Null],
            ],
            "limit": 100,
            "orderBy": [
                ["birthday", "asc"],
            ],
        });

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("The query itself should be valid for a null type");
        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("a Null value doesn't make sense for a float");
    }

    #[test]
    fn test_invalid_query_in_with_empty_array() {
        let (drive, contract) = setup_family_contract();

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let query_value = json!({
            "where": [
                ["firstName", "in", []],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("query should be valid for empty array");

        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("query should not be able to execute for empty array");
    }

    #[test]
    fn test_invalid_query_in_too_many_elements() {
        let (drive, contract) = setup_family_contract();

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let mut array: Vec<String> = Vec::with_capacity(101);
        for _ in 0..array.capacity() {
            array.push(String::from("a"));
        }
        let query_value = json!({
            "where": [
                ["firstName", "in", array],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("query is valid for too many elements");

        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("query should not be able to execute with too many elements");
    }

    #[test]
    fn test_invalid_query_in_unique_elements() {
        let (drive, contract) = setup_family_contract();

        let platform_version = PlatformVersion::latest();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let query_value = json!({
            "where": [
                ["firstName", "in", ["a", "a"]],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");

        // The is actually valid, however executing it is not
        // This is in order to optimize query execution

        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("the query should be created");

        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect_err("there should be no duplicates values for In query");
    }

    #[test]
    fn test_invalid_query_starts_with_empty_string() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", ""],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_invalid_query_limit_too_high() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", "a"],
            ],
            "limit": 101,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_invalid_query_limit_too_low() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", "a"],
            ],
            "limit": -1,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_invalid_query_limit_zero() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", "a"],
            ],
            "limit": 0,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = get_data_contract_fixture(None, 0, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_withdrawal_query_with_missing_transaction_index() {
        // Setup the withdrawal contract
        let (_, contract) = setup_withdrawal_contract();
        let platform_version = PlatformVersion::latest();

        let document_type_name = "withdrawal";
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected to get document type");

        // Create a DriveDocumentQuery that simulates missing 'transactionIndex' in documents
        let drive_document_query = DriveDocumentQuery {
            contract: &contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: Some(WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::U64(0),
                        Value::U64(1),
                        Value::U64(2),
                        Value::U64(3),
                        Value::U64(4),
                    ]),
                }),
                range_clause: None,
                equal_clauses: BTreeMap::default(),
            },
            offset: None,
            limit: Some(3),
            order_by: IndexMap::from([
                (
                    "status".to_string(),
                    OrderClause {
                        field: "status".to_string(),
                        ascending: true,
                    },
                ),
                (
                    "transactionIndex".to_string(),
                    OrderClause {
                        field: "transactionIndex".to_string(),
                        ascending: true,
                    },
                ),
            ]),
            start_at: Some([3u8; 32]),
            start_at_included: false,
            block_time_ms: None,
        };

        // Create a document that we are starting at, which may be missing 'transactionIndex'
        let mut properties = BTreeMap::new();
        properties.insert("status".to_string(), Value::U64(0));
        // We intentionally omit 'transactionIndex' to simulate missing field

        let starts_at_document = DocumentV0 {
            id: Identifier::from([3u8; 32]), // The same as start_at
            owner_id: Identifier::random(),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
        }
        .into();

        // Attempt to construct the path query
        let result = drive_document_query
            .construct_path_query(Some(starts_at_document), platform_version)
            .expect("expected to construct a path query");

        assert_eq!(
            result
                .clone()
                .query
                .query
                .default_subquery_branch
                .subquery
                .expect("expected subquery")
                .items,
            Query::new_range_full().items
        );
    }
}
