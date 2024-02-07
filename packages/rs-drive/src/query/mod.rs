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

#[cfg(any(feature = "full", feature = "verify"))]
use std::collections::BTreeMap;
#[cfg(any(feature = "full", feature = "verify"))]
use std::ops::BitXor;

#[cfg(feature = "full")]
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
/// Import grovedb
#[cfg(feature = "full")]
pub use grovedb::{Element, Error as GroveError, GroveDb, TransactionArg};
#[cfg(any(feature = "full", feature = "verify"))]
pub use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

#[cfg(any(feature = "full", feature = "verify"))]
use indexmap::IndexMap;

#[cfg(any(feature = "full", feature = "verify"))]
use sqlparser::ast;
#[cfg(any(feature = "full", feature = "verify"))]
use sqlparser::ast::TableFactor::Table;
#[cfg(any(feature = "full", feature = "verify"))]
use sqlparser::ast::Value::Number;
#[cfg(any(feature = "full", feature = "verify"))]
use sqlparser::ast::{OrderByExpr, Select, Statement};
#[cfg(any(feature = "full", feature = "verify"))]
use sqlparser::dialect::MySqlDialect;
#[cfg(any(feature = "full", feature = "verify"))]
use sqlparser::parser::Parser;

#[cfg(any(feature = "full", feature = "verify"))]
pub use conditions::WhereClause;
/// Import conditions
#[cfg(any(feature = "full", feature = "verify"))]
pub use conditions::WhereOperator;
#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

#[cfg(any(feature = "full", feature = "verify"))]
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::data_contract::document_type::{Index, IndexProperty};
#[cfg(any(feature = "full", feature = "verify"))]

/// Import ordering
#[cfg(any(feature = "full", feature = "verify"))]
pub use ordering::OrderClause;

#[cfg(feature = "full")]
#[cfg(feature = "full")]
use crate::drive::grove_operations::QueryType::StatefulQuery;
#[cfg(feature = "full")]
use crate::drive::Drive;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::error::drive::DriveError;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::error::query::QuerySyntaxError;
#[cfg(any(feature = "full", feature = "verify"))]
use crate::error::Error;
#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::data_contract::DataContract;

#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::contract::paths::DataContractPaths;

use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::document::Document;

#[cfg(any(feature = "full", feature = "verify"))]
use dpp::platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use dpp::platform_value::platform_value;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::platform_value::Value;

use crate::common::encode::encode_u64;
use crate::drive::config::DriveConfig;
use crate::error::Error::GroveDB;

use dpp::version::PlatformVersion;
#[cfg(any(feature = "full", feature = "verify"))]
use dpp::ProtocolError;

#[cfg(any(feature = "full", feature = "verify"))]
pub mod conditions;
#[cfg(any(feature = "full", feature = "verify"))]
mod defaults;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod ordering;
#[cfg(any(feature = "full", feature = "verify"))]
mod single_document_drive_query;
#[cfg(feature = "full")]
mod test_index;

#[cfg(any(feature = "full", feature = "verify"))]
pub use single_document_drive_query::SingleDocumentDriveQuery;

use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::DocumentV0Getters;

#[cfg(any(feature = "full", feature = "verify"))]
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
    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Returns true if the query clause is for primary keys.
    pub fn is_for_primary_key(&self) -> bool {
        self.primary_key_in_clause.is_some() || self.primary_key_equal_clause.is_some()
    }

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Returns true if self is empty.
    pub fn is_empty(&self) -> bool {
        self.in_clause.is_none()
            && self.range_clause.is_none()
            && self.equal_clauses.is_empty()
            && self.primary_key_in_clause.is_none()
            && self.primary_key_equal_clause.is_none()
    }

    #[cfg(any(feature = "full", feature = "verify"))]
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
                    .get(0)
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
                    .get(0)
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

#[cfg(any(feature = "full", feature = "verify"))]
/// The encoding returned by queries
#[derive(Debug, PartialEq)]
pub enum QueryResultEncoding {
    /// Cbor encoding
    CborEncodedQueryResult,
    /// Platform base encoding
    PlatformEncodedQueryResult,
}

#[cfg(any(feature = "full", feature = "verify"))]
impl QueryResultEncoding {
    /// Encode the value based on the encoding desired
    pub fn encode_value(&self, value: &Value) -> Result<Vec<u8>, Error> {
        let mut buffer = vec![];
        match self {
            QueryResultEncoding::CborEncodedQueryResult => {
                ciborium::ser::into_writer(value, &mut buffer)
                    .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
            }
            QueryResultEncoding::PlatformEncodedQueryResult => {}
        }
        Ok(buffer)
    }
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct DriveQuery<'a> {
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
    /// Start at
    pub start_at: Option<[u8; 32]>,
    /// Start at included
    pub start_at_included: bool,
    /// Block time
    pub block_time_ms: Option<u64>,
}

// TODO: expose this also
//  also figure out main export
impl<'a> DriveQuery<'a> {
    #[cfg(feature = "full")]
    /// Returns any item
    pub fn any_item_query(contract: &'a DataContract, document_type: DocumentTypeRef<'a>) -> Self {
        DriveQuery {
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

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(any(feature = "full", feature = "verify"))]
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

        Ok(DriveQuery {
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

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Converts a query Value to a `DriveQuery`.
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

        Ok(DriveQuery {
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

    #[cfg(any(feature = "full", feature = "verify"))]
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
                .get(0)
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
            .get(0)
            .ok_or(Error::Query(QuerySyntaxError::InvalidSQL(
                "Invalid query: missing from section".to_string(),
            )))?
            .relation
        {
            Table { name, .. } => name.0.get(0).as_ref().map(|identifier| &identifier.value),
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

        Ok(DriveQuery {
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
    /// FIXME: The data contract is only refered as ID, and document type as its name.
    /// This can change in the future to include full data contract and document type.
    #[cfg(any(feature = "full", feature = "verify"))]
    pub fn to_cbor(&self) -> Result<Vec<u8>, Error> {
        let data: BTreeMap<String, Value> = self.into();
        let cbor: BTreeMap<String, ciborium::Value> = Value::convert_to_cbor_map(data)?;
        let mut output = Vec::new();

        ciborium::ser::into_writer(&cbor, &mut output)
            .map_err(|e| ProtocolError::PlatformSerializationError(e.to_string()))?;
        Ok(output)
    }

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(feature = "full")]
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
                // First if we have a startAt or or startsAfter we must get the element
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
            let mut merged =
                PathQuery::merge(vec![&start_at_path_query, &main_path_query]).map_err(GroveDB)?;
            merged.query.limit = limit.map(|a| a.saturating_add(1));
            Ok(merged)
        } else {
            Ok(main_path_query)
        }
    }

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(any(feature = "full", feature = "verify"))]
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

    #[cfg(any(feature = "full", feature = "verify"))]
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
                QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "query must be for valid indexes",
                ),
            ))?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Err(Error::Query(QuerySyntaxError::QueryTooFarFromIndex(
                "query must better match an existing index",
            )));
        }
        Ok(index)
    }

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Returns a `QueryItem` given a start key and query direction.
    pub fn query_item_for_starts_at_key(starts_at_key: Vec<u8>, left_to_right: bool) -> QueryItem {
        if left_to_right {
            QueryItem::RangeAfter(starts_at_key..)
        } else {
            QueryItem::RangeTo(..starts_at_key)
        }
    }

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given document ID if given.
    fn inner_query_from_starts_at_for_id(
        starts_at_document: &Option<(Document, DocumentTypeRef, &IndexProperty, bool)>,
        left_to_right: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);

        if let Some((document, _, _, included)) = starts_at_document {
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

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given key.
    fn inner_query_starts_from_key(
        start_at_key: Vec<u8>,
        left_to_right: bool,
        included: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);
        if left_to_right {
            if included {
                inner_query.insert_range_from(start_at_key..);
            } else {
                inner_query.insert_range_after(start_at_key..);
            }
        } else if included {
            inner_query.insert_range_to_inclusive(..=start_at_key);
        } else {
            inner_query.insert_range_to(..start_at_key);
        }
        inner_query
    }

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given document if given.
    // We are passing in starts_at_document 4 parameters
    // The document
    // The document type (borrowed)
    // The index property (borrowed)
    // if the element itself should be included. ie StartAt vs StartAfter
    fn inner_query_from_starts_at(
        starts_at_document: &Option<(Document, DocumentTypeRef, &IndexProperty, bool)>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Query, Error> {
        let mut inner_query = Query::new_with_direction(left_to_right);
        if let Some((document, document_type, indexed_property, included)) = starts_at_document {
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

    #[cfg(any(feature = "full", feature = "verify"))]
    /// Recursively queries as long as there are leftover index properties.
    fn recursive_insert_on_query(
        query: Option<&mut Query>,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: &Option<(Document, DocumentTypeRef, &IndexProperty, bool)>, //for key level, included
        default_left_to_right: bool,
        order_by: Option<&IndexMap<String, OrderClause>>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Query>, Error> {
        match left_over_index_properties.split_first() {
            None => {
                if let Some(query) = query {
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
                            let full_query = Self::inner_query_from_starts_at_for_id(
                                &None,
                                default_left_to_right,
                            );
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

                match query {
                    None => {
                        let mut inner_query = Self::inner_query_from_starts_at(
                            starts_at_document,
                            left_to_right,
                            platform_version,
                        )?;
                        DriveQuery::recursive_insert_on_query(
                            Some(&mut inner_query),
                            left_over,
                            unique,
                            starts_at_document,
                            left_to_right,
                            order_by,
                            platform_version,
                        )?;
                        Ok(Some(inner_query))
                    }
                    Some(query) => {
                        if let Some((document, document_type, _indexed_property, included)) =
                            starts_at_document
                        {
                            let start_at_key = document
                                .get_raw_for_document_type(
                                    first.name.as_str(),
                                    *document_type,
                                    None,
                                    platform_version,
                                )
                                .ok()
                                .flatten()
                                .unwrap_or_default();

                            // We should always include if we have left_over
                            let non_conditional_included = !left_over.is_empty() | *included;

                            let mut non_conditional_query = Self::inner_query_starts_from_key(
                                start_at_key,
                                left_to_right,
                                non_conditional_included,
                            );

                            DriveQuery::recursive_insert_on_query(
                                Some(&mut non_conditional_query),
                                left_over,
                                unique,
                                starts_at_document,
                                left_to_right,
                                order_by,
                                platform_version,
                            )?;

                            query.set_subquery(non_conditional_query);
                        } else {
                            let mut inner_query = Query::new_with_direction(first.ascending);
                            inner_query.insert_all();
                            DriveQuery::recursive_insert_on_query(
                                Some(&mut inner_query),
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
        }
    }

    #[cfg(any(feature = "full", feature = "verify"))]
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
                    .get(field.name.as_str())
                    .is_some()
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
                    DriveError::CorruptedContractIndexes("index must have properties"),
                ))?; // Index must have properties
                Self::recursive_insert_on_query(
                    None,
                    left_over_index_properties.as_slice(),
                    index.unique,
                    &starts_at_document.map(|(document, included)| {
                        (document, self.document_type, first_index, included)
                    }),
                    first_index.ascending,
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
                        // There is a last_clause, but no subquery_clause, we should use the index property of the last clause
                        // We need to get the terminal indexes unused by clauses.
                        let last_index_property = index
                            .properties
                            .iter()
                            .find(|field| where_clause.field == field.name)
                            .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(
                                "index must have last_clause field",
                            )))?;
                        Self::recursive_insert_on_query(
                            Some(&mut query),
                            left_over_index_properties.as_slice(),
                            index.unique,
                            &starts_at_document.map(|(document, included)| {
                                (document, self.document_type, last_index_property, included)
                            }),
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
                        let last_index_property = index
                            .properties
                            .iter()
                            .find(|field| subquery_where_clause.field == field.name)
                            .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(
                                "index must have subquery_clause field",
                            )))?;
                        Self::recursive_insert_on_query(
                            Some(&mut subquery),
                            left_over_index_properties.as_slice(),
                            index.unique,
                            &starts_at_document.map(|(document, included)| {
                                (document, self.document_type, last_index_property, included)
                            }),
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

    #[cfg(feature = "full")]
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
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, cost))
    }

    #[cfg(feature = "full")]
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
            false,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }

    #[cfg(feature = "full")]
    /// Executes a query with proof and returns the root hash, items, and fee.
    pub fn execute_with_proof_only_get_elements(
        self,
        drive: &Drive,
        block_info: Option<BlockInfo>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<([u8; 32], Vec<Vec<u8>>, u64), Error> {
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
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((root_hash, items, cost))
    }

    #[cfg(feature = "full")]
    /// Executes an internal query with proof and returns the root hash and values.
    pub(crate) fn execute_with_proof_only_get_elements_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<([u8; 32], Vec<Vec<u8>>), Error> {
        let path_query = self.construct_path_query_operations(
            drive,
            true,
            transaction,
            drive_operations,
            platform_version,
        )?;

        let proof = drive.grove_get_proved_path_query(
            &path_query,
            self.start_at.is_some(),
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
        self.verify_proof_keep_serialized(proof.as_slice(), platform_version)
    }

    #[cfg(feature = "full")]
    /// Executes a query with no proof and returns the items encoded in a map.
    pub fn execute_serialized_as_result_no_proof(
        &self,
        drive: &Drive,
        _block_info: Option<BlockInfo>,
        query_result_encoding: QueryResultEncoding,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations = vec![];
        let (items, _) = self.execute_no_proof_internal(
            drive,
            QueryResultType::QueryKeyElementPairResultType,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        //todo: we could probably give better results depending on the query
        let result = platform_value!({
            "documents": items.to_key_elements()
        });
        query_result_encoding.encode_value(&result)
    }

    #[cfg(feature = "full")]
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
            )?;
            fee_result.processing_fee
        } else {
            0
        };
        Ok((items, skipped, cost))
    }

    #[cfg(feature = "full")]
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

    #[cfg(feature = "full")]
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
impl<'a> From<&DriveQuery<'a>> for BTreeMap<String, Value> {
    fn from(query: &DriveQuery<'a>) -> Self {
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

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {

    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    use dpp::prelude::Identifier;
    use serde_json::json;
    use std::borrow::Cow;
    use std::option::Option::None;
    use tempfile::TempDir;

    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;
    use crate::query::DriveQuery;

    use dpp::data_contract::DataContract;

    use serde_json::Value::Null;

    use crate::drive::config::DriveConfig;
    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::util::cbor_serializer;
    use dpp::version::PlatformVersion;

    fn setup_family_contract() -> (Drive, DataContract) {
        let tmp_dir = TempDir::new().unwrap();

        let (drive, _) = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let platform_version = PlatformVersion::latest();

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

    fn setup_family_birthday_contract() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure();

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
        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
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
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, document_type, &config)
            .expect("deserialize cbor shouldn't fail");

        let cbor = query.to_cbor().expect("should serialize cbor");

        let deserialized = DriveQuery::from_cbor(&cbor, &contract, document_type, &config)
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
        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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
        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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

        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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

        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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
        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect("query should be fine for a 255 byte long string");
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
        let query = DriveQuery::from_cbor(
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
        let query = DriveQuery::from_cbor(
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
        let query = DriveQuery::from_cbor(
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
        let query = DriveQuery::from_cbor(
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

        let query = DriveQuery::from_cbor(
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

        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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

        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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

        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
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

        let contract = get_data_contract_fixture(None, 1).data_contract_owned();
        let document_type = contract
            .document_type_for_name("niceDocument")
            .expect("expected to get nice document");

        let where_cbor = cbor_serializer::serializable_value_to_cbor(&query_value, None)
            .expect("expected to serialize to cbor");
        DriveQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &DriveConfig::default(),
        )
        .expect_err("starts with can not start with an empty string");
    }
}
