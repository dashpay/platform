//! Method to query documents from the Drive.

use std::sync::Arc;

use crate::{error::Error, sdk::Sdk};
use dapi_grpc::platform::v0::get_documents_request::Version::V1;
use dapi_grpc::platform::v0::{
    self as platform_proto,
    get_documents_request::{
        document_field_value,
        get_documents_request_v0::Start,
        get_documents_request_v1::{select, Select as ProtoSelect, Start as V1Start},
        having_aggregate, having_clause, having_ranking,
        DocumentFieldValue as ProtoDocumentFieldValue, GetDocumentsRequestV1,
        HavingAggregate as ProtoHavingAggregate, HavingClause as ProtoHavingClause,
        HavingRanking as ProtoHavingRanking, OrderClause as ProtoOrderClause,
        WhereClause as ProtoWhereClause, WhereOperator as ProtoWhereOperator,
    },
    GetDocumentsRequest, Proof, ResponseMetadata,
};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    document::Document,
    platform_value::{platform_value, Value},
    prelude::{DataContract, Identifier},
    InvalidVectorSizeError, ProtocolError,
};
use drive::query::{
    DriveDocumentQuery, HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator,
    HavingRanking, HavingRankingKind, HavingRightOperand, InternalClauses, OrderClause,
    SelectFunction, SelectProjection, WhereClause, WhereOperator,
};
use drive_proof_verifier::{types::Documents, FromProof};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};

use crate::platform::Fetch;

// TODO: remove DocumentQuery once ContextProvider that provides data contracts is merged.

/// Request that is used to query documents from the Dash Platform.
///
/// This is an abstraction layer built on top of [GetDocumentsRequest] to address issues with missing details
/// required to correctly verify proofs returned by the Dash Platform.
///
/// Conversions are implemented between this type, [GetDocumentsRequest] and [DriveDocumentQuery] using [TryFrom] trait.
#[derive(Debug, Clone, PartialEq, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentQuery {
    /// SQL-shaped `SELECT` projection — `(function, field)` pair.
    /// `Documents` returns matched rows; `Count` / `Sum` / `Avg`
    /// return either a single aggregate (empty `group_by`) or
    /// per-group entries (non-empty `group_by`). Defaults to
    /// `SelectProjection::documents()` so callers that don't opt
    /// into the SQL-shaped surface get plain document-fetch
    /// semantics.
    ///
    /// `#[serde(default)]` here (and on `group_by` / `having`
    /// below) is wire-format-compat for mock vectors captured
    /// before the SQL-shaped surface was added: default
    /// `SelectProjection` is `documents()`, `Vec` defaults to
    /// empty — together those mean an old fixture without these
    /// fields deserializes to the documents-fetch shape it was
    /// originally captured under. New fixtures should serialize
    /// the fields explicitly.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub select: SelectProjection,
    /// Data contract
    pub data_contract: Arc<DataContract>,
    /// Document type for the data contract
    pub document_type_name: String,
    /// `where` clauses for the query
    pub where_clauses: Vec<WhereClause>,
    /// SQL `GROUP BY` field names, in left-to-right order. Empty =
    /// no explicit grouping (aggregate count for `select=Count`).
    /// Only meaningful when `select=Count`; non-empty with
    /// `select=Documents` is rejected by the server as unsupported.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub group_by: Vec<String>,
    /// SQL `HAVING` clauses — aggregate filters that apply to the
    /// grouped rows produced by `select = Count`, `group_by =
    /// […]`. Unlike `where_clauses`, the left side is an aggregate
    /// (`COUNT(*)`, `SUM(field)`, `AVG(field)`, `MIN`/`MAX`,
    /// `TOP`/`BOTTOM` for N-th-element selection) rather than a
    /// raw row field. See [`HavingClause`] /
    /// [`drive::query::HavingAggregate`] /
    /// [`drive::query::HavingOperator`] for the catalogs. Multiple
    /// entries combine with implicit `AND`.
    ///
    /// Non-empty values are rejected by the server today with
    /// `QuerySyntaxError::Unsupported("HAVING clause is not yet
    /// implemented")` — the typed builder exists so callers can
    /// encode the full aggregate-filter surface ahead of server
    /// support landing without a wire-format change.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub having: Vec<HavingClause>,
    /// `order_by` clauses for the query
    pub order_by_clauses: Vec<OrderClause>,
    /// queryset limit. `0` is the sentinel for "unset / default" and
    /// is translated to `None` on the V1 wire (`optional uint32`).
    pub limit: u32,
    /// first object to start with
    pub start: Option<Start>,
}

impl DocumentQuery {
    /// Create new DocumentQuery for provided contract and document type name.
    pub fn new<C: Into<Arc<DataContract>>>(
        contract: C,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        let contract = contract.into();
        // ensure document type name is correct
        contract
            .document_type_for_name(document_type_name)
            .map_err(ProtocolError::DataContractError)?;

        Ok(Self {
            select: SelectProjection::documents(),
            data_contract: Arc::clone(&contract),
            document_type_name: document_type_name.to_string(),
            where_clauses: vec![],
            group_by: Vec::new(),
            having: Vec::new(),
            order_by_clauses: vec![],
            limit: 0,
            start: None,
        })
    }

    /// Create new document query based on a [DriveDocumentQuery].
    pub fn new_with_drive_query(d: &DriveDocumentQuery) -> Self {
        Self::from(d)
    }

    /// Create new document query for provided document type name and data contract ID.
    ///
    /// Note that this method will fetch data contract first.
    pub async fn new_with_data_contract_id(
        api: &Sdk,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        let data_contract =
            DataContract::fetch(api, data_contract_id)
                .await?
                .ok_or(Error::MissingDependency(
                    "DataContract".to_string(),
                    format!("data contract {} not found", data_contract_id),
                ))?;

        Self::new(data_contract, document_type_name)
    }

    /// Point to a specific document ID.
    pub fn with_document_id(self, document_id: &Identifier) -> Self {
        let clause = WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(document_id),
        };

        self.with_where(clause)
    }

    /// Add new where clause to the query.
    ///
    /// Existing where clauses will be preserved.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.where_clauses.push(clause);

        self
    }

    /// Add order by clause to the query.
    ///
    /// Existing order by clauses will be preserved.
    pub fn with_order_by(mut self, clause: OrderClause) -> Self {
        self.order_by_clauses.push(clause);

        self
    }

    /// Set the SQL-shaped `SELECT` projection.
    ///
    /// Construct the [`SelectProjection`] via its helpers:
    /// [`SelectProjection::documents`] (the default — matched
    /// rows), [`SelectProjection::count_star`] for `COUNT(*)`,
    /// [`SelectProjection::count_field`] for `COUNT(field)`,
    /// [`SelectProjection::sum`] for `SUM(field)`,
    /// [`SelectProjection::avg`] for `AVG(field)`. Pair the
    /// count/sum/avg projections with [`DocumentCount::fetch`]
    /// (single aggregate, empty `group_by`) or
    /// [`DocumentSplitCounts::fetch`] (per-group entries,
    /// non-empty `group_by`).
    ///
    /// `SUM` / `AVG` and `COUNT(field)` are accepted by the SDK
    /// but the server rejects them today with `Unsupported("…
    /// is not yet implemented")` — the surface is shipped first
    /// and execution lands later.
    pub fn with_select(mut self, select: SelectProjection) -> Self {
        self.select = select;
        self
    }

    /// Set the `GROUP BY` field to a single field name.
    ///
    /// Convenience wrapper around [`Self::with_group_by_fields`].
    /// Replaces any previously set `group_by`. Pair with
    /// [`Self::with_select`] (e.g.
    /// `with_select(SelectProjection::count_star())`) for the
    /// per-group entries shape.
    pub fn with_group_by<S: Into<String>>(mut self, field: S) -> Self {
        self.group_by = vec![field.into()];
        self
    }

    /// Set the full `GROUP BY` field list (replaces any previously
    /// set `group_by`).
    ///
    /// Multi-field `group_by` is only accepted by the server for
    /// `(in_field, range_field)` matching a compound `In + range`
    /// where clause against a `rangeCountable: true` index. Other
    /// non-empty shapes return `QuerySyntaxError::Unsupported`.
    pub fn with_group_by_fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.group_by = fields.into_iter().map(Into::into).collect();
        self
    }

    /// Set the `HAVING` clauses (replaces any prior value).
    ///
    /// Non-empty values are rejected by the server with
    /// `QuerySyntaxError::Unsupported("HAVING clause is not yet
    /// implemented")`. The builder exists so SDK callers can
    /// encode `HAVING` ahead of server support landing without
    /// another version bump.
    pub fn with_having(mut self, having: Vec<HavingClause>) -> Self {
        self.having = having;
        self
    }

    /// Set the query limit. `0` means "unset" — translated to
    /// `None` on the V1 wire (the proto field is `optional uint32`).
    ///
    /// On `select=Count` with non-empty `group_by` against the
    /// prove path, the server validates rather than clamps:
    /// `limit > max_query_limit` is rejected with
    /// `InvalidLimit` rather than silently truncated, since
    /// clamping would invisibly break proof verification.
    /// Leaving the limit unset (`0`) falls back to
    /// `drive::config::DEFAULT_QUERY_LIMIT` on the proof verifier
    /// side, keeping proof bytes deterministic across operators.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }
}

impl TransportRequest for DocumentQuery {
    type Client = <GetDocumentsRequest as TransportRequest>::Client;
    type Response = <GetDocumentsRequest as TransportRequest>::Response;
    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings =
        <GetDocumentsRequest as TransportRequest>::SETTINGS_OVERRIDES;

    fn request_name(&self) -> &'static str {
        "GetDocumentsRequest"
    }

    fn method_name(&self) -> &'static str {
        "get_documents"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let request: GetDocumentsRequest = self
            .try_into()
            .expect("DocumentQuery should always be valid");
        request.execute_transport(client, settings)
    }
}

impl FromProof<DocumentQuery> for Document {
    type Request = DocumentQuery;
    type Response = platform_proto::GetDocumentsResponse;
    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();

        let (documents, metadata, proof): (Option<Documents>, ResponseMetadata, Proof) =
            <Documents as FromProof<Self::Request>>::maybe_from_proof_with_metadata(
                request,
                response,
                network,
                platform_version,
                provider,
            )?;

        match documents {
            None => Ok((None, metadata, proof)),
            Some(docs) => match docs.len() {
                0 | 1 => Ok((
                    docs.into_iter().next().and_then(|(_, v)| v),
                    metadata,
                    proof,
                )),
                n => Err(drive_proof_verifier::Error::ResponseDecodeError {
                    error: format!("expected 1 element, got {}", n),
                }),
            },
        }
    }
}

impl FromProof<DocumentQuery> for drive_proof_verifier::types::Documents {
    type Request = DocumentQuery;
    type Response = platform_proto::GetDocumentsResponse;
    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let drive_query: DriveDocumentQuery =
            (&request)
                .try_into()
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!("Failed to convert DocumentQuery to DriveQuery: {}", e),
                })?;

        <drive_proof_verifier::types::Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
            drive_query,
            response,
            network,
            platform_version,
            provider,
        )
    }
}

impl TryFrom<DocumentQuery> for platform_proto::GetDocumentsRequest {
    type Error = Error;
    fn try_from(dapi_request: DocumentQuery) -> Result<Self, Self::Error> {
        // `try_from` owns `dapi_request` — destructure once and
        // consume the owned vectors below (no `.clone()` per field).
        let DocumentQuery {
            select,
            data_contract,
            document_type_name,
            where_clauses,
            group_by,
            having,
            order_by_clauses,
            limit,
            start,
        } = dapi_request;

        let where_clauses = where_clauses
            .into_iter()
            .map(where_clause_to_proto)
            .collect::<Result<Vec<_>, _>>()?;
        let order_by = order_by_clauses
            .into_iter()
            .map(order_clause_to_proto)
            .collect();
        let having = having
            .into_iter()
            .map(having_clause_to_proto)
            .collect::<Result<Vec<_>, _>>()?;
        // `limit: u32` with `0` sentinel → `optional uint32` on the
        // V1 wire. `None` lets the server apply its own default;
        // explicit `0` would be a strange "return zero rows" request.
        let limit = if limit == 0 { None } else { Some(limit) };
        // V0 and V1 ship separate `Start` enums even though the
        // shape is identical. Translate at the wire boundary so the
        // `DocumentQuery.start` field stays stable for callers
        // already using the V0 type.
        let start_v1 = start.map(|s| match s {
            Start::StartAfter(b) => V1Start::StartAfter(b),
            Start::StartAt(b) => V1Start::StartAt(b),
        });

        //todo: transform this into PlatformVersionedTryFrom
        Ok(GetDocumentsRequest {
            version: Some(V1(GetDocumentsRequestV1 {
                data_contract_id: data_contract.id().to_vec(),
                document_type: document_type_name,
                where_clauses,
                order_by,
                limit,
                // Document fetch always proves via this conversion.
                // Count fetch uses the same wire shape; both paths
                // go through the `FromProof` decoders which expect
                // the `Proof(...)` response variant. `SdkBuilder::
                // with_proofs(false)` is consequently a no-op for
                // both — see the blanket `Query<T> for T` impl in
                // `packages/rs-sdk/src/platform/query.rs` for the
                // `tracing::warn!` emitted at fetch time when proofs
                // are disabled.
                prove: true,
                start: start_v1,
                select: Some(select_to_proto(select)),
                group_by,
                having,
            })),
        })
    }
}

impl<'a> From<&'a DriveDocumentQuery<'a>> for DocumentQuery {
    fn from(value: &'a DriveDocumentQuery<'a>) -> Self {
        let data_contract = value.contract.clone();
        let document_type_name = value.document_type.name();
        let where_clauses = value.internal_clauses.clone().into();
        let order_by_clauses = value.order_by.iter().map(|(_, v)| v.clone()).collect();
        let limit = value.limit.unwrap_or(0) as u32;

        let start = if let Some(start_at) = value.start_at {
            match value.start_at_included {
                true => Some(Start::StartAt(start_at.to_vec())),
                false => Some(Start::StartAfter(start_at.to_vec())),
            }
        } else {
            None
        };

        Self {
            // `DriveDocumentQuery` has no SELECT/GROUP BY/HAVING
            // concept — it's a documents-only query. Default to the
            // v1 documents shape.
            select: SelectProjection::documents(),
            data_contract: Arc::new(data_contract),
            document_type_name: document_type_name.to_string(),
            where_clauses,
            group_by: Vec::new(),
            having: Vec::new(),
            order_by_clauses,
            limit,
            start,
        }
    }
}

impl<'a> From<DriveDocumentQuery<'a>> for DocumentQuery {
    fn from(value: DriveDocumentQuery<'a>) -> Self {
        let data_contract = value.contract.clone();
        let document_type_name = value.document_type.name();
        let where_clauses = value.internal_clauses.clone().into();
        let order_by_clauses = value.order_by.iter().map(|(_, v)| v.clone()).collect();
        let limit = value.limit.unwrap_or(0) as u32;

        let start = if let Some(start_at) = value.start_at {
            match value.start_at_included {
                true => Some(Start::StartAt(start_at.to_vec())),
                false => Some(Start::StartAfter(start_at.to_vec())),
            }
        } else {
            None
        };

        Self {
            // `DriveDocumentQuery` has no SELECT/GROUP BY/HAVING
            // concept — it's a documents-only query. Default to the
            // v1 documents shape.
            select: SelectProjection::documents(),
            data_contract: Arc::new(data_contract),
            document_type_name: document_type_name.to_string(),
            where_clauses,
            group_by: Vec::new(),
            having: Vec::new(),
            order_by_clauses,
            limit,
            start,
        }
    }
}

impl<'a> TryFrom<&'a DocumentQuery> for DriveDocumentQuery<'a> {
    type Error = crate::error::Error;

    fn try_from(request: &'a DocumentQuery) -> Result<Self, Self::Error> {
        // let data_contract = request.data_contract.clone();
        let document_type = request
            .data_contract
            .document_type_for_name(&request.document_type_name)
            .map_err(ProtocolError::DataContractError)?;

        let internal_clauses = InternalClauses::extract_from_clauses(request.where_clauses.clone())
            .map_err(Error::Drive)?;

        let limit = if request.limit != 0 {
            Some(request.limit as u16)
        } else {
            None
        };

        let (start_at, start_at_included) = match request.start.as_ref() {
            None => (None, false),
            Some(Start::StartAt(at)) => (
                Some(at.clone().try_into().map_err(|_| {
                    ProtocolError::InvalidVectorSizeError(InvalidVectorSizeError::new(32, at.len()))
                })?),
                true,
            ),
            Some(Start::StartAfter(after)) => (
                Some(after.clone().try_into().map_err(|_| {
                    ProtocolError::InvalidVectorSizeError(InvalidVectorSizeError::new(
                        32,
                        after.len(),
                    ))
                })?),
                false,
            ),
        };

        let query = Self {
            contract: &request.data_contract,
            document_type,
            internal_clauses,
            offset: None,
            limit,
            order_by: request
                .order_by_clauses
                .clone()
                .into_iter()
                .map(|v| (v.field.clone(), v))
                .collect(),
            start_at,
            start_at_included,
            block_time_ms: None,
        };

        Ok(query)
    }
}

/// Convert a drive [`WhereClause`] into its wire-format proto
/// counterpart. The proto value variant is picked from the
/// `dpp::platform_value::Value` variant by primitive type — schema-
/// agnostic, matching the inverse direction the rs-drive-abci v1
/// handler runs via its `conversions::value_from_proto`.
///
/// Errors only on `Value` variants that have no wire-format
/// counterpart (`Map`, `EnumU8`, `EnumString`) — these aren't
/// produced by the SDK's typical WhereClause builders, so a
/// rejection here flags an unsupported caller construction at the
/// wire boundary rather than silently dropping the value.
fn where_clause_to_proto(clause: WhereClause) -> Result<ProtoWhereClause, Error> {
    Ok(ProtoWhereClause {
        field: clause.field,
        operator: where_operator_to_proto(clause.operator) as i32,
        value: Some(value_to_proto(clause.value)?),
    })
}

fn order_clause_to_proto(clause: OrderClause) -> ProtoOrderClause {
    ProtoOrderClause {
        field: clause.field,
        ascending: clause.ascending,
    }
}

/// Convert a drive [`HavingClause`] into its wire-format proto
/// counterpart. The inverse of `rs-drive-abci`'s
/// `having_clause_from_proto`. Errors only on `Value` variants
/// the underlying `value_to_proto` can't represent — every
/// `HavingOperator` / `HavingAggregateFunction` /
/// `HavingRankingKind` discriminant has a 1:1 wire counterpart
/// and is always convertible.
fn having_clause_to_proto(clause: HavingClause) -> Result<ProtoHavingClause, Error> {
    let right = match clause.right {
        HavingRightOperand::Value(v) => having_clause::Right::Value(value_to_proto(v)?),
        HavingRightOperand::Ranking(r) => having_clause::Right::Ranking(having_ranking_to_proto(r)),
    };
    Ok(ProtoHavingClause {
        aggregate: Some(having_aggregate_to_proto(clause.aggregate)),
        operator: having_operator_to_proto(clause.operator) as i32,
        right: Some(right),
    })
}

fn having_aggregate_to_proto(aggregate: HavingAggregate) -> ProtoHavingAggregate {
    ProtoHavingAggregate {
        function: having_function_to_proto(aggregate.function) as i32,
        field: aggregate.field,
    }
}

fn having_function_to_proto(function: HavingAggregateFunction) -> having_aggregate::Function {
    match function {
        HavingAggregateFunction::Count => having_aggregate::Function::Count,
        HavingAggregateFunction::Sum => having_aggregate::Function::Sum,
        HavingAggregateFunction::Avg => having_aggregate::Function::Avg,
    }
}

fn having_ranking_to_proto(ranking: HavingRanking) -> ProtoHavingRanking {
    ProtoHavingRanking {
        kind: having_ranking_kind_to_proto(ranking.kind) as i32,
        n: ranking.n,
    }
}

fn having_ranking_kind_to_proto(kind: HavingRankingKind) -> having_ranking::Kind {
    match kind {
        HavingRankingKind::Min => having_ranking::Kind::Min,
        HavingRankingKind::Max => having_ranking::Kind::Max,
        HavingRankingKind::Top => having_ranking::Kind::Top,
        HavingRankingKind::Bottom => having_ranking::Kind::Bottom,
    }
}

/// Convert a drive [`SelectProjection`] into its wire-format
/// proto counterpart. Inverse of `rs-drive-abci`'s
/// `select_from_proto`. Always succeeds — every
/// `SelectFunction` discriminant has a 1:1 wire counterpart.
fn select_to_proto(select: SelectProjection) -> ProtoSelect {
    ProtoSelect {
        function: select_function_to_proto(select.function) as i32,
        field: select.field,
    }
}

fn select_function_to_proto(function: SelectFunction) -> select::Function {
    match function {
        SelectFunction::Documents => select::Function::Documents,
        SelectFunction::Count => select::Function::Count,
        SelectFunction::Sum => select::Function::Sum,
        SelectFunction::Avg => select::Function::Avg,
    }
}

fn having_operator_to_proto(op: HavingOperator) -> having_clause::Operator {
    match op {
        HavingOperator::Equal => having_clause::Operator::Equal,
        HavingOperator::NotEqual => having_clause::Operator::NotEqual,
        HavingOperator::GreaterThan => having_clause::Operator::GreaterThan,
        HavingOperator::GreaterThanOrEquals => having_clause::Operator::GreaterThanOrEquals,
        HavingOperator::LessThan => having_clause::Operator::LessThan,
        HavingOperator::LessThanOrEquals => having_clause::Operator::LessThanOrEquals,
        HavingOperator::Between => having_clause::Operator::Between,
        HavingOperator::BetweenExcludeBounds => having_clause::Operator::BetweenExcludeBounds,
        HavingOperator::BetweenExcludeLeft => having_clause::Operator::BetweenExcludeLeft,
        HavingOperator::BetweenExcludeRight => having_clause::Operator::BetweenExcludeRight,
        HavingOperator::In => having_clause::Operator::In,
    }
}

fn where_operator_to_proto(op: WhereOperator) -> ProtoWhereOperator {
    match op {
        WhereOperator::Equal => ProtoWhereOperator::Equal,
        WhereOperator::GreaterThan => ProtoWhereOperator::GreaterThan,
        WhereOperator::GreaterThanOrEquals => ProtoWhereOperator::GreaterThanOrEquals,
        WhereOperator::LessThan => ProtoWhereOperator::LessThan,
        WhereOperator::LessThanOrEquals => ProtoWhereOperator::LessThanOrEquals,
        WhereOperator::Between => ProtoWhereOperator::Between,
        WhereOperator::BetweenExcludeBounds => ProtoWhereOperator::BetweenExcludeBounds,
        WhereOperator::BetweenExcludeLeft => ProtoWhereOperator::BetweenExcludeLeft,
        WhereOperator::BetweenExcludeRight => ProtoWhereOperator::BetweenExcludeRight,
        WhereOperator::In => ProtoWhereOperator::In,
        WhereOperator::StartsWith => ProtoWhereOperator::StartsWith,
    }
}

/// Map `dpp::platform_value::Value` onto the wire-shape
/// [`ProtoDocumentFieldValue`]. The schema-driven decode on the
/// server side resolves the actual indexed type — this layer just
/// names the primitive.
///
/// Mapping rules:
/// - `Bool` → `BoolValue`
/// - `I8`/`I16`/`I32`/`I64` → `Int64Value` (widened)
/// - `U8`/`U16`/`U32`/`U64` → `Uint64Value` (widened)
/// - `Float` → `DoubleValue`
/// - `Text` → `Text`
/// - `Bytes`/`Bytes20`/`Bytes32`/`Bytes36`/`Identifier` → `BytesValue`
/// - `U128`/`I128` → `Text` (decimal string; the server decodes
///   against the indexed `U128`/`I128` field type)
/// - `Array` → `List` (recursive — operand for `IN` / `BETWEEN*`)
/// - `Null` → `NullValue(true)` (the `bool` payload is a
///   placeholder per the proto-side comment; only the variant
///   discriminant carries meaning)
/// - `Map`/`EnumU8`/`EnumString` → `Error` (no wire-format
///   counterpart for these shapes in a WhereClause operand)
fn value_to_proto(value: Value) -> Result<ProtoDocumentFieldValue, Error> {
    let variant = match value {
        Value::Null => document_field_value::Variant::NullValue(true),
        Value::Bool(b) => document_field_value::Variant::BoolValue(b),
        Value::I8(i) => document_field_value::Variant::Int64Value(i as i64),
        Value::I16(i) => document_field_value::Variant::Int64Value(i as i64),
        Value::I32(i) => document_field_value::Variant::Int64Value(i as i64),
        Value::I64(i) => document_field_value::Variant::Int64Value(i),
        Value::U8(u) => document_field_value::Variant::Uint64Value(u as u64),
        Value::U16(u) => document_field_value::Variant::Uint64Value(u as u64),
        Value::U32(u) => document_field_value::Variant::Uint64Value(u as u64),
        Value::U64(u) => document_field_value::Variant::Uint64Value(u),
        Value::Float(f) => document_field_value::Variant::DoubleValue(f),
        Value::Text(s) => document_field_value::Variant::Text(s),
        Value::Bytes(b) => document_field_value::Variant::BytesValue(b),
        Value::Bytes20(b) => document_field_value::Variant::BytesValue(b.to_vec()),
        Value::Bytes32(b) => document_field_value::Variant::BytesValue(b.to_vec()),
        Value::Bytes36(b) => document_field_value::Variant::BytesValue(b.to_vec()),
        Value::Identifier(b) => document_field_value::Variant::BytesValue(b.to_vec()),
        // u128 / i128 don't fit in `int64_value`/`uint64_value`;
        // encode as a decimal string. The server's schema-driven
        // decode path accepts text against U128/I128 fields.
        Value::U128(u) => document_field_value::Variant::Text(u.to_string()),
        Value::I128(i) => document_field_value::Variant::Text(i.to_string()),
        Value::Array(items) => {
            document_field_value::Variant::List(document_field_value::ValueList {
                values: items
                    .into_iter()
                    .map(value_to_proto)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        // Catches both `Value::Map(_)` / `Value::EnumU8(_)` /
        // `Value::EnumString(_)` (no wire-format counterpart for
        // these shapes in a WhereClause operand) and any
        // future-added variant — `dpp::platform_value::Value` is
        // `#[non_exhaustive]`, so the SDK fails loudly rather
        // than silently dropping data the moment upstream adds a
        // variant we don't yet know how to encode.
        _ => {
            return Err(Error::Protocol(dpp::ProtocolError::EncodingError(format!(
                "Value variant has no `DocumentFieldValue` wire-format counterpart: {value:?}"
            ))));
        }
    };
    Ok(ProtoDocumentFieldValue {
        variant: Some(variant),
    })
}
