#[cfg(feature = "server")]
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::document_type::{
    IndexBucketing, IntegerRangeTransform, TimeRangeTransform,
};
use std::sync::Arc;

#[cfg(any(feature = "server", feature = "verify"))]
pub use {
    // Chained-query building blocks: the result shape and the join-value
    // cap. The join itself is a by-id join sub-query in
    // [`DriveDocumentQuery::sub_queries`].
    chained_document_query::{
        ChainedDocumentsResult, ChainedOuterDocuments, MAX_CHAINED_JOIN_VALUES,
    },
    // Composite-query building blocks: the sub-query shapes carried by
    // [`DriveDocumentQuery::sub_queries`] and the assembled result. The
    // verifier needs them all to rebuild and route the merged proof.
    composite_document_query::{
        BindingSource, CompositeDocumentsResult, DriveSubQuery, SubQueryBinding, SubQueryKind,
        SubQueryResult, MAX_BOUND_VALUES, MAX_SUB_QUERIES,
    },
    conditions::{ValueClause, WhereClause, WhereOperator},
    // Average-query verifier-shareable types — same split as sum:
    // `AverageEntry` is the per-key `(count, sum)` pair the verifier
    // returns; `AverageMode` is the SQL-shape input the verifier needs
    // to rebuild the path query.
    drive_document_average_query::{AverageEntry, AverageMode},
    // `CountMode` is the SQL-shape contract (Aggregate /
    // GroupByIn / GroupByRange / GroupByCompound) the prover
    // dispatches on; the verifier needs the same enum to route
    // proof verification to the matching primitive
    // (`DocumentCountMode`). Available under either `server`
    // (executor input) or `verify` (proof-decode input).
    drive_document_count_query::{
        CountMode, DocumentCountMode, DriveDocumentCountQuery, SplitCountEntry,
    },
    // Having-range verifier-shareable types — same split as ranked:
    // `DocumentHavingMode` + `AxisRangeBounds` to re-run the same
    // versioned request validation (and bounds translation) the prover
    // ran, `DriveDocumentHavingQuery` to rebuild the proved grove path
    // and secondary query. Entries reuse the ranked `RankedEntry` shape.
    drive_document_having_query::{
        AxisRangeBounds, DocumentHavingMode, DriveDocumentHavingQuery, MAX_HAVING_LIMIT,
    },
    // Ranked-query verifier-shareable types. The verifier needs the
    // whole set: `DocumentRankedMode` + `RankedPaginationInputs` to
    // re-run the same versioned request validation the prover ran,
    // `DriveDocumentRankedQuery` to rebuild the proved grove path, and
    // `RankedEntry` / `RankedEntryValue` as the verified result shape.
    drive_document_ranked_query::{
        DocumentRankedMode, DriveDocumentRankedQuery, RankedAxis, RankedEntry, RankedEntryValue,
        RankedPage, RankedPaginationInputs, MAX_RANKED_LIMIT, RANKED_AVG_SCALE,
        RANKED_COUNT_ORDER_KEY,
    },
    // Sum-query verifier-shareable types: `SumEntry` is the per-key
    // entry type the verifier returns, `SumMode` / `DriveDocumentSumQuery`
    // are shape inputs the verifier needs to rebuild the path query.
    // Parallels the count-side exports above.
    drive_document_sum_query::{DriveDocumentSumQuery, SumEntry, SumMode},
    grovedb::{PathQuery, Query, QueryItem, SizedQuery},
    having::{
        HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    },
    ordering::OrderClause,
    projection::{SelectFunction, SelectProjection},
    single_document_drive_query::SingleDocumentDriveQuery,
    single_document_drive_query::SingleDocumentDriveQueryContestedStatus,
    vote_polls_by_end_date_query::VotePollsByEndDateDriveQuery,
    vote_query::IdentityBasedVoteDriveQuery,
};

// `DocumentCountRequest` / `RangeCountOptions` are the
// server-side executor inputs and stay `server`-only.
#[cfg(feature = "server")]
pub use drive_document_count_query::{
    DocumentCountRequest, DocumentCountResponse, RangeCountOptions, MAX_LIMIT_AS_FAILSAFE,
};

// `DocumentSumRequest` / `DocumentSumResponse` / range-sum options are
// the server-side executor inputs and stay `server`-only (parallels
// the count-side `DocumentCountRequest` etc. above).
#[cfg(feature = "server")]
pub use drive_document_sum_query::{
    DocumentSumRequest, DocumentSumResponse, RangeSumOptions, RangeSumWalkMode,
};

// `DocumentAverageRequest` / `DocumentAverageResponse` are the
// server-side executor inputs for the average surface and stay
// `server`-only (parallels the sum-side server-only exports above).
#[cfg(feature = "server")]
pub use drive_document_average_query::{DocumentAverageRequest, DocumentAverageResponse};

// `DocumentRankedRequest` / `DocumentRankedResponse` are the
// server-side dispatcher ABI for the ranked surface — the types
// drive-abci's routing layer names. Server-only for the same reason
// as the count / sum / average request types above.
#[cfg(feature = "server")]
pub use drive_document_ranked_query::{DocumentRankedRequest, DocumentRankedResponse};

// `DocumentHavingRequest` / `DocumentHavingResponse` are the
// server-side dispatcher ABI for the having-range surface — the types
// drive-abci's routing layer names. Server-only for the same reason as
// the ranked request types above.
#[cfg(feature = "server")]
pub use drive_document_having_query::{DocumentHavingRequest, DocumentHavingResponse};
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
            document_type::{DocumentTypeRef, Index},
            DataContract,
        },
        document::{document_methods::DocumentMethodsV0, Document},
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

#[cfg(all(feature = "server", feature = "verify"))]
use crate::verify::RootHash;

#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes_for_level;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
#[cfg(feature = "server")]
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::DocumentV0Getters;
#[cfg(feature = "server")]
pub use grovedb::{
    query_result_type::{QueryResultElements, QueryResultType},
    Element, Error as GroveError, TransactionArg,
};

use dpp::document;
use dpp::prelude::Identifier;
use dpp::validation::{SimpleValidationResult, ValidationResult};
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
#[cfg(feature = "server")]
use dpp::version::FeatureVersion;

// Module declarations that are conditional on either "server" or "verify" features
#[cfg(any(feature = "server", feature = "verify"))]
pub mod canonicalize;
#[cfg(any(feature = "server", feature = "verify"))]
pub use canonicalize::validate_and_canonicalize_where_clauses;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod conditions;
#[cfg(any(feature = "server", feature = "verify"))]
mod defaults;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod having;
mod non_primary_key_path_query;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod ordering;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod projection;
#[cfg(any(feature = "server", feature = "verify"))]
mod single_document_drive_query;
/// Versioned grouping of raw where clauses into equality / range / in buckets
pub(crate) mod where_clause_grouping;

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
pub type ContractLookupFn<'a> =
    dyn Fn(&Identifier) -> Result<Option<Arc<DataContract>>, Error> + 'a;

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
    let func = move |id: &Identifier| -> Result<Option<Arc<DataContract>>, Error> {
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

/// Document subscription filtering
#[cfg(any(feature = "server", feature = "verify"))]
pub mod filter;
/// A query to get the token's status
#[cfg(any(feature = "server", feature = "verify"))]
pub mod token_status_drive_query;

/// A query to count documents using CountTree elements
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive_document_count_query;

/// A query to sum an integer property across documents using SumTree
/// elements. Parallels [`drive_document_count_query`] for the sum
/// surface — see `book/src/drive/document-sum-trees.md` for the
/// design and `book/src/drive/sum-index-examples.md` for the worked
/// example contract.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive_document_sum_query;

/// A query to compute the average of an integer property across
/// documents using `CountSumTree` / `ProvableCountProvableSumTree`
/// (PCPS) elements. Averages are NOT computed server-side; the
/// response carries a `(count, sum)` pair (atomic per group) and the
/// client divides. See `book/src/drive/average-index-examples.md` for
/// the worked example contract.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive_document_average_query;

/// A query to filter an index's groups by a per-group aggregate bound —
/// "hashtags with more than 100 posts" — served as a value-bounded
/// range read of the same per-axis secondary Merk the ranked surface
/// walks (PR #657, PV14). Like ranked, it never opens the value trees,
/// so a having-range read is `O(log n + k)` with a proof.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive_document_having_query;

/// A query to rank an index's groups by a per-group aggregate — "top
/// 5 restaurants by average grade" — reading grovedb's per-axis
/// secondary Merk of an indexed tree (PR #657, PV14). Unlike the
/// count / sum / average surfaces this one never opens the value
/// trees: the ordering is maintained on write, so a ranked read is
/// `O(log n + k)` with a proof.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive_document_ranked_query;

/// Document synthesis for indexOnly queries: an indexOnly entry's proved
/// `(path, key)` position IS the document, and this module is the single
/// builder both the server's no-proof execution and the proof verifier
/// call to turn one back into a `Document`.
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) mod index_only_synthesis;

/// Chained document queries — a provable semi-join: an inner indexOnly
/// [`DriveDocumentQuery`] whose proven `refersTo` values become the outer
/// query's primary keys (carried as a single by-id join in
/// [`DriveDocumentQuery::sub_queries`]), proven against one state root.
/// See the module docs.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod chained_document_query;

/// Joins through a `refersTo: moderatedDocument` property: the removal
/// records a chained or composite join proves beside the documents it joins.
/// See the module docs.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod moderated_join;

/// Composite document queries — a [`DriveDocumentQuery`] page plus
/// sub-queries derived from its proven results (joins, lookups, counts),
/// proven as one merged proof against one state root. See the module docs.
#[cfg(any(feature = "server", feature = "verify"))]
pub mod composite_document_query;

/// Joint count-and-sum no-prove executor surface — backs the AVG
/// no-prove path's unified single-walk dispatch. See its module
/// docstring for the perf / atomicity contract. Server-only because
/// the surface only fires on the no-prove (server-materialized) path.
#[cfg(feature = "server")]
pub mod drive_document_count_and_sum_query;

/// A Query Syntax Validation Result that contains data
pub type QuerySyntaxValidationResult<TData> = ValidationResult<TData, QuerySyntaxError>;

/// A Query Syntax Validation Result
pub type QuerySyntaxSimpleValidationResult = SimpleValidationResult<QuerySyntaxError>;

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

/// Internal clauses struct
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InternalClauses {
    /// Primary key in clause
    pub primary_key_in_clause: Option<WhereClause>,
    /// Primary key equal clause
    pub primary_key_equal_clause: Option<WhereClause>,
    /// In clauses, on distinct non-primary-key fields.
    ///
    /// The grammar groups any number of them structurally; whether more
    /// than one is accepted is a protocol-versioned decision made at
    /// path-query lowering (protocol version 14 is the first to accept
    /// multiple in clauses, on consecutive index properties).
    pub in_clauses: Vec<WhereClause>,
    /// Range clause.
    ///
    /// On an indexOnly document type this may sit on an index's TERMINAL
    /// (member-key) property, not only on an index prefix property — see
    /// [`InternalClauses::classify_fields`] for the modeled roles instead
    /// of assuming property placement.
    pub range_clause: Option<WhereClause>,
    /// Equal clause
    pub equal_clauses: BTreeMap<String, WhereClause>,
}

/// How one where-clause (or order-by) field relates to a document type's
/// indexes — classified ONCE against the doctype instead of re-derived by
/// every consumer. Roles are not exclusive: on the yappr fixture `postId`
/// is a prefix property of `byHashtagPost`/`byPost` AND the terminal of
/// `byLiker`.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClauseFieldRoles {
    /// The field is `$id`.
    pub primary_key: bool,
    /// The field is a prefix property of at least one index.
    pub index_property: bool,
    /// The field is the terminal (member-key property) of at least one
    /// index — only ever true on indexOnly document types.
    pub terminal: bool,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl ClauseFieldRoles {
    /// The field appears in no index at all (and is not the primary key)
    /// — a clause on it can never be served.
    pub fn unindexed(&self) -> bool {
        !self.primary_key && !self.index_property && !self.terminal
    }
}

/// The outcome of generic index selection
/// ([`DriveDocumentQuery::select_best_index`]): a match, or the fact that
/// no index serves the query — carried as a value, not an error, so a
/// route that may legitimately stand in for a miss (the indexOnly
/// terminal route) never has to reconstruct that fact from error
/// variants. Structural failures never appear here; they stay `Err`.
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) enum BestIndexOutcome<'a> {
    /// An index serves the query.
    Matched(&'a Index),
    /// No index matches; carries the error [`DriveDocumentQuery::find_best_index`]
    /// reports for this query.
    NoIndexMatches(Error),
}

impl InternalClauses {
    /// Every constraint these clauses and `order_by_keys` make, for the
    /// `skipIfAbsent` admissibility gate
    /// ([`index_admissible_for_skip_if_absent`]).
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn skip_if_absent_bindings<'a>(
        &'a self,
        order_by_keys: &[&'a str],
    ) -> Vec<SkipIfAbsentBinding<'a>> {
        self.equal_clauses
            .values()
            .chain(self.in_clauses.iter())
            .chain(self.range_clause.iter())
            .map(SkipIfAbsentBinding::for_where_clause)
            .chain(
                order_by_keys
                    .iter()
                    .map(|key| SkipIfAbsentBinding::ordering(key)),
            )
            .collect()
    }

    /// Classify one field's index roles against `document_type`. The
    /// single derivation site for "is this a prefix property, a terminal,
    /// or `$id`" — consumers must branch on this instead of assuming a
    /// clause sits on an index prefix property (on indexOnly types it may
    /// sit on a terminal).
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn classify_field(document_type: DocumentTypeRef, field: &str) -> ClauseFieldRoles {
        let mut roles = ClauseFieldRoles {
            primary_key: field == "$id",
            ..Default::default()
        };
        for index in document_type.indexes().values() {
            if index
                .properties
                .iter()
                .any(|property| property.name == field)
            {
                roles.index_property = true;
            }
            if index.terminal_contains(field) {
                roles.terminal = true;
            }
            if roles.index_property && roles.terminal {
                break;
            }
        }
        roles
    }

    /// [`Self::classify_field`] over every field these clauses name —
    /// classification happens once, at the seam between clause extraction
    /// and routing, instead of being re-derived downstream.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn classify_fields(
        &self,
        document_type: DocumentTypeRef,
    ) -> BTreeMap<String, ClauseFieldRoles> {
        let mut classified = BTreeMap::new();
        let mut add = |field: &str| {
            classified
                .entry(field.to_string())
                .or_insert_with(|| Self::classify_field(document_type, field));
        };
        if self.primary_key_equal_clause.is_some() || self.primary_key_in_clause.is_some() {
            add("$id");
        }
        for field in self.equal_clauses.keys() {
            add(field);
        }
        if let Some(range_clause) = &self.range_clause {
            add(&range_clause.field);
        }
        for in_clause in &self.in_clauses {
            add(&in_clause.field);
        }
        classified
    }

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
            !(!self.in_clauses.is_empty()
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
        self.in_clauses.is_empty()
            && self.range_clause.is_none()
            && self.equal_clauses.is_empty()
            && self.primary_key_in_clause.is_none()
            && self.primary_key_equal_clause.is_none()
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Extracts the `WhereClause`s and returns them as type `InternalClauses`.
    pub fn extract_from_clauses(
        all_where_clauses: Vec<WhereClause>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
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

        let (equal_clauses, range_clause, in_clauses) =
            WhereClause::group_clauses(&all_where_clauses, platform_version)?;

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
            in_clauses,
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

    /// Validate this collection of InternalClauses against the document schema
    #[cfg(any(feature = "server", feature = "verify"))]
    pub fn validate_against_schema(
        &self,
        document_type: DocumentTypeRef,
    ) -> QuerySyntaxSimpleValidationResult {
        // Basic composition
        if !self.verify() {
            return QuerySyntaxSimpleValidationResult::new_with_error(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "invalid composition of where clauses",
                ),
            );
        }

        // Validate in_clauses against schema
        for in_clause in &self.in_clauses {
            // Forbid $id in non-primary-key clauses
            if in_clause.field == "$id" {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "use primary_key_* clauses for $id",
                    ),
                );
            }
            let result = in_clause.validate_against_schema(document_type);
            if !result.is_valid() {
                return result;
            }
        }

        // Validate range_clause against schema
        if let Some(range_clause) = &self.range_clause {
            // Forbid $id in non-primary-key clauses
            if range_clause.field == "$id" {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "use primary_key_* clauses for $id",
                    ),
                );
            }
            let result = range_clause.validate_against_schema(document_type);
            if !result.is_valid() {
                return result;
            }
        }

        // Validate equal_clauses against schema
        for (field, eq_clause) in &self.equal_clauses {
            // Forbid $id in non-primary-key clauses
            if field.as_str() == "$id" {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "use primary_key_* clauses for $id",
                    ),
                );
            }
            let result = eq_clause.validate_against_schema(document_type);
            if !result.is_valid() {
                return result;
            }
        }

        // Validate primary key clauses typing
        if let Some(pk_eq) = &self.primary_key_equal_clause {
            if pk_eq.operator != WhereOperator::Equal
                || !matches!(pk_eq.value, Value::Identifier(_))
            {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "primary key equality must compare an identifier",
                    ),
                );
            }
        }
        if let Some(pk_in) = &self.primary_key_in_clause {
            if pk_in.operator != WhereOperator::In {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "primary key IN must use IN operator",
                    ),
                );
            }
            // enforce array shape and no duplicates/size
            let result = pk_in.in_values();
            if !result.is_valid() {
                return QuerySyntaxSimpleValidationResult::new_with_errors(result.errors);
            }
            if let Value::Array(arr) = &pk_in.value {
                if !arr.iter().all(|v| matches!(v, Value::Identifier(_))) {
                    return QuerySyntaxSimpleValidationResult::new_with_error(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "primary key IN must contain identifiers",
                        ),
                    );
                }
            } else {
                return QuerySyntaxSimpleValidationResult::new_with_error(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "primary key IN must contain an array of identifiers",
                    ),
                );
            }
        }

        QuerySyntaxSimpleValidationResult::default()
    }
}

impl From<InternalClauses> for Vec<WhereClause> {
    fn from(clauses: InternalClauses) -> Self {
        let mut result: Self = clauses.equal_clauses.into_values().collect();

        result.extend(clauses.in_clauses);
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

/// Which window of a `timeRange` grid an `IN_TIME_RANGE` selection resolves
/// to. Time-range queries are a v1-only feature; the v0 query surface is
/// unaffected.
///
/// The two relative selectors are resolved against an authoritative "now"
/// (block time on the server, the quorum-signed metadata time on the
/// verifier); [`Self::ByStart`] names a window absolutely, so its resolution
/// reads the query alone and needs no clock at all — which is what makes
/// historic windows addressable.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum TimeRangeSelector {
    /// The freshest started range (largest start ≤ now). Covers the latest
    /// partial slice (0..step of history).
    Newest,
    /// The oldest range still active at now. Covers a near-full trailing
    /// window of ~range of history. Best for "trending over the last window".
    Oldest,
    /// The range starting exactly at `start_ms` (a millisecond timestamp).
    /// Must lie on the grid — `phase + k * step`, the same values
    /// [`TimeRangeTransform::containing_buckets`] produces and the storage
    /// keys spell — or resolution rejects it (see
    /// [`TimeRangeTransform::is_bucket_start`]). A window with no documents,
    /// including one that has not started yet, is a provable empty answer
    /// rather than an error.
    ByStart {
        /// The selected window's start on the millisecond timeline.
        start_ms: u64,
    },
}

#[cfg(any(feature = "server", feature = "verify"))]
impl TimeRangeSelector {
    /// The selector's JSON spelling — the `selector` string of the wasm-sdk
    /// query surface. The single source of truth for the string form: the
    /// wasm-sdk JSON parser and every error message quote these spellings.
    /// (The gRPC wire does not use them: since the typed
    /// `TimeRangeSelection` operand, the selector rides as a proto enum.)
    ///
    /// [`Self::ByStart`] names its *kind* only — the `start_ms` payload
    /// rides in a separate JSON field, so [`Self::from_string`] cannot
    /// construct it and parsers of the full shape handle it themselves.
    pub fn as_str(&self) -> &'static str {
        match self {
            TimeRangeSelector::Newest => "newest",
            TimeRangeSelector::Oldest => "oldest",
            TimeRangeSelector::ByStart { .. } => "byStart",
        }
    }

    /// Parses the spelling of the payload-free selectors. Returns `None`
    /// for anything else — including `"byStart"`, whose `start_ms` payload
    /// a bare string cannot carry (see [`Self::as_str`]).
    pub fn from_string(value: &str) -> Option<Self> {
        match value {
            "newest" => Some(TimeRangeSelector::Newest),
            "oldest" => Some(TimeRangeSelector::Oldest),
            _ => None,
        }
    }
}

/// A concrete grid specification, matching a contract's `timeRange`
/// declaration verbatim (`range` / `step` / `phase`, in seconds).
///
/// The structured `IN_TIME_RANGE` operand carries one of these when the
/// queried field is bucketed by more than one grid: the bare selector
/// (`"newest"` / `"oldest"`) is unambiguous only while exactly one time-range
/// index exists on the field, so a multi-grid field requires the query to
/// name the grid it wants.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeRangeGridSpec {
    /// Window length in seconds, as the contract declares it.
    pub range_seconds: u64,
    /// Interval between window starts in seconds, as the contract declares it.
    pub step_seconds: u64,
    /// Grid alignment phase in seconds (0 when the contract omits `phase`).
    pub phase_seconds: u64,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl TimeRangeGridSpec {
    /// Whether this spec names exactly the given transform's grid.
    pub fn matches(&self, transform: &TimeRangeTransform) -> bool {
        self.range_seconds == transform.range_seconds
            && self.step_seconds == transform.step_seconds
            && self.phase_seconds == transform.phase_seconds
    }
}

/// Resolution provenance for one window selection — an `IN_TIME_RANGE` or
/// an `IN_INTEGER_RANGE` clause: the field the selection named and the
/// exact grid the resolution used. Recorded by the resolver's caller on the
/// query (see [`DriveDocumentQuery::resolved_time_ranges`]) and consumed by
/// the index pickers through [`index_admissible_for_resolved_time_range`],
/// which pins selection to the index carrying exactly this grid — a field
/// may be bucketed by several grids, so the field name alone no longer
/// identifies the index the resolution was computed against.
///
/// Named for the first kind of grid; an `integerRange` resolution rides the
/// same provenance, since both kinds store window starts under a
/// grid-qualified level and every admissibility rule is shared.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTimeRange {
    /// The grid the window start was computed from. The grid carries its own
    /// source field, so the provenance cannot name a field the grid does not
    /// bucket — [`Self::field`] reads it from here.
    pub transform: IndexBucketing,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl ResolvedTimeRange {
    /// The bucketed source field the resolved equality is on — always the
    /// grid's own source.
    pub fn field(&self) -> &str {
        self.transform.source()
    }

    /// The kind of selection the equality was resolved from, as refusals
    /// name it: `"time-range"` or `"integer-range"`.
    pub fn kind(&self) -> &'static str {
        match self.transform {
            IndexBucketing::Time(_) => "time-range",
            IndexBucketing::Integer(_) => "integer-range",
        }
    }

    /// The where operator that selects a window of this kind.
    pub fn operator(&self) -> &'static str {
        match self.transform {
            IndexBucketing::Time(_) => "IN_TIME_RANGE",
            IndexBucketing::Integer(_) => "IN_INTEGER_RANGE",
        }
    }
}

/// How a window selection names its kind of grid in refusals.
#[cfg(any(feature = "server", feature = "verify"))]
struct SelectionGridKind {
    /// `"time-range"` or `"integer-range"`.
    kind: &'static str,
    /// The where operator the selection rides.
    operator: &'static str,
    /// The unit note of the grid's parameters, e.g. `", in seconds,"`.
    units: &'static str,
}

/// The grid a window selection on `field` resolves against, shared by the
/// time and integer resolvers so they pick grids by the same rules: the
/// distinct grids of `field` (indexes sharing a grid share its storage
/// level too, so they dedupe), the named one when `spec` names one, else
/// the only one — a bare selection on a field with several grids is
/// ambiguous and refused.
#[cfg(any(feature = "server", feature = "verify"))]
fn select_selection_grid<'a, T>(
    document_type: &'a DocumentTypeRef<'_>,
    field: &str,
    grid_of: impl Fn(&'a Index) -> Option<&'a T>,
    spec: Option<(impl Fn(&T) -> bool, String)>,
    names: SelectionGridKind,
) -> Result<&'a T, Error>
where
    T: BucketSource + PartialEq,
{
    let mut grids: Vec<&'a T> = Vec::new();
    for index in document_type.indexes().values() {
        if let Some(transform) = grid_of(index).filter(|transform| transform.source() == field) {
            if !grids.contains(&transform) {
                grids.push(transform);
            }
        }
    }
    let SelectionGridKind {
        kind,
        operator,
        units,
    } = names;
    let Some(first) = grids.first().copied() else {
        return Err(Error::Query(
            QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
                "no {kind} index is defined on field \"{field}\""
            )),
        ));
    };
    match spec {
        Some((matches, described)) => grids
            .into_iter()
            .find(|transform| matches(transform))
            .ok_or(Error::Query(QuerySyntaxError::Unsupported(format!(
                "no {kind} index on \"{field}\" declares the grid {described}"
            )))),
        None if grids.len() > 1 => Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "field \"{field}\" is bucketed by {} different {kind} grids; the {operator} \
             selection must name one in its `grid` (range/step/phase{units} as the contract \
             declares them)",
            grids.len()
        )))),
        None => Ok(first),
    }
}

/// The bucketed source field of a grid, for [`select_selection_grid`].
#[cfg(any(feature = "server", feature = "verify"))]
trait BucketSource {
    fn source(&self) -> &str;
}

#[cfg(any(feature = "server", feature = "verify"))]
impl BucketSource for TimeRangeTransform {
    fn source(&self) -> &str {
        &self.source
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl BucketSource for IntegerRangeTransform {
    fn source(&self) -> &str {
        &self.source
    }
}

/// Resolves a time-range selection on `field` into a concrete equality
/// [`WhereClause`] on the bucketed source field, using the named grid's
/// `timeRange` transform and an authoritative `block_time_ms`.
///
/// For the relative selectors the server supplies `block_time_ms` from
/// current block time and the verifier re-derives it from the quorum-signed
/// response metadata `time_ms`, so both produce the identical concrete
/// equality query — the existing index/count proofs apply unchanged and the
/// engine never needs a dedicated time-range operator. A
/// [`TimeRangeSelector::ByStart`] selection reads its start from the query
/// itself (validated to lie on the grid) and consults `block_time_ms` only
/// to reject windows past a declared `ttl`'s horizon — a window that may be
/// mid-drainage must not serve a truncated answer, and since drainage only
/// touches expired buckets, every window this resolver admits is complete.
///
/// `grid` selects among several time-range indexes on the same field: `None`
/// is accepted only while exactly one grid buckets the field (the common
/// case); with two or more grids the caller must name one, and naming a grid
/// no index declares is an error either way.
///
/// What comes back is an ordinary equality clause, byte-identical to one a
/// client could have written by hand against a raw timestamp, plus the
/// [`ResolvedTimeRange`] provenance callers must record on the query (see
/// [`DriveDocumentQuery::resolved_time_ranges`]), which
/// [`DriveDocumentQuery::find_best_index`] and the aggregate index pickers
/// consume through [`index_admissible_for_resolved_time_range`] to pin
/// selection to the grid's index — and to keep raw queries off it.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn resolve_time_range_bucket_clause(
    field: &str,
    selector: TimeRangeSelector,
    grid: Option<TimeRangeGridSpec>,
    document_type: DocumentTypeRef,
    block_time_ms: u64,
) -> Result<(WhereClause, ResolvedTimeRange), Error> {
    let transform = select_selection_grid(
        &document_type,
        field,
        |index| index.time_range.as_ref(),
        grid.map(|spec| {
            (
                move |transform: &TimeRangeTransform| spec.matches(transform),
                format!(
                    "range={}s step={}s phase={}s",
                    spec.range_seconds, spec.step_seconds, spec.phase_seconds
                ),
            )
        }),
        SelectionGridKind {
            kind: "time-range",
            operator: "IN_TIME_RANGE",
            units: ", in seconds,",
        },
    )?;

    let bucket_start = match selector {
        TimeRangeSelector::Newest => transform.newest_active_start(block_time_ms),
        TimeRangeSelector::Oldest => transform.oldest_active_start(block_time_ms),
        // Absolute selection: the start comes from the query itself, so the
        // clock is consulted only for the TTL gate — prover and verifier
        // agree by construction (the verifier passes the signed time_ms).
        // Only grid membership is checked; an empty (or not-yet-started)
        // window is a provable empty answer, not an invalid question.
        TimeRangeSelector::ByStart { start_ms } => {
            if !transform.is_bucket_start(start_ms) {
                return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "byStart {} on \"{}\" is not a window start of the grid range={}s \
                     step={}s phase={}s: starts are phase + k*step on the millisecond \
                     timeline, and an off-grid start is rejected rather than snapped",
                    start_ms,
                    field,
                    transform.range_seconds,
                    transform.step_seconds,
                    transform.phase_seconds
                ))));
            }
            // TTL gate: an expired window may be mid-drainage, and a
            // partially drained window would serve a truncated answer that
            // looks authoritative. Drainage only ever touches expired
            // buckets (same `bucket_expired` predicate), so everything on
            // the queryable side of this gate is complete — and rejecting
            // the question is deterministic where "whatever the drain has
            // left" is not. The verifier resolves this clause with the
            // quorum-signed response `time_ms`, so a node cannot serve an
            // expired window's remnants past a verifying client.
            if transform.bucket_expired(start_ms, block_time_ms) {
                return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "byStart {} on \"{}\" is past the ttl horizon ({}s): expired windows \
                     drain lazily and may be mid-removal, so they are not queryable — \
                     entries under this index live at most `ttl` past their window's start",
                    start_ms,
                    field,
                    transform.ttl_seconds.unwrap_or_default()
                ))));
            }
            Some(start_ms)
        }
    }
    .ok_or(Error::Query(QuerySyntaxError::Unsupported(format!(
        "no time range on \"{}\" is active yet: the block time predates the grid's phase \
         anchor (only possible within the first step after the epoch)",
        field
    ))))?;

    Ok((
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value: Value::U64(bucket_start),
        },
        ResolvedTimeRange {
            transform: transform.clone().into(),
        },
    ))
}

/// A concrete integer grid, matching a contract's `integerRange`
/// declaration verbatim (`range` / `step` / `phase`).
///
/// An `IN_INTEGER_RANGE` selection carries one when the queried field is
/// bucketed by more than one grid; with a single grid it may be omitted.
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IntegerRangeGridSpec {
    /// Window length, as the contract declares it.
    pub range: u64,
    /// Interval between window starts, as the contract declares it.
    pub step: u64,
    /// Grid alignment (0 when the contract omits `phase`).
    pub phase: u64,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl IntegerRangeGridSpec {
    /// Whether this spec names exactly the given transform's grid.
    pub fn matches(&self, transform: &IntegerRangeTransform) -> bool {
        self.range == transform.range
            && self.step == transform.step
            && self.phase == transform.phase
    }
}

/// Resolves an `IN_INTEGER_RANGE` selection on `field` — the window of an
/// `integerRange` grid starting at `start` — into a concrete equality
/// [`WhereClause`] on the bucketed source field, plus the
/// [`ResolvedTimeRange`] provenance callers must record on the query (see
/// [`DriveDocumentQuery::resolved_time_ranges`]).
///
/// The integer counterpart of [`resolve_time_range_bucket_clause`]'s
/// `byStart`: the window is named absolutely, so the server and the proof
/// verifier resolve it from the query alone, with no clock. `start` must be
/// an integer that names a window of the grid (a grid start the property's
/// type can hold, or the type's minimum for the clamped bottom window — see
/// [`IntegerRangeTransform::is_window_start`]); anything else is rejected
/// rather than snapped. An empty window is a provable empty answer.
///
/// `grid` selects among several integer-range indexes on the same field:
/// `None` is accepted only while exactly one grid buckets the field.
///
/// The equality's value is the start as an integer; the query path
/// serializes it through the schema exactly like a raw value of the field,
/// which is how the walkers stored it.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn resolve_integer_range_bucket_clause(
    field: &str,
    start: &Value,
    grid: Option<IntegerRangeGridSpec>,
    document_type: DocumentTypeRef,
) -> Result<(WhereClause, ResolvedTimeRange), Error> {
    let transform = select_selection_grid(
        &document_type,
        field,
        |index| index.integer_range.as_ref(),
        grid.map(|spec| {
            (
                move |transform: &IntegerRangeTransform| spec.matches(transform),
                format!(
                    "range={} step={} phase={}",
                    spec.range, spec.step, spec.phase
                ),
            )
        }),
        SelectionGridKind {
            kind: "integer-range",
            operator: "IN_INTEGER_RANGE",
            units: "",
        },
    )?;

    let start = start.as_integer::<i128>().ok_or(Error::Query(
        QuerySyntaxError::InvalidWhereClauseComponents(
            "an IN_INTEGER_RANGE selection's start must be an integer",
        ),
    ))?;
    if !transform.is_window_start(start) {
        return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "{} on \"{}\" is not a window start of the grid range={} step={} phase={}: starts \
             are phase + k*step within the property's integer type (or the type's minimum, for \
             the clamped bottom window), and an off-grid start is rejected rather than snapped",
            start, field, transform.range, transform.step, transform.phase
        ))));
    }
    // `is_window_start` checked the key type holds the start, so it always
    // converts; the one conversion the uniqueness probe shares.
    let value = transform.key_type.value_of(start).ok_or(Error::Drive(
        DriveError::CorruptedCodeExecution("a window start is held by its key type"),
    ))?;

    Ok((
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value,
        },
        ResolvedTimeRange {
            transform: transform.clone().into(),
        },
    ))
}

/// Whether `index` may serve a query whose equality clauses on
/// `resolved_time_ranges` were produced by
/// [`resolve_time_range_bucket_clause`] or
/// [`resolve_integer_range_bucket_clause`].
///
/// A bucketed (time- or integer-range) index does not store the source
/// field's raw values: under its grid-qualified first level it stores window
/// *starts*, and one document is stored once per window that contains its
/// value. So a bucketed index, a
/// raw index and another grid's bucketed index are never interchangeable, and
/// every mismatch is silent — a validly-proven wrong answer rather than an
/// error:
///
/// - A raw query (`resolved_time_ranges` empty) that landed on a bucketed
///   index would compare a real timestamp against bucket starts and see
///   nothing (or, for range/IN shapes, walk overlapping buckets and count the
///   same document up to `overlap_factor` times).
/// - A resolved query that landed on a raw index would compare a bucket start
///   against real timestamps and see nothing.
/// - A resolved query that landed on a *different grid's* index would compare
///   one grid's bucket start against another grid's — every 6-hour start is
///   also a 3-hour start, so this can silently return the wrong window.
///
/// Hence the rule: with no resolution only non-bucketed indexes are
/// admissible, and with one resolution only an index bucketing exactly that
/// field *with exactly that grid* is. Two resolutions can never be served by
/// a single index — a transform's source must be its index's first property,
/// so one index buckets exactly one field — and are rejected by the caller.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn index_admissible_for_resolved_time_range(
    index: &Index,
    resolved_time_ranges: &[ResolvedTimeRange],
) -> bool {
    match resolved_time_ranges {
        [] => !index.is_bucketed(),
        // The provenance's grid must equal the candidate's — kind, grid AND
        // source field, since the grid carries its own source. The
        // provenance cannot name a field its grid does not bucket
        // ([`ResolvedTimeRange::field`] is derived from the grid), so a
        // fabricated field/grid pair is unrepresentable rather than guarded
        // against.
        [resolved] => index.is_bucketed_by(&resolved.transform),
        _ => false,
    }
}

/// How a query constrains one field, as the `skipIfAbsent` gate reads it
/// ([`index_admissible_for_skip_if_absent`]).
#[cfg(any(feature = "server", feature = "verify"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkipIfAbsentBinding<'a> {
    /// The field.
    pub field: &'a str,
    /// Whether the constraint can never match a document missing the field.
    /// On a stored type a missing value is indexed under the empty key,
    /// which sorts first, so an ordering, a range with no lower bound and a
    /// null equality all reach it on an index that does not skip.
    pub excludes_missing: bool,
}

#[cfg(any(feature = "server", feature = "verify"))]
impl<'a> SkipIfAbsentBinding<'a> {
    /// An order-by on `field`: it reaches documents missing the field on an
    /// index that does not skip them.
    pub fn ordering(field: &'a str) -> Self {
        Self {
            field,
            excludes_missing: false,
        }
    }

    /// The binding a where clause makes: equality and `in` exclude missing
    /// documents unless they name a value that may encode as missing, a
    /// range excludes them when its lower bound may not (strict bounds
    /// exclude the empty key outright), and `startsWith` when its prefix is
    /// not empty.
    pub fn for_where_clause(clause: &'a WhereClause) -> Self {
        let lower_bound_excludes_missing = |value: &Value| {
            value
                .as_array()
                .and_then(|bounds| bounds.first())
                .is_some_and(|lower| !may_encode_as_missing(lower))
        };
        let excludes_missing = match clause.operator {
            WhereOperator::Equal | WhereOperator::GreaterThanOrEquals => {
                !may_encode_as_missing(&clause.value)
            }
            // `in_values` reads every spelling the query layer accepts,
            // bytes on a U8 property included.
            WhereOperator::In => clause
                .in_values()
                .data
                .is_some_and(|values| values.iter().all(|value| !may_encode_as_missing(value))),
            WhereOperator::GreaterThan
            | WhereOperator::BetweenExcludeBounds
            | WhereOperator::BetweenExcludeLeft => true,
            WhereOperator::Between | WhereOperator::BetweenExcludeRight => {
                lower_bound_excludes_missing(&clause.value)
            }
            WhereOperator::StartsWith => clause
                .value
                .as_text()
                .is_some_and(|prefix| !prefix.is_empty()),
            WhereOperator::LessThan | WhereOperator::LessThanOrEquals => false,
        };
        Self {
            field: clause.field.as_str(),
            excludes_missing,
        }
    }

    /// The bindings of `where_clauses`.
    pub fn for_where_clauses(where_clauses: &'a [WhereClause]) -> Vec<Self> {
        where_clauses.iter().map(Self::for_where_clause).collect()
    }
}

/// Whether `value` may encode to the empty key a missing value takes on a
/// stored index: null, or an empty byte array in any spelling a byteArray
/// property accepts (bytes, an array, base64 text). A string's `""` encodes
/// apart, but a binding does not know the property's type, so it is read
/// the cautious way.
#[cfg(any(feature = "server", feature = "verify"))]
fn may_encode_as_missing(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bytes(bytes) => bytes.is_empty(),
        Value::Array(values) => values.is_empty(),
        Value::Text(text) => text.is_empty(),
        _ => false,
    }
}

/// Whether a query constraining fields as `bindings` may be served by
/// `index` given its `skipIfAbsent` participation.
///
/// A `skipIfAbsent` index holds only the documents that carry every property
/// of its skip set — it is a SPARSE projection of the document type. An index
/// picker must not route a query to it that never binds those properties:
/// the generic matcher does not require contiguously bound prefixes (an
/// unused property merely counts toward the difference score), and the
/// aggregate pickers match a prefix that may stop above a deep skip
/// property. Without this gate such a query would silently omit every
/// document the index skipped — a result a complete index would have
/// included. So every skip property must be bound:
///
/// - On an indexOnly type (whose indexes carry a `terminal`) any binding
///   counts, the order-by included: a missing value has no index
///   representation anywhere on such a type, so binding the property is
///   asking "among documents carrying it", which is exactly what the index
///   holds.
/// - On a stored type a missing value is indexed under the empty key by
///   every index that does not skip it, so a skip property counts as bound
///   only by a constraint that no missing value can meet
///   ([`SkipIfAbsentBinding::excludes_missing`]).
///
/// For every index a contract could declare before skip properties could sit
/// below the first position, the skip set is the first property, which the
/// contiguous and exact matchers already bind; the gate then changes no
/// choice.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn index_admissible_for_skip_if_absent(
    index: &Index,
    bindings: &[SkipIfAbsentBinding<'_>],
) -> bool {
    if !index.skip_if_absent {
        return true;
    }
    // Every protocol version reaches this; `is_index_only`'s counter term is
    // inert before protocol version 14, the only one whose grammar admits the
    // keyword.
    let index_only = index.is_index_only();
    index.skip_if_absent_properties.iter().all(|skip_property| {
        bindings.iter().any(|binding| {
            binding.field == skip_property.as_str() && (index_only || binding.excludes_missing)
        })
    })
}

/// Whether `index` may serve a query at all: the one candidate filter every
/// index picker applies, so no picker can take one rule and miss the other.
/// It joins the time-range provenance rule
/// ([`index_admissible_for_resolved_time_range`]) and the `skipIfAbsent`
/// rule ([`index_admissible_for_skip_if_absent`]). A picker reading documents
/// through the index's entries applies
/// [`document_index_admissible_for_query`] instead.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn index_admissible_for_query(
    index: &Index,
    resolved_time_ranges: &[ResolvedTimeRange],
    skip_bindings: &[SkipIfAbsentBinding<'_>],
) -> bool {
    index_admissible_for_resolved_time_range(index, resolved_time_ranges)
        && index_admissible_for_skip_if_absent(index, skip_bindings)
}

/// [`index_admissible_for_query`] for a picker reading documents through the
/// index's entries: a `summableOffCountIndex` index keeps none (its source
/// serves the documents it counts), so it never serves such a read.
///
/// Unversioned, so every protocol version reaches it (the document pickers of
/// every lowering): only protocol version 14's grammar admits a
/// `summableOffCountIndex` index, so before it this is exactly
/// [`index_admissible_for_query`].
#[cfg(any(feature = "server", feature = "verify"))]
pub fn document_index_admissible_for_query(
    index: &Index,
    resolved_time_ranges: &[ResolvedTimeRange],
    skip_bindings: &[SkipIfAbsentBinding<'_>],
) -> bool {
    !index.is_summable_off_count_index()
        && index_admissible_for_query(index, resolved_time_ranges, skip_bindings)
}

/// Whether a point read pinning the first `pin_depth` properties of `index`
/// may stop at the deepest pin's value tree: the pins reach the chain
/// starting at `chain_position` (the shallowest ranked `at` level, whose
/// value trees and every one below aggregate their whole subtree) and leave a
/// deeper property free. The count and sum pickers and path builders all ask
/// this, each with its own chain position, so a picker never admits what its
/// builder refuses.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn pins_reach_chain(index: &Index, pin_depth: usize, chain_position: Option<usize>) -> bool {
    pin_depth >= 1
        && pin_depth < index.properties.len()
        && chain_position.is_some_and(|min_at| min_at < pin_depth)
}

/// Whether a range walk over `index` keeps a group whose aggregates are
/// zero: whether `index` can hold an empty group. A preallocated index
/// creates every group on its path with its referenced document, before any
/// entry, so its own groups can be empty, and so can those of any index of
/// the type ending at a level a preallocated index passes through, whose
/// value trees that preallocation creates too. An `outlivesDelete` index
/// passing through that level empties a group as well: its entries stay when
/// a delete takes the index's own, so the group's value tree stands at zero.
/// The walk's limit counted such a group, so dropping it would shorten a page
/// while more groups follow; the count, sum and average walks and the count
/// verifier all ask this, so the proof keeps what the unproven read keeps.
/// Only meta-schema v3 (protocol version 14) admits `preallocated` and
/// `outlivesDelete`, so every earlier index drops its empty groups as before.
///
/// One empty group it does not cover: a `nullSearchable: false` index keeps
/// the value tree of the null key (the empty key) with nothing in it, since
/// the walker creates the value tree before the reference step declines a
/// document without the value, and no delete prunes it. A range that includes
/// the empty key (`<`, `<=`, a `between` from it) still counts that group in
/// its limit and leaves it out, so such a page may come back short with more
/// groups after it. Keeping it here would change what the shipped count
/// verifier returns at every protocol version, so it stays a documented
/// exception.
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn index_keeps_empty_groups(document_type: DocumentTypeRef, index: &Index) -> bool {
    document_type.indexes().values().any(|other| {
        (other.preallocated || other.outlives_delete)
            && other.shares_leading_levels(index, index.properties.len())
    })
}

/// Whether a grovedb error from a raw path query says the queried path does
/// not exist yet (no document of the type, no entry under the index), which a
/// walk answers with no rows.
#[cfg(feature = "server")]
pub(crate) fn is_absent_path(error: &Error) -> bool {
    matches!(
        error,
        Error::GroveDB(e) if matches!(
            e.as_ref(),
            grovedb::Error::PathKeyNotFound(_)
                | grovedb::Error::PathNotFound(_)
                | grovedb::Error::PathParentLayerNotFound(_)
        )
    )
}

/// The refusal of a non-proof read whose indexOnly documents lack a required
/// property their index does not hold, so they cannot be serialized: the
/// query's doing, refused with a query error (`Unsupported`), not an internal
/// one.
#[cfg(feature = "server")]
pub fn uncovered_required_property_refusal() -> QuerySyntaxError {
    QuerySyntaxError::Unsupported(
        "this indexOnly query's index does not cover every required property, so the \
         documents it synthesizes cannot be serialized into a non-proof response; query \
         through an index covering all properties, or use a proved query"
            .to_string(),
    )
}

/// The query error a failure serializing a document an indexOnly index
/// synthesized answers a non-proof read with, `None` when the failure is no
/// query's doing: a missing required property, which
/// `DriveDocumentQuery::refuse_an_uncovered_index_only_projection` refuses
/// before the read, so this only guards a read that check let through. The
/// documents query and drive-abci's chained and composite handlers share it.
#[cfg(feature = "server")]
pub fn index_only_serialization_refusal(error: &ProtocolError) -> Option<QuerySyntaxError> {
    matches!(
        error,
        ProtocolError::DataContractError(
            dpp::data_contract::errors::DataContractError::MissingRequiredKey(_)
        )
    )
    .then(uncovered_required_property_refusal)
}

/// A range total's aggregate read without a proof (the tree at `path`), or
/// zero when that tree does not exist. An equality or `IN` value on the prefix
/// that no document holds has no subtree, and grovedb's
/// `query_aggregate_count`, `query_aggregate_sum` and
/// `query_aggregate_count_and_sum` open the leaf tree directly and fail with
/// `InvalidParentLayerPath`. grovedb raises that for any failure reading a
/// parent key, a corrupt or unreadable element included, so the zero is
/// answered only once a plain read of the path, from its root down, finds a key
/// missing; any other error stays an error. From protocol version 14 the
/// proof agrees: the range-total verifiers at version 1 accept the prover's
/// proof that the key is missing as a zero total
/// ([`crate::verify::or_empty_range_total`]), where version 0 refuses it. Not
/// where the missing key sits below an `IN` value of a carrier proof, between
/// the `IN` and the range: the carrier verifier refuses that proof, though
/// zero is the answer.
///
/// Keyed on `range_total_verifier`, the version of the verifier proving the
/// same total, so the unproven answer moves with the proved one: from version
/// 1, which only protocol version 14 selects, an absent value reads zero; at
/// version 0 the read fails with grovedb's `InvalidParentLayerPath` error, as
/// released.
#[cfg(feature = "server")]
pub(crate) fn aggregate_or_zero_when_absent<T: Default>(
    drive: &Drive,
    path: &[Vec<u8>],
    value: Result<T, grovedb::Error>,
    range_total_verifier: FeatureVersion,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<T, Error> {
    match (value, range_total_verifier) {
        (value, 0) => value.map_err(|e| Error::GroveDB(Box::new(e))),
        (Err(error @ grovedb::Error::InvalidParentLayerPath(_)), 1) => {
            for depth in 0..path.len() {
                let found = drive
                    .grove
                    .get_raw_optional(
                        path[..depth].into(),
                        &path[depth],
                        transaction,
                        &platform_version.drive.grove_version,
                    )
                    .unwrap()
                    .map_err(|e| Error::GroveDB(Box::new(e)))?;
                if found.is_none() {
                    return Ok(T::default());
                }
            }
            Err(Error::GroveDB(Box::new(error)))
        }
        (value, 1) => value.map_err(|e| Error::GroveDB(Box::new(e))),
        (_, version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method: "aggregate_or_zero_when_absent".to_string(),
            known_versions: vec![0, 1],
            received: version,
        })),
    }
}

/// The prefix-to-last point read's path query, shared by the count and sum
/// surfaces so a `count(*)` and a `sum`/`avg` over one tree build one shape:
/// the tree of the index's last property read as one element by its level key
/// `terminal_level_key` from the last pin's value tree at `base_path`.
/// Structurally the `Key([0])` shape with the terminal level key in place of
/// the bucket key: nothing is hoisted, so the `In` branches (`in_outer_keys`,
/// the pinned `In` values under `base_path`) keep their full trailing pairs
/// (`subquery_path_extension`) in `set_subquery_path`.
///
/// Unversioned, so every protocol version reaches it: the count point read
/// over a regular index (protocol version 12 on) builds exactly this, moved
/// here unchanged, so an edit for the sum surface changes those proofs too.
#[cfg(any(feature = "server", feature = "verify"))]
pub(crate) fn prefix_to_last_path_query(
    base_path: Vec<Vec<u8>>,
    in_outer_keys: Option<Vec<Vec<u8>>>,
    subquery_path_extension: Vec<Vec<u8>>,
    terminal_level_key: Vec<u8>,
) -> PathQuery {
    match in_outer_keys {
        None => {
            let mut query = Query::new();
            query.insert_key(terminal_level_key);
            PathQuery::new(base_path, SizedQuery::new(query, None, None))
        }
        Some(keys) => {
            let mut outer_query = Query::new();
            for key in keys {
                outer_query.insert_key(key);
            }
            let mut subquery = Query::new();
            subquery.insert_key(terminal_level_key);
            if !subquery_path_extension.is_empty() {
                outer_query.set_subquery_path(subquery_path_extension);
            }
            outer_query.set_subquery(subquery);
            PathQuery::new(base_path, SizedQuery::new(outer_query, None, None))
        }
    }
}

/// Refuses a range total (one aggregate over a range, or one per carrier
/// branch) read through `index` when its path passes through a ranked level:
/// a level whose property-name tree Drive lays out as an indexed tree,
/// whichever of the type's indexes ranks it (an index may continue below
/// another's ranked last property). It walks the type's index structure along
/// the index's levels and asks each the resolver the write path uses
/// ([`property_name_tree_type_and_ranked_axes_for_level`]), so it reads the
/// layout Drive builds. grovedb's range aggregates neither read an indexed
/// tree (`AggregateCountOnRange`, `AggregateSumOnRange` and
/// `AggregateCountAndSumOnRange` take provable trees only) nor descend
/// through one when proving. An unproven read through a ranked ancestor
/// would succeed (it opens only the leaf tree), but it is refused as well:
/// every count, sum and count-and-sum range-total path builder calls this, so
/// the unproven read, the proof and its verification refuse alike rather than
/// answering only without a proof. Grouped by the last property, the same
/// range reads each value.
///
/// Unversioned, so every protocol version reaches it: rankings exist only
/// from protocol version 14 (meta-schema v3), so no level is an indexed tree
/// before and it refuses nothing.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn refuse_a_range_total_through_a_ranked_index(
    document_type: DocumentTypeRef,
    index: &Index,
) -> Result<(), Error> {
    let mut level = document_type.index_structure();
    for (position, property) in index.properties.iter().enumerate() {
        let Some(sub_level) = level
            .sub_levels()
            .get(&index.level_key(position, &property.name))
        else {
            return Ok(());
        };
        if !property_name_tree_type_and_ranked_axes_for_level(sub_level)?
            .1
            .is_empty()
        {
            let last = index
                .properties
                .last()
                .map(|property| property.name.as_str())
                .unwrap_or_default();
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "a range total over the index `{}` is not available: its path passes through a \
                 ranked level, and a range total is read only through unranked trees; group by \
                 `{last}` to read each value in the range",
                index.name
            ))));
        }
        level = sub_level;
    }
    Ok(())
}

/// Rejects a query whose resolution provenance and clause shapes disagree:
/// every field in `resolved_time_ranges` must appear in the where
/// clauses as exactly one `Equal` clause — the only shape
/// [`resolve_time_range_bucket_clause`] produces.
///
/// A range or `In` clause on a resolved field means the caller attached
/// provenance to a clause the resolver never built. Executors that fan a
/// clause out per value (the per-`In`-value count/sum paths rewrite each `In`
/// value into an equality) would then present raw client values to the index
/// pickers as if they were resolved bucket starts, and the pickers would
/// admit the bucketed index for them. The wire path can never produce the
/// mismatch — provenance is not parseable from the wire, and the abci handler
/// pushes the resolved equality itself — so this guards direct API callers,
/// and it runs identically under `server` and `verify`.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn validate_resolved_time_range_clause_shapes(
    where_clauses: &[WhereClause],
    resolved_time_ranges: &[ResolvedTimeRange],
) -> Result<(), Error> {
    for field in resolved_time_ranges.iter().map(|resolved| resolved.field()) {
        let mut equalities = 0usize;
        for clause in where_clauses.iter().filter(|c| c.field == field) {
            if clause.operator == WhereOperator::Equal {
                equalities += 1;
            } else {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "a field resolved from a window selection (IN_TIME_RANGE or \
                         IN_INTEGER_RANGE) may only carry the single equality its resolution \
                         produced, not a range or In clause",
                    ),
                ));
            }
        }
        if equalities != 1 {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "a field resolved from a window selection (IN_TIME_RANGE or \
                     IN_INTEGER_RANGE) must carry exactly one equality clause — the one its \
                     resolution produced",
                ),
            ));
        }
    }
    Ok(())
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
    /// The fields whose equality clause in `internal_clauses` was produced by
    /// `IN_TIME_RANGE` resolution — i.e. by
    /// [`resolve_time_range_bucket_clause`], on the server from committed
    /// block time and in the verifier from the quorum-signed response metadata
    /// time.
    ///
    /// Never parsed from the wire: every `from_cbor` / `from_value` /
    /// `from_typed_clauses` entry point leaves this empty, so a client cannot
    /// claim resolution it did not go through. It is what
    /// [`Self::find_best_index`] uses to pin index selection to the index that
    /// buckets the field (see [`index_admissible_for_resolved_time_range`]),
    /// which is required because the resolved clause is an ordinary equality
    /// and cannot be told apart from a raw-timestamp lookup once built.
    ///
    /// Empty for every raw query.
    pub resolved_time_ranges: Vec<ResolvedTimeRange>,
    /// The composite sub-queries: queries whose `IN` clauses are derived
    /// from this query's proven results (by-id joins, indexed lookups,
    /// counts — see the [`composite_document_query`] module docs), listed
    /// in binding order (a sub-query may only bind an earlier one) and
    /// answered together with this query as ONE merged grovedb proof.
    ///
    /// Empty for an ordinary documents query, which is what every plain
    /// entry point requires: a query carrying sub-queries is served by
    /// `Drive::query_composite_documents` /
    /// `query_composite_documents_with_proof` and verified by
    /// `verify_composite_documents_proof`, and the plain
    /// query/proof/verify surfaces refuse it rather than silently prove
    /// the page alone.
    ///
    /// Never parsed from the wire: every `from_cbor` / `from_value` /
    /// `from_typed_clauses` entry point leaves this empty; composite
    /// requests are built programmatically (see
    /// [`Self::with_sub_queries`]).
    pub sub_queries: Vec<DriveSubQuery<'a>>,
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
                in_clauses: Vec::new(),
                range_clause: None,
                equal_clauses: Default::default(),
            },
            offset: None,
            limit: None,
            order_by: Default::default(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
            resolved_time_ranges: vec![],
            sub_queries: vec![],
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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Extends this query into a composite one: `self` becomes the page
    /// and `sub_queries` are derived from its proven results — see
    /// [`Self::sub_queries`] and the [`composite_document_query`] module
    /// docs.
    pub fn with_sub_queries(mut self, sub_queries: Vec<DriveSubQuery<'a>>) -> Self {
        self.sub_queries = sub_queries;
        self
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Appends a by-id join sub-query: `source_property`'s values, read
    /// off this query's proven documents, become the `$id`s of
    /// `document_type` documents fetched from the same contract. The
    /// property must carry a `refersTo: permanentDocument` declaration
    /// targeting `document_type`, so every derived id resolves.
    ///
    /// This is the one shape the chained surface
    /// (`Drive::query_chained_documents`,
    /// `verify_chained_documents_proof`) requires exactly one of, and one
    /// of the composite sub-query shapes. A cross-contract by-id join
    /// (composite only) is built by pushing a [`DriveSubQuery`] with the
    /// target contract instead.
    pub fn with_by_id_join(
        mut self,
        source_property: impl Into<String>,
        document_type: DocumentTypeRef<'a>,
    ) -> Self {
        self.sub_queries.push(DriveSubQuery {
            contract: self.contract,
            document_type,
            kind: SubQueryKind::Documents,
            where_clauses: vec![],
            order_by: vec![],
            limit: None,
            binding: Some(SubQueryBinding {
                source: BindingSource::Page,
                source_property: source_property.into(),
                field: document::property_names::ID.to_string(),
            }),
        });
        self
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Refuses a query carrying composite sub-queries on a plain
    /// (page-only) surface, which would otherwise silently ignore them —
    /// on the verify side that would mean reporting the composition
    /// verified when only the page was.
    pub(crate) fn ensure_no_sub_queries(&self, surface: &str) -> Result<(), Error> {
        if self.sub_queries.is_empty() {
            return Ok(());
        }
        Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "this query carries {} sub-queries, which {} would silently ignore; execute and \
             verify it on the composite surface (query_composite_documents / \
             verify_composite_documents_proof) or, for a single by-id join, the chained one \
             (query_chained_documents / verify_chained_documents_proof)",
            self.sub_queries.len(),
            surface,
        ))))
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
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let query_document_value: Value = ciborium::de::from_reader(query_cbor).map_err(|_| {
            Error::Query(QuerySyntaxError::DeserializationError(
                "unable to decode query from cbor".to_string(),
            ))
        })?;
        Self::from_value(
            query_document_value,
            contract,
            document_type,
            config,
            platform_version,
        )
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a query Value to a `DriveQuery`.
    pub fn from_value(
        query_value: Value,
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let query_document: BTreeMap<String, Value> = query_value.into_btree_string_map()?;
        Self::from_btree_map_value(
            query_document,
            contract,
            document_type,
            config,
            platform_version,
        )
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a query Value to a `DriveQuery`.
    pub fn from_btree_map_value(
        mut query_document: BTreeMap<String, Value>,
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        if let Some(contract_id) = query_document
            .remove_optional_identifier("contract_id")
            .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))?
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
            .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))?
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
            .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))?;

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
            .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))?;

        let block_time_ms: Option<u64> = query_document
            .remove_optional_integer("blockTime")
            .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))?;

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
                                        "where clause must be an array".to_string(),
                                    )))
                                }
                            })
                            .collect::<Result<Vec<WhereClause>, Error>>()
                    } else {
                        Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                            "where clause must be an array".to_string(),
                        )))
                    }
                })?;

        let internal_clauses =
            InternalClauses::extract_from_clauses(all_where_clauses, platform_version)?;

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
                    .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))
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
                                            .map_err(Error::from);
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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
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
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let all_where_clauses: Vec<WhereClause> = match where_clause {
            Value::Null => Ok(vec![]),
            Value::Array(clauses) => clauses
                .iter()
                .map(|where_clause| {
                    if let Value::Array(clauses_components) = where_clause {
                        WhereClause::from_components(clauses_components)
                    } else {
                        Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                            "where clause must be an array".to_string(),
                        )))
                    }
                })
                .collect::<Result<Vec<WhereClause>, Error>>(),
            _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                "where clause must be an array".to_string(),
            ))),
        }?;

        // Malformed `order_by` payloads reject the request — the
        // pre-existing `filter_map(... .ok())` here silently dropped
        // bad clauses (or the whole field for non-array shapes),
        // which could mutate result ordering and (on the prove
        // path) proof bytes without telling the caller. Tighten the
        // contract: every clause must parse, and the top-level
        // shape must be `Value::Null` or `Value::Array`.
        let order_by_clauses: Vec<OrderClause> = match order_by {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(clauses)) => clauses
                .iter()
                .map(|order_clause| match order_clause {
                    Value::Array(components) => {
                        OrderClause::from_components(components).map_err(|_| {
                            Error::Query(QuerySyntaxError::InvalidOrderByProperties(
                                "invalid order_by clause components",
                            ))
                        })
                    }
                    _ => Err(Error::Query(QuerySyntaxError::InvalidOrderByProperties(
                        "order_by clause must be an array",
                    ))),
                })
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => {
                return Err(Error::Query(QuerySyntaxError::InvalidOrderByProperties(
                    "order_by must be an array",
                )));
            }
        };

        Self::from_typed_clauses(
            all_where_clauses,
            order_by_clauses,
            maybe_limit,
            start_at,
            start_at_included,
            block_time_ms,
            contract,
            document_type,
            config,
            platform_version,
        )
    }

    /// Build a `DriveDocumentQuery` from already-structured where /
    /// order_by clauses. This is the typed-input twin of
    /// [`Self::from_decomposed_values`] — same downstream shape, just
    /// without the `Value::Array(...)` parse step.
    ///
    /// Used by the v1 `getDocuments` ABCI handler whose wire format
    /// carries `repeated WhereClause` / `repeated OrderClause`
    /// natively (no CBOR envelope). The v0 path keeps using
    /// `from_decomposed_values` so its CBOR-decoded inputs flow
    /// through the existing `WhereClause::from_components` parser
    /// for shape validation; the typed path expects that validation
    /// (or the equivalent proto→drive conversion) to have run
    /// upstream.
    ///
    /// Limit semantics mirror `from_decomposed_values`:
    /// `maybe_limit = None` or `Some(0)` falls back to
    /// `config.default_query_limit`; `Some(N)` with `N >
    /// config.default_query_limit` is rejected as
    /// `QuerySyntaxError::InvalidLimit`.
    #[cfg(any(feature = "server", feature = "verify"))]
    #[allow(clippy::too_many_arguments)]
    pub fn from_typed_clauses(
        where_clauses: Vec<WhereClause>,
        order_by_clauses: Vec<OrderClause>,
        maybe_limit: Option<u16>,
        start_at: Option<[u8; 32]>,
        start_at_included: bool,
        block_time_ms: Option<u64>,
        contract: &'a DataContract,
        document_type: DocumentTypeRef<'a>,
        config: &DriveConfig,
        platform_version: &PlatformVersion,
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

        let internal_clauses =
            InternalClauses::extract_from_clauses(where_clauses, platform_version)?;

        let order_by: IndexMap<String, OrderClause> = order_by_clauses
            .into_iter()
            .map(|c| (c.field.clone(), c))
            .collect();

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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
        })
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Converts a SQL expression to a `DriveQuery`.
    pub fn from_sql_expr(
        sql_string: &str,
        contract: &'a DataContract,
        config: Option<&DriveConfig>,
        platform_version: &PlatformVersion,
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

        let internal_clauses =
            InternalClauses::extract_from_clauses(all_where_clauses, platform_version)?;

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
                    .map_err(|e| Error::Protocol(Box::new(ProtocolError::ValueError(e))))
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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
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

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Versioned preflight over the non-primary-key `In` clause shape.
    ///
    /// Runs before any cursor storage lookup or proof processing so the
    /// rejection precedence matches each protocol version's contract: v0
    /// rejects more than one `In` clause with `MultipleInClauses` before a
    /// `startAt`/`startAfter` document is ever fetched (matching the
    /// pre-protocol-version-14 parse-time rejection), and v1 rejects the
    /// unsupported multi-`In` + cursor combination with `Unsupported`
    /// before spending state or proof work on the cursor. The lowering
    /// keeps equivalent guards for callers that reach it directly.
    pub fn validate_in_clause_shape(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .non_primary_key_path_query
        {
            0 => {
                if self.internal_clauses.in_clauses.len() > 1 {
                    return Err(Error::Query(QuerySyntaxError::MultipleInClauses(
                        "There should only be one in clause",
                    )));
                }
                Ok(())
            }
            1 => {
                if self.internal_clauses.in_clauses.len() > 1 && self.start_at.is_some() {
                    return Err(Error::Query(QuerySyntaxError::Unsupported(
                        "startAt/startAfter is not supported with multiple in clauses".to_string(),
                    )));
                }
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentQuery::validate_in_clause_shape".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
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
        self.validate_in_clause_shape(platform_version)?;
        // indexOnly documents have no primary-key tree: nothing is ever
        // addressed by document id, so a by-id query has no tree to land on.
        {
            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if self.document_type.index_only()
                && self.is_for_primary_key()
                && !self.index_only_flat_scan_applies()
            {
                return Err(Error::Query(QuerySyntaxError::Unsupported(
                    "indexOnly documents cannot be fetched by id: there is no primary-key \
                     tree; query through one of the type's indexes"
                        .to_string(),
                )));
            }
            if self.document_type.index_only() && self.start_at.is_some() {
                return Err(Error::Query(QuerySyntaxError::Unsupported(
                    "startAt/startAfter cursors cannot address an indexOnly position (the \
                     synthesized document id is a one-way hash of it); paginate with a \
                     range clause on the terminal property instead — equality clauses on \
                     the index's properties, `terminal > <last seen value>` ordered by the \
                     terminal, and a limit"
                        .to_string(),
                )));
            }
        }
        let drive_version = &platform_version.drive;
        // First we should get the overall document_type_path
        let document_type_path = self
            .contract
            .document_type_path(self.document_type.name().as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();

        // indexOnly terminal-clause route: a clause on an index's terminal
        // lowers onto the entry level's member keys when the generic
        // matcher cannot serve the query. Shared with the verifier-side
        // constructor below so prover and verifier build the same query.
        {
            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if self.document_type.index_only() {
                if let Some(path_query) =
                    self.index_only_route(&document_type_path, platform_version)?
                {
                    return Ok(path_query);
                }
            }
        }

        let cursor_included = self.start_at_included || self.pads_cursor_page(platform_version);
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
                        Error::GroveDB(e)
                            if matches!(
                                e.as_ref(),
                                GroveError::PathKeyNotFound(_)
                                    | GroveError::PathNotFound(_)
                                    | GroveError::PathParentLayerNotFound(_)
                            ) =>
                        {
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
                    Ok((Some((document, cursor_included)), Some(path_query)))
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
        self.pad_cursor_page_limit(&mut main_path_query, platform_version)?;
        if !include_start_at_for_proof {
            return Ok(main_path_query);
        }

        if let Some(mut start_at_path_query) = start_at_path_query {
            // The cursor query selects exactly one key, so its walk
            // direction carries no meaning — but grovedb's merge (V4+)
            // requires every input to agree on direction, so align it to
            // the main query's `orderBy` direction so a descending page
            // merges at all.
            start_at_path_query.query.query.left_to_right =
                main_path_query.query.query.left_to_right;
            let limit = main_path_query.query.limit.take();
            let mut merged = PathQuery::merge(
                vec![&start_at_path_query, &main_path_query],
                &platform_version.drive.grove_version,
            )
            .map_err(Error::from)?;
            // Where the merge lands decides how the two queries combine. An
            // index-ordered page lives under its index tree while the cursor
            // lookup lives under the primary-key tree, so the merge synthesizes
            // a root one level above both with each query in its own branch.
            // A `$id`-ordered page addresses the primary-key tree directly: the
            // cursor lookup shares that path (or, for a history-keeping type,
            // sits one level below it), so the merge point is the page query's
            // own root layer and the merged query IS that layer.
            let cursor_on_page_layer = merged.path == main_path_query.path;
            // The cursor's key on that shared layer: the cursor query's own key
            // when both paths coincide, otherwise the path component the cursor
            // query descends through.
            let cursor_key_on_page_layer: Option<&[u8]> = if !cursor_on_page_layer {
                None
            } else if let Some(component) = start_at_path_query.path.get(merged.path.len()) {
                Some(component.as_slice())
            } else {
                match start_at_path_query.query.query.items.as_slice() {
                    [QueryItem::Key(cursor_key)] => Some(cursor_key.as_slice()),
                    _ => None,
                }
            };
            // On a shared layer the cursor row is already one of the page's
            // rows whenever the page's own items cover it (an inclusive
            // `startAt` on a range, or an `in` list that names the cursor). It
            // then needs no reserved slot: reserving one would make the layer
            // return `limit + 1` rows matching the page query, which the
            // verifier rejects as more data than its limit.
            let cursor_row_in_page = cursor_key_on_page_layer.is_some_and(|cursor_key| {
                main_path_query
                    .query
                    .query
                    .items
                    .iter()
                    .any(|item| item.contains(cursor_key))
            });
            merged.query.limit = limit.map(|a| {
                if cursor_row_in_page {
                    a
                } else {
                    a.saturating_add(1)
                }
            });
            if !cursor_on_page_layer {
                // A synthesized root must be walked ascending regardless of
                // the page's `orderBy` direction: the `limit + 1` above
                // reserves one result slot for the cursor document, and the
                // prover spends the budget in root traversal order. Ascending,
                // the cursor branch (key `[0]`) is visited first and takes its
                // reserved slot; descending, the index branch sorts first,
                // consumes the whole budget mid-timeline, and the prover then
                // omits the cursor subtree's lower layer — an unverifiable
                // proof (the verifier extracts the cursor document from the
                // proof before rebuilding the main query). Only this
                // synthesized root flips: each input's own query lands intact
                // inside a subquery branch, keeping in-branch result order.
                // The verifier never rebuilds the merged query — it runs the
                // cursor and main queries as separate subset queries — so the
                // root's direction is not client-visible.
                //
                // A shared layer has no synthesized root to flip: the merged
                // query IS the page query's own layer, which the verifier
                // walks in the requested direction, and the cursor row sits at
                // the page's boundary, so the requested direction reaches it
                // first anyway.
                merged.query.query.left_to_right = true;
            }
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
        self.validate_in_clause_shape(platform_version)?;
        // indexOnly documents have no primary-key tree: nothing is ever
        // addressed by document id, so a by-id query has no tree to land on.
        {
            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if self.document_type.index_only()
                && self.is_for_primary_key()
                && !self.index_only_flat_scan_applies()
            {
                return Err(Error::Query(QuerySyntaxError::Unsupported(
                    "indexOnly documents cannot be fetched by id: there is no primary-key \
                     tree; query through one of the type's indexes"
                        .to_string(),
                )));
            }
            if self.document_type.index_only() && self.start_at.is_some() {
                return Err(Error::Query(QuerySyntaxError::Unsupported(
                    "startAt/startAfter cursors cannot address an indexOnly position (the \
                     synthesized document id is a one-way hash of it); paginate with a \
                     range clause on the terminal property instead — equality clauses on \
                     the index's properties, `terminal > <last seen value>` ordered by the \
                     terminal, and a limit"
                        .to_string(),
                )));
            }
        }
        // First we should get the overall document_type_path
        let document_type_path = self
            .contract
            .document_type_path(self.document_type.name().as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();

        // indexOnly terminal-clause route — the verifier-side mirror of
        // the dispatch in `construct_path_query_operations`, so both
        // sides build the same query.
        {
            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if self.document_type.index_only() {
                if let Some(path_query) =
                    self.index_only_route(&document_type_path, platform_version)?
                {
                    return Ok(path_query);
                }
            }
        }

        let cursor_included = self.start_at_included || self.pads_cursor_page(platform_version);
        let starts_at_document =
            starts_at_document.map(|starts_at_document| (starts_at_document, cursor_included));
        let mut path_query = if self.is_for_primary_key() {
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
        self.pad_cursor_page_limit(&mut path_query, platform_version)?;
        Ok(path_query)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Whether a `startAfter` cursor on this query is lowered as `startAt`
    /// with one extra result slot, the cursor document then being dropped
    /// from the page by [`Self::strip_cursor_from_page`].
    ///
    /// GroveDB charges a result slot for every visited subtree whose
    /// subquery yields nothing, and the prover accounts the same way. A
    /// lowering that visits the cursor's branch or index key looking for
    /// rows *after* the cursor therefore pays that slot whenever nothing
    /// follows, which is nearly every page on unique-valued data: pages
    /// came back one row short, and a limit of one came back empty while
    /// later rows remained. Keeping the cursor document in the walk keeps
    /// every subtree on its path non-empty, and documents sharing its
    /// index key continue by document id. The primary-key path has no
    /// subtrees to charge and is left alone, as is the released (protocol
    /// version 13) lowering.
    pub fn pads_cursor_page(&self, platform_version: &PlatformVersion) -> bool {
        use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
        self.start_at.is_some()
            && !self.start_at_included
            && !self.is_for_primary_key()
            && !self.document_type.index_only()
            && platform_version
                .drive
                .methods
                .document
                .query
                .non_primary_key_path_query
                >= 1
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Reserves the cursor document's slot on a padded `startAfter` page
    /// (see [`Self::pads_cursor_page`]). A page offset is folded into the
    /// fetch and applied by [`Self::strip_cursor_from_page`] once the
    /// cursor document is gone: left to GroveDB, the offset would consume
    /// the cursor row itself and the page would start one row early.
    fn pad_cursor_page_limit(
        &self,
        path_query: &mut PathQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !self.pads_cursor_page(platform_version) {
            return Ok(());
        }
        let offset = path_query.query.offset.take().unwrap_or(0);
        if let Some(limit) = path_query.query.limit {
            let padded = limit
                .checked_add(1)
                .and_then(|limit| limit.checked_add(offset))
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "limit {limit} and offset {offset} are too large together with a \
                         startAfter cursor"
                    )))
                })?;
            path_query.query.limit = Some(padded);
        }
        Ok(())
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Drops the cursor document from a page produced by a padded
    /// `startAfter` query (see [`Self::pads_cursor_page`]), applies the
    /// query's offset, and trims the page back to the query's limit,
    /// returning the page with the number of rows the offset skipped.
    /// When the cursor document matches the query it is the page's first
    /// row, since the lowering excludes everything ordered before it; a
    /// cursor outside the query's clauses is simply absent and the trim
    /// restores the limit.
    pub(crate) fn strip_cursor_from_page(
        &self,
        mut serialized_documents: Vec<Vec<u8>>,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
        use dpp::document::DocumentV0Getters;
        if !self.pads_cursor_page(platform_version) {
            return Ok((serialized_documents, 0));
        }
        let Some(start_at) = self.start_at else {
            return Ok((serialized_documents, 0));
        };
        if let Some(first) = serialized_documents.first() {
            let document = Document::from_bytes(first, self.document_type, platform_version)?;
            if document.id().to_buffer() == start_at {
                serialized_documents.remove(0);
            }
        }
        let skipped = (self.offset.unwrap_or(0) as usize).min(serialized_documents.len());
        serialized_documents.drain(..skipped);
        if let Some(limit) = self.limit {
            serialized_documents.truncate(limit as usize);
        }
        Ok((serialized_documents, skipped as u16))
    }

    #[cfg(feature = "server")]
    /// [`Self::strip_cursor_from_page`] over query result elements.
    fn strip_cursor_from_elements(
        &self,
        mut elements: QueryResultElements,
        platform_version: &PlatformVersion,
    ) -> Result<(QueryResultElements, u16), Error> {
        use dpp::document::DocumentV0Getters;
        use grovedb::query_result_type::QueryResultElement;
        if !self.pads_cursor_page(platform_version) {
            return Ok((elements, 0));
        }
        let Some(start_at) = self.start_at else {
            return Ok((elements, 0));
        };
        let first_element = match elements.elements.first() {
            Some(QueryResultElement::ElementResultItem(element))
            | Some(QueryResultElement::KeyElementPairResultItem((_, element)))
            | Some(QueryResultElement::PathKeyElementTrioResultItem((_, _, element))) => {
                Some(element)
            }
            None => None,
        };
        let first_is_cursor = match first_element {
            Some(Element::Item(bytes, _)) => {
                Document::from_bytes(bytes, self.document_type, platform_version)?
                    .id()
                    .to_buffer()
                    == start_at
            }
            _ => false,
        };
        if first_is_cursor {
            elements.elements.remove(0);
        }
        let skipped = (self.offset.unwrap_or(0) as usize).min(elements.elements.len());
        elements.elements.drain(..skipped);
        if let Some(limit) = self.limit {
            elements.elements.truncate(limit as usize);
        }
        Ok((elements, skipped as u16))
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
                let in_values = primary_key_in_clause.in_values().into_data_with_error()??;

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
    ///
    /// Queries with more than one `In` clause use their own selection
    /// ([`Self::find_best_index_for_multiple_in_clauses`]); they only
    /// reach it through the v1 (protocol version 14+) path-query
    /// lowering, since the v0 lowering rejects them first.
    ///
    /// Selection is restricted to the indexes admissible for this query's
    /// [`Self::resolved_time_ranges`]: a query carrying an
    /// `IN_TIME_RANGE`-resolved equality may only be served by the index that
    /// buckets that field, and a raw query may never be served by a bucketed
    /// index. See [`index_admissible_for_resolved_time_range`] for why either
    /// mismatch would produce a validly-proven wrong answer. The rule applies
    /// on both routes, including the multiple-`In` selection.
    pub fn find_best_index(&self, platform_version: &PlatformVersion) -> Result<&Index, Error> {
        match self.select_best_index(platform_version)? {
            BestIndexOutcome::Matched(index) => Ok(index),
            BestIndexOutcome::NoIndexMatches(no_index_error) => Err(no_index_error),
        }
    }

    /// Generic index selection with "no index matches" separated from the
    /// structural failures, in the type instead of in error variants:
    /// `Err` is a structural problem with the query itself (preflight,
    /// resolved-source shape, version dispatch) and always propagates,
    /// while `NoIndexMatches` carries the would-be [`Self::find_best_index`]
    /// error as a value — a routing fact the indexOnly terminal route is
    /// allowed to stand in for. [`Self::find_best_index`] collapses both
    /// non-matches back into `Err` for every ordinary caller.
    pub(crate) fn select_best_index(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<BestIndexOutcome<'_>, Error> {
        // A transform's source must be its index's first property, so one
        // index buckets exactly one field and no index can carry two resolved
        // equalities. Serving such a query would need a join across two
        // bucketed indexes, which the engine has no shape for. This runs
        // before any routing so the multiple-`In` path cannot bypass it.
        if self.resolved_time_ranges.len() > 1 {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "at most one window selection (IN_TIME_RANGE or IN_INTEGER_RANGE) is \
                 supported per query; this one resolves {:?}, and no single index can bucket \
                 more than one field",
                self.resolved_time_ranges
            ))));
        }

        // One shared source-shape guard for every selection route — see its
        // doc for the contract. Running it before routing keeps the single-
        // and multiple-`In` routes rejecting the same shapes.
        self.validate_resolved_source_shape()?;

        if self.internal_clauses.in_clauses.len() > 1 {
            // The multi-`In` machinery keeps its own error surface; its
            // shapes are never terminal-routable, so there is nothing to
            // classify as a plain miss.
            return Ok(BestIndexOutcome::Matched(
                self.find_best_index_for_multiple_in_clauses()?.0,
            ));
        }

        let equal_fields = self
            .internal_clauses
            .equal_clauses
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();
        let in_field = self
            .internal_clauses
            .in_clauses
            .first()
            .map(|in_clause| in_clause.field.as_str());
        let range_field = self
            .internal_clauses
            .range_clause
            .as_ref()
            .map(|range_clause| range_clause.field.as_str());
        let order_by_keys: Vec<&str> = self.order_by.keys().map(String::as_str).collect();

        // Every constraint the query makes, for the skip-index
        // admissibility gate — the by-role slices above are what the
        // matcher consumes. An all-unused match inside the difference
        // budget could otherwise select a sparse index for a query that
        // never names its skip properties.
        let skip_bindings = self
            .internal_clauses
            .skip_if_absent_bindings(&order_by_keys);

        let Some((index, difference)) = self.document_type.index_for_types_matching(
            equal_fields.as_slice(),
            range_field,
            in_field,
            order_by_keys.as_slice(),
            // The document form of the gate, reached by every protocol version:
            // its counter exclusion is inert before protocol version 14 (see
            // `document_index_admissible_for_query`).
            |index| {
                document_index_admissible_for_query(
                    index,
                    &self.resolved_time_ranges,
                    &skip_bindings,
                )
            },
            platform_version,
        )?
        else {
            return Ok(BestIndexOutcome::NoIndexMatches(
                match self.resolved_time_ranges.first() {
                    // A window query is only servable by the index that
                    // buckets the field with the resolved grid, so "no index"
                    // here is a narrower fact than the generic case: some index
                    // buckets the field (the clause could not have been resolved
                    // otherwise), but none with that grid also covers the rest
                    // of the query.
                    Some(resolved) => {
                        Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
                            "a {} query on \"{}\" requires an index that buckets it with \
                         the resolved grid AND covers the query's other where and order-by \
                         fields; valid indexes are: {:?}",
                            resolved.kind(),
                            resolved.field(),
                            self.document_type.indexes()
                        )))
                    }
                    None => {
                        // A raw query never binds to a bucketed index; when one
                        // exists, say so — the caller may be holding a
                        // time-range proof on a surface that cannot supply
                        // resolution provenance (e.g. the standalone wasm
                        // verifiers), where this refusal is otherwise opaque.
                        let has_bucketed_index = self
                            .document_type
                            .indexes()
                            .values()
                            .any(|index| index.is_bucketed());
                        if has_bucketed_index {
                            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                                format!(
                            "query must be for valid indexes, valid indexes are: {:?}; note: \
                             this document type's bucketed (timeRange / integerRange) indexes \
                             only serve IN_TIME_RANGE / IN_INTEGER_RANGE selections carrying \
                             their resolution — a raw clause on the bucketed field never binds \
                             to them",
                            self.document_type.indexes()
                        ),
                            ))
                        } else {
                            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                                format!(
                                    "query must be for valid indexes, valid indexes are: {:?}",
                                    self.document_type.indexes()
                                ),
                            ))
                        }
                    }
                },
            ));
        };
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Ok(BestIndexOutcome::NoIndexMatches(Error::Query(
                QuerySyntaxError::QueryTooFarFromIndex("query must better match an existing index"),
            )));
        }

        // The residual source-shape contract already ran at the top of this
        // function ([`Self::validate_resolved_source_shape`]) — with a
        // resolution present, admissibility restricts candidates to the one
        // index bucketing exactly the resolved field, so guarding by
        // provenance there is equivalent to guarding by the selected index's
        // transform here.
        Ok(BestIndexOutcome::Matched(index))
    }

    /// The residual source-shape contract for a query carrying a
    /// time-range resolution: the resolved equality must be present on the
    /// bucketed source, and the source must not ALSO carry an `In`, a
    /// range, or an ordering — those walk overlapping bucket keys and
    /// return each document up to `overlap_factor` times with a perfectly
    /// valid proof. The `!has_equality_on_source` arm is defensive:
    /// resolution always pushes the equality, so reaching it means the
    /// provenance and the clauses disagree.
    ///
    /// Runs identically on the server and in proof verification, and on
    /// every selection route: [`Self::find_best_index`] calls it before
    /// routing, and [`Self::find_best_index_for_multiple_in_clauses`]
    /// calls it itself because the multiple-`In` execution lowering picks
    /// its index directly, without going through `find_best_index`.
    #[cfg(any(feature = "server", feature = "verify"))]
    pub(crate) fn validate_resolved_source_shape(&self) -> Result<(), Error> {
        let Some(resolved) = self.resolved_time_ranges.first() else {
            return Ok(());
        };
        let source = resolved.field();
        let has_equality_on_source = self.internal_clauses.equal_clauses.contains_key(source);
        let range_or_in_on_source = self
            .internal_clauses
            .range_clause
            .as_ref()
            .is_some_and(|clause| clause.field == source)
            || self
                .internal_clauses
                .in_clauses
                .iter()
                .any(|clause| clause.field == source);
        if !has_equality_on_source || range_or_in_on_source || self.order_by.contains_key(source) {
            return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                "the index on \"{source}\" buckets it into {kind} windows: it can only be \
                 queried through a {kind} selection ({operator}, which resolves to an exact \
                 window equality), not with ranges, IN, or ordering on that property",
                kind = resolved.kind(),
                operator = resolved.operator(),
            ))));
        }
        Ok(())
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
    /// Returns a path query for non-primary keys given a document type path and starting document.
    ///
    /// Versioned because the set of accepted query shapes is part of the
    /// consensus query contract: v0 rejects more than one `In` clause per
    /// query, v1 (protocol version 14) lowers multiple `In` clauses on
    /// consecutive index properties to a multi-level key-set path query.
    pub fn get_non_primary_key_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let starts_at_document = match starts_at_document {
            Some((document, included)) => Some((
                self.cursor_with_derived_values(document, platform_version)?,
                included,
            )),
            None => None,
        };
        match platform_version
            .drive
            .methods
            .document
            .query
            .non_primary_key_path_query
        {
            0 => self.get_non_primary_key_path_query_v0(
                document_type_path,
                starts_at_document,
                platform_version,
            ),
            1 => self.get_non_primary_key_path_query_v1(
                document_type_path,
                starts_at_document,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentQuery::get_non_primary_key_path_query".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// A startAt or startAfter cursor places the page by the values the document it names
    /// holds for the index's properties, read off the stored document (by the server, and by
    /// a verifier from the proof), which holds no derived index property's value (protocol
    /// version 14: read from the document a reference points at). So a query paging with a
    /// cursor may only go through an index whose derived properties it fixes with `==`: the
    /// cursor document is given those values from the query itself, as the page it places
    /// holds them. Any other such query is refused. A cursor of a type without derived index
    /// properties passes through unchanged, which is every cursor before protocol version 14.
    #[cfg(any(feature = "server", feature = "verify"))]
    fn cursor_with_derived_values(
        &self,
        mut document: Document,
        platform_version: &PlatformVersion,
    ) -> Result<Document, Error> {
        let derived_index_properties = self.document_type.derived_index_properties();
        if derived_index_properties.is_empty() {
            return Ok(document);
        }
        let index = self.find_best_index(platform_version)?;
        for property in &index.properties {
            if !derived_index_properties.contains_key(&property.name) {
                continue;
            }
            let Some(clause) = self.internal_clauses.equal_clauses.get(&property.name) else {
                return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "a startAt or startAfter cursor is placed by the values the document it \
                     names stores, and the index {} reads \"{}\" from the document a reference \
                     points at instead: fix \"{}\" with == to page with a cursor, or page by a \
                     range on another property of the index",
                    index.name, property.name, property.name
                ))));
            };
            document
                .properties_mut()
                .insert(property.name.clone(), clause.value.clone());
        }
        Ok(document)
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
        self.ensure_no_sub_queries("execute_with_proof")?;
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
        self.ensure_no_sub_queries("execute_with_proof_only_get_elements")?;
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
        self.ensure_no_sub_queries("execute_raw_results_no_proof")?;
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
    /// Refuses a read without a proof of an indexOnly type through an index
    /// that does not cover EVERY property: such a read serializes the
    /// documents it synthesizes, and a projection lacking a property cannot
    /// produce a faithful one. Partial projections only travel the proved read
    /// surface, where the client synthesizes them itself from the proof. The
    /// check includes optional properties (skipIfAbsent skip properties): the
    /// wire encodes absent-vs-present, and a projection that does not carry
    /// an optional property cannot distinguish "absent on the row" from "not
    /// in this index", so serializing it would assert an absence the index
    /// cannot know (and a delete built from it would not match the row).
    /// Every required system property must be one synthesis fills too. Checked
    /// from the schema before any read, by the documents query and by the
    /// chained and composite reads of indexOnly documents, so the answer does
    /// not depend on whether a document matches. Does nothing on any other
    /// type.
    pub(crate) fn refuse_an_uncovered_index_only_projection(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !self.document_type.index_only() {
            return Ok(());
        }
        // By-id and cursor shapes carry dedicated guidance deeper in the route
        // (no primary-key tree; keyset pagination) — let them reach it instead
        // of preempting with the coverage refusal below, which would
        // misdescribe the problem. A clause-free flat scan is classified as a
        // primary-key query, but still synthesizes an index projection and
        // must pass the same coverage check as a filtered query.
        if (self.is_for_primary_key() && !self.index_only_flat_scan_applies())
            || self.start_at.is_some()
        {
            return Ok(());
        }
        let index = self.index_only_query_index(platform_version)?;
        let covers_every_property = self
            .document_type
            .flattened_properties()
            .iter()
            .filter(|(_, property)| {
                !matches!(property.property_type, DocumentPropertyType::Object(_))
            })
            .all(|(name, _)| {
                index.terminal_contains(name)
                    || self.document_type.entry_payload().contains(name.as_str())
                    || index
                        .properties
                        .iter()
                        .any(|index_property| index_property.name == *name)
            });
        // Synthesis fills `$id`, `$ownerId` (every entry index involves it) and
        // `$createdAt` where the index holds it, so any other required system
        // property, or a required `$createdAt` the index lacks, would fail the
        // serialization of every document read: refused here, before the read,
        // so the answer does not depend on whether a document matches
        let covers_required_system_properties = self
            .document_type
            .required_fields()
            .iter()
            .filter(|name| {
                name.starts_with('$')
                    && name.as_str() != document::property_names::ID
                    && name.as_str() != document::property_names::OWNER_ID
            })
            .all(|name| {
                name.as_str() == document::property_names::CREATED_AT
                    && index.involves(document::property_names::CREATED_AT)
            });
        if !covers_required_system_properties {
            return Err(Error::Query(uncovered_required_property_refusal()));
        }
        if covers_every_property {
            return Ok(());
        }
        Err(Error::Query(QuerySyntaxError::Unsupported(
            "this indexOnly query's index does not cover every property, so the documents it \
             synthesizes cannot be serialized into a non-proof response; query through an \
             index covering all properties, or use a proved query"
                .to_string(),
        )))
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
        // indexOnly documents have no stored bodies — the raw elements under
        // the entries are row commitments, not documents. Synthesize the
        // documents from their (path, key) positions and serialize them into
        // the wire shape this path's callers return, once the index is known
        // to cover every property
        // (`refuse_an_uncovered_index_only_projection`).
        {
            use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
            if self.document_type.index_only() {
                self.refuse_an_uncovered_index_only_projection(platform_version)?;
                let (documents, skipped) = self.execute_index_only_documents_no_proof_internal(
                    drive,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;
                let serialized = documents
                    .into_iter()
                    .map(|document| {
                        document
                            .serialize(self.document_type, self.contract, platform_version)
                            .map_err(|error| match index_only_serialization_refusal(&error) {
                                Some(refusal) => Error::Query(refusal),
                                None => error.into(),
                            })
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                return Ok((serialized, skipped));
            }
        }

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
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                Ok((Vec::new(), 0))
            }
            _ => {
                let (data, skipped) = query_result?;
                let (data, cursor_skipped) = self.strip_cursor_from_page(data, platform_version)?;
                Ok((data, skipped.saturating_add(cursor_skipped)))
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
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    GroveError::PathKeyNotFound(_)
                        | GroveError::PathNotFound(_)
                        | GroveError::PathParentLayerNotFound(_)
                ) =>
            {
                Ok((QueryResultElements::new(), 0))
            }
            _ => {
                let (data, skipped) = query_result?;
                let (data, cursor_skipped) =
                    self.strip_cursor_from_elements(data, platform_version)?;
                Ok((data, skipped.saturating_add(cursor_skipped)))
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

    use crate::drive::Drive;
    use crate::query::{
        DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator,
    };
    use crate::util::storage_flags::StorageFlags;

    use dpp::data_contract::DataContract;

    use serde_json::Value::Null;

    use crate::config::DriveConfig;
    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
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
        let platform_version = PlatformVersion::latest();

        let drive = setup_drive(None);

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
        let platform_version = PlatformVersion::latest();

        let drive = setup_drive(None);

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
        let query = DriveDocumentQuery::from_cbor(
            where_cbor.as_slice(),
            &contract,
            document_type,
            &config,
            PlatformVersion::latest(),
        )
        .expect("deserialize cbor shouldn't fail");

        let cbor = query.to_cbor().expect("should serialize cbor");

        let deserialized = DriveDocumentQuery::from_cbor(
            &cbor,
            &contract,
            document_type,
            &config,
            PlatformVersion::latest(),
        )
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
                in_clauses: Vec::new(),
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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
        };

        let path_query = query_asc
            .construct_path_query(None, platform_version)
            .expect("expected to create path query");

        assert_eq!(path_query.to_string(), "PathQuery { path: [@, 0x1da29f488023e306ff9a680bc9837153fb0778c8ee9c934a87dc0de1d69abd3c, 0x01, domain, 0x7265636f7264732e6964656e74697479], query: SizedQuery { query: Query {\n  items: [\n    RangeTo(.. 0x8dc201fd7ad7905f8a84d66218e2b387daea7fe4739ae0e21e8c3ee755e6a2c0),\n  ],\n  default_subquery_branch: SubqueryBranch { subquery_path: [0x00], subquery: Query {\n  items: [\n    RangeFull,\n  ],\n  default_subquery_branch: SubqueryBranch { subquery_path: None subquery: None },\n  left_to_right: false,\n  add_parent_tree_on_subquery: false,\n} },\n  conditional_subquery_branches: {\n    Key(): SubqueryBranch { subquery_path: [0x00], subquery: Query {\n  items: [\n    RangeFull,\n  ],\n  default_subquery_branch: SubqueryBranch { subquery_path: None subquery: None },\n  left_to_right: false,\n  add_parent_tree_on_subquery: false,\n} },\n  },\n  left_to_right: false,\n  add_parent_tree_on_subquery: false,\n}, limit: 6 } }");

        // Serialize the PathQuery to a Vec<u8>
        let encoded = bincode::encode_to_vec(&path_query, bincode::config::standard())
            .expect("Failed to serialize PathQuery");

        // Convert the encoded bytes to a hex string
        let hex_string = hex::encode(encoded);

        // Note: The expected encoding changed due to an upstream GroveDB
        // serialization update. Keep this value in sync with the current
        // GroveDB revision pinned in Cargo.toml.
        assert_eq!(hex_string, "050140201da29f488023e306ff9a680bc9837153fb0778c8ee9c934a87dc0de1d69abd3c010106646f6d61696e107265636f7264732e6964656e74697479010105208dc201fd7ad7905f8a84d66218e2b387daea7fe4739ae0e21e8c3ee755e6a2c00101010001010103000000000001010000010101000101010300000000000000010600");
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
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
            PlatformVersion::latest(),
        )
        .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn resolved_time_range_shape_guard_accepts_only_the_single_resolution_equality() {
        use crate::query::{validate_resolved_time_range_clause_shapes, ResolvedTimeRange};
        use dpp::data_contract::document_type::TimeRangeTransform;

        let resolved = vec![ResolvedTimeRange {
            transform: TimeRangeTransform {
                source: "$createdAt".to_string(),
                range_seconds: 21_600,
                step_seconds: 7_200,
                phase_seconds: 0,
                ttl_seconds: None,
            }
            .into(),
        }];
        let equality = WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::Equal,
            value: Value::U64(21_600_000),
        };
        let other = WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("ibiza".to_string()),
        };

        validate_resolved_time_range_clause_shapes(&[equality.clone(), other.clone()], &resolved)
            .expect("one equality on the resolved field is the resolution shape");

        // An `In` on the resolved field would be fanned out per raw value by
        // the aggregate executors and admitted against bucket keys.
        let in_clause = WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(0), Value::U64(7_200_000)]),
        };
        validate_resolved_time_range_clause_shapes(&[in_clause, other.clone()], &resolved)
            .expect_err("an In clause on a resolved field must be rejected");

        let range_clause = WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(0),
        };
        validate_resolved_time_range_clause_shapes(&[equality.clone(), range_clause], &resolved)
            .expect_err("a range clause riding along on a resolved field must be rejected");

        validate_resolved_time_range_clause_shapes(&[other], &resolved)
            .expect_err("a resolved field with no equality at all must be rejected");
    }

    /// The TTL horizon gate: an expired window may be mid-drainage, so
    /// `byStart` must reject it rather than serve whatever the drain has
    /// left. The boundary is the drain's own predicate — a window starting
    /// exactly at the horizon is not yet expired and stays queryable — and
    /// an index without a `ttl` keeps serving arbitrarily old windows.
    #[test]
    fn by_start_rejects_windows_past_the_ttl_horizon() {
        use crate::query::{resolve_time_range_bucket_clause, TimeRangeSelector};
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;
        use dpp::prelude::Identifier;

        let factory =
            DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
        let hour_ms: u64 = 3_600_000;
        let build = |seed: u8, with_ttl: bool| {
            let mut time_range = vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (Value::Text("range".to_string()), Value::U64(7_200)),
                (Value::Text("step".to_string()), Value::U64(7_200)),
            ];
            if with_ttl {
                time_range.push((Value::Text("ttl".to_string()), Value::U64(14_400)));
            }
            let index_map = vec![
                (
                    Value::Text("name".to_string()),
                    Value::Text("trending".to_string()),
                ),
                (
                    Value::Text("properties".to_string()),
                    Value::Array(vec![
                        platform_value!({"$createdAt": "asc"}),
                        platform_value!({"hashtag": "asc"}),
                    ]),
                ),
                (Value::Text("timeRange".to_string()), Value::Map(time_range)),
                (
                    Value::Text("countable".to_string()),
                    Value::Text("countable".to_string()),
                ),
            ];
            let document_schema = platform_value!({
                "type": "object",
                "properties": {
                    "hashtag": {"type": "string", "maxLength": 61, "position": 0},
                },
                "required": ["hashtag", "$createdAt"],
                "indices": Value::Array(vec![Value::Map(index_map)]),
                "additionalProperties": false,
            });
            factory
                .create_with_value_config(
                    Identifier::from([seed; 32]),
                    0,
                    platform_value!({ "post": document_schema }),
                    None,
                    None,
                )
                .expect("contract registers")
                .data_contract_owned()
        };

        let ttl_contract = build(101, true);
        let standing_contract = build(102, false);
        let expired_start = 5_000 * hour_ms;
        let block_time = expired_start + 6 * hour_ms;

        let resolve = |contract: &DataContract, start_ms: u64| {
            resolve_time_range_bucket_clause(
                "$createdAt",
                TimeRangeSelector::ByStart { start_ms },
                None,
                contract
                    .document_type_for_name("post")
                    .expect("document type"),
                block_time,
            )
        };

        let error = resolve(&ttl_contract, expired_start)
            .expect_err("a window past the ttl horizon must be rejected, not served");
        assert!(
            error.to_string().contains("ttl horizon"),
            "the rejection names the horizon: {error}"
        );
        resolve(&ttl_contract, expired_start + 2 * hour_ms).expect(
            "a window starting exactly at the horizon is not expired — same \
             strictly-below boundary the drain uses",
        );
        resolve(&ttl_contract, expired_start + 4 * hour_ms)
            .expect("a live window resolves normally");
        resolve_time_range_bucket_clause(
            "$createdAt",
            TimeRangeSelector::Newest,
            None,
            ttl_contract
                .document_type_for_name("post")
                .expect("document type"),
            block_time,
        )
        .expect("relative selectors never address expired windows and stay unaffected");
        resolve(&standing_contract, expired_start)
            .expect("without a ttl, arbitrarily old windows stay queryable");
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
                in_clauses: vec![WhereClause {
                    field: "status".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::U64(0),
                        Value::U64(1),
                        Value::U64(2),
                        Value::U64(3),
                        Value::U64(4),
                    ]),
                }],
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
            resolved_time_ranges: vec![],
            sub_queries: vec![],
        };

        // Create a document that we are starting at, which may be missing 'transactionIndex'
        let mut properties = BTreeMap::new();
        properties.insert("status".to_string(), Value::U64(0));
        // We intentionally omit 'transactionIndex' to simulate missing field

        let starts_at_document = DocumentV0 {
            contract_version: None,
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
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
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

    /// Unit coverage for the v1 multi-`In` path-query lowering. These
    /// mirror the storage-backed integration tests in
    /// `tests/query_tests.rs::multi_in_tests`, but exercise the lowering
    /// as the pure function it is (contract in, path query out), so the
    /// selection, validation, and rejection branches are covered by the
    /// lib test target.
    mod multiple_in_clause_lowering {
        use super::*;
        use crate::error::query::QuerySyntaxError;
        use crate::error::Error;

        fn family_contract() -> DataContract {
            json_document_to_contract(
                "tests/supporting_files/contract/family/family-contract.json",
                false,
                PlatformVersion::latest(),
            )
            .expect("expected to load family contract")
        }

        fn text_array(values: &[&str]) -> Value {
            Value::Array(
                values
                    .iter()
                    .map(|value| Value::Text(value.to_string()))
                    .collect(),
            )
        }

        fn in_clause(field: &str, values: &[&str]) -> WhereClause {
            WhereClause {
                field: field.to_string(),
                operator: WhereOperator::In,
                value: text_array(values),
            }
        }

        fn ascending_order_by(fields: &[&str]) -> IndexMap<String, OrderClause> {
            fields
                .iter()
                .map(|field| {
                    (
                        field.to_string(),
                        OrderClause {
                            field: field.to_string(),
                            ascending: true,
                        },
                    )
                })
                .collect()
        }

        fn person_query<'a>(
            contract: &'a DataContract,
            where_clauses: Vec<WhereClause>,
            order_by_fields: &[&str],
        ) -> DriveDocumentQuery<'a> {
            let internal_clauses =
                InternalClauses::extract_from_clauses(where_clauses, PlatformVersion::latest())
                    .expect("clauses should group structurally");
            DriveDocumentQuery {
                contract,
                document_type: contract
                    .document_type_for_name("person")
                    .expect("person document type should exist"),
                internal_clauses,
                offset: None,
                limit: Some(100),
                order_by: ascending_order_by(order_by_fields),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
                resolved_time_ranges: vec![],
                sub_queries: vec![],
            }
        }

        #[test]
        fn two_in_clauses_lower_to_nested_key_sets() {
            let contract = family_contract();
            let platform_version = PlatformVersion::latest();
            let query = person_query(
                &contract,
                vec![
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("lastName", &["Kriskov", "Randolf"]),
                ],
                &["firstName", "lastName"],
            );

            let path_query = query
                .construct_path_query(None, platform_version)
                .expect("two in clauses should lower at protocol version 14");

            // The path descends to the first in field of the
            // [firstName, lastName] index
            assert_eq!(
                path_query.path.last().expect("path should not be empty"),
                &b"firstName".to_vec()
            );

            // Outer level: one key per firstName in value
            let outer = &path_query.query.query;
            assert_eq!(outer.items.len(), 2);
            assert!(outer.left_to_right);

            // Second level: a key set over lastName under the subquery
            // path [lastName]
            assert_eq!(
                outer.default_subquery_branch.subquery_path,
                Some(vec![b"lastName".to_vec()])
            );
            let inner = outer
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a lastName subquery");
            assert_eq!(inner.items.len(), 2);

            // Terminal level: the document id tree under [0]
            assert_eq!(
                inner.default_subquery_branch.subquery_path,
                Some(vec![vec![0]])
            );
        }

        #[test]
        #[cfg(feature = "cbor_query")]
        fn two_in_clauses_survive_cbor_round_trip() {
            let contract = family_contract();
            let mut query = person_query(
                &contract,
                vec![
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("lastName", &["Kriskov", "Randolf"]),
                ],
                &["firstName", "lastName"],
            );
            // `from_cbor` defaults start_at_included to true when no cursor
            // is present; align so the round trip compares equal
            query.start_at_included = true;

            let cbor = query.to_cbor().expect("should serialize cbor");
            let deserialized = DriveDocumentQuery::from_cbor(
                &cbor,
                &contract,
                contract
                    .document_type_for_name("person")
                    .expect("person document type should exist"),
                &DriveConfig::default(),
                PlatformVersion::latest(),
            )
            .expect("should deserialize cbor");

            assert_eq!(query, deserialized);
            assert_eq!(
                deserialized
                    .internal_clauses
                    .in_clauses
                    .iter()
                    .map(|in_clause| in_clause.field.as_str())
                    .collect::<Vec<_>>(),
                vec!["firstName", "lastName"],
                "both in clauses must survive the round trip in order"
            );
        }

        #[test]
        fn descending_order_by_on_left_over_property_is_honored() {
            let contract = family_contract();
            let platform_version = PlatformVersion::latest();
            // [firstName, middleName, lastName]: two in levels, lastName
            // left over with an explicit descending order
            let mut query = person_query(
                &contract,
                vec![
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("middleName", &["Ivanna", "Evangeline"]),
                ],
                &["firstName", "middleName"],
            );
            query.order_by.insert(
                "lastName".to_string(),
                OrderClause {
                    field: "lastName".to_string(),
                    ascending: false,
                },
            );

            let path_query = query
                .construct_path_query(None, platform_version)
                .expect("two in clauses with a left-over order should lower");

            let outer = &path_query.query.query;
            let middle = outer
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a middleName subquery");
            assert_eq!(
                middle.default_subquery_branch.subquery_path,
                Some(vec![b"lastName".to_vec()])
            );
            let left_over_level = middle
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a lastName subquery");
            assert!(
                !left_over_level.left_to_right,
                "left-over lastName level must honor the descending order by"
            );

            // Without an order by entry the level falls back to the index
            // property's direction (ascending)
            query.order_by.shift_remove("lastName");
            let path_query = query
                .construct_path_query(None, platform_version)
                .expect("two in clauses should lower");
            let left_over_level = path_query
                .query
                .query
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a middleName subquery")
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a lastName subquery");
            assert!(left_over_level.left_to_right);
        }

        #[test]
        fn two_in_clauses_rejected_at_protocol_version_13() {
            let contract = family_contract();
            let platform_version_13 =
                PlatformVersion::get(13).expect("protocol version 13 should exist");
            let query = person_query(
                &contract,
                vec![
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("lastName", &["Kriskov", "Randolf"]),
                ],
                &["firstName", "lastName"],
            );

            let error = query
                .construct_path_query(None, platform_version_13)
                .expect_err("multiple in clauses must be rejected before protocol version 14");
            assert!(
                matches!(error, Error::Query(QuerySyntaxError::MultipleInClauses(_))),
                "expected MultipleInClauses, got {error:?}"
            );

            query
                .construct_path_query(None, PlatformVersion::latest())
                .expect("the same query should lower at protocol version 14");
        }

        #[test]
        fn equality_prefix_two_in_clauses_and_trailing_range_lowering() {
            let contract = family_contract();
            let platform_version = PlatformVersion::latest();
            let mut query = person_query(
                &contract,
                vec![
                    WhereClause {
                        field: "age".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::U8(30),
                    },
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("middleName", &["Ivanna", "Evangeline"]),
                    WhereClause {
                        field: "lastName".to_string(),
                        operator: WhereOperator::GreaterThan,
                        value: Value::Text("M".to_string()),
                    },
                ],
                &["firstName", "middleName", "lastName"],
            );
            query.limit = Some(50);

            // Matches the [age, firstName, middleName, lastName] index:
            // equality prefix on age, then two consecutive in levels, then
            // the range level
            let path_query = query
                .construct_path_query(None, platform_version)
                .expect("equality + in + in + range should lower");

            let path_len = path_query.path.len();
            assert_eq!(path_query.path[path_len - 3], b"age".to_vec());
            assert_eq!(
                path_query.path.last().expect("path should not be empty"),
                &b"firstName".to_vec()
            );

            let outer = &path_query.query.query;
            assert_eq!(outer.items.len(), 2);
            assert_eq!(
                outer.default_subquery_branch.subquery_path,
                Some(vec![b"middleName".to_vec()])
            );
            let middle = outer
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a middleName subquery");
            assert_eq!(middle.items.len(), 2);
            assert_eq!(
                middle.default_subquery_branch.subquery_path,
                Some(vec![b"lastName".to_vec()])
            );
            let range_level = middle
                .default_subquery_branch
                .subquery
                .as_deref()
                .expect("expected a lastName subquery");
            // The trailing range is a single range item, not a key set
            assert_eq!(range_level.items.len(), 1);
            assert_eq!(
                range_level.default_subquery_branch.subquery_path,
                Some(vec![vec![0]])
            );
        }

        #[test]
        fn cross_product_above_cap_is_rejected() {
            let contract = family_contract();
            let first_names: Vec<String> = (0..20).map(|i| format!("First{i:02}")).collect();
            let last_names: Vec<String> = (0..6).map(|i| format!("Last{i}")).collect();
            let query = person_query(
                &contract,
                vec![
                    WhereClause {
                        field: "firstName".to_string(),
                        operator: WhereOperator::In,
                        value: Value::Array(first_names.iter().cloned().map(Value::Text).collect()),
                    },
                    WhereClause {
                        field: "lastName".to_string(),
                        operator: WhereOperator::In,
                        value: Value::Array(last_names.iter().cloned().map(Value::Text).collect()),
                    },
                ],
                &["firstName", "lastName"],
            );

            let error = query
                .construct_path_query(None, PlatformVersion::latest())
                .expect_err("a 120-branch cross product must be rejected");
            assert!(
                matches!(error, Error::Query(QuerySyntaxError::InvalidInClause(_))),
                "expected InvalidInClause, got {error:?}"
            );
        }

        #[test]
        fn non_consecutive_in_fields_are_rejected() {
            let contract = family_contract();
            // [firstName, middleName, lastName] holds middleName and
            // lastName at positions 1 and 2 with no equality on firstName,
            // so no index conforms
            let query = person_query(
                &contract,
                vec![
                    in_clause("middleName", &["Ivanna", "Evangeline"]),
                    in_clause("lastName", &["Kriskov", "Randolf"]),
                ],
                &["middleName", "lastName"],
            );

            let error = query
                .construct_path_query(None, PlatformVersion::latest())
                .expect_err("non-consecutive in clauses must be rejected");
            assert!(
                matches!(
                    error,
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
                ),
                "expected WhereClauseOnNonIndexedProperty, got {error:?}"
            );
        }

        #[test]
        fn cursor_pagination_is_rejected() {
            let contract = family_contract();
            let mut query = person_query(
                &contract,
                vec![
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("lastName", &["Kriskov", "Randolf"]),
                ],
                &["firstName", "lastName"],
            );
            query.start_at = Some([5u8; 32]);
            query.start_at_included = false;

            let error = query
                .construct_path_query(None, PlatformVersion::latest())
                .expect_err("cursor pagination with multiple in clauses must be rejected");
            assert!(
                matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
                "expected Unsupported, got {error:?}"
            );
        }

        #[test]
        fn missing_order_by_on_an_in_field_is_rejected() {
            let contract = family_contract();
            let query = person_query(
                &contract,
                vec![
                    in_clause("firstName", &["Adey", "Briney"]),
                    in_clause("lastName", &["Kriskov", "Randolf"]),
                ],
                &["firstName"],
            );

            let error = query
                .construct_path_query(None, PlatformVersion::latest())
                .expect_err("missing order by on an in field must be rejected");
            // Index selection rejects the shape first: the order-by
            // continuity rule in `Index::matches` disqualifies every
            // candidate index before the per-field `MissingOrderByForRange`
            // guard could fire
            assert!(
                matches!(
                    error,
                    Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
                ),
                "expected WhereClauseOnNonIndexedProperty, got {error:?}"
            );
        }
    }
}
