//! Request-driven document proof verification.
//!
//! [`FromProof<GetDocumentsRequest>`] reconstructs the [`DriveDocumentQuery`]
//! an honest server ran from the *wire request bytes that were actually sent*,
//! then verifies the proved response against it. This is the entry point for
//! embedders that own their transport (the Dash Core platform GUI, explorers):
//! their request only ever exists as protobuf bytes, so the verifier must
//! rebuild the query the same way the server did.
//!
//! The reconstruction runs the server's own pipeline:
//! - the wire decode is the shared `platform_query_wire` decoder rs-drive-abci
//!   decodes incoming requests with, so server and verifier cannot drift on
//!   clause interpretation;
//! - the lowering into a [`DriveDocumentQuery`] is rs-drive's
//!   [`DriveDocumentQuery::from_typed_clauses`] under the default
//!   [`DriveConfig`] (the limit contract every deployed server runs with) and
//!   the platform version the response was produced under;
//! - request shapes the server would never have answered with a plain proved
//!   document set (aggregate projections, `HAVING`, `GROUP BY`, `OFFSET`,
//!   `prove = false`, a wire version outside the served bounds) are rejected
//!   up front, since a proof can never belong to them.

use crate::from_request::TryFromRequest;
use crate::types::Documents;
use crate::{ContextProvider, Error, FromProof, Length};
use dapi_grpc::platform::v0::get_documents_request::{
    get_documents_request_v0::Start as V0Start, get_documents_request_v1::Start as V1Start,
    GetDocumentsRequestV0, GetDocumentsRequestV1, Version,
};
use dapi_grpc::platform::v0::{GetDocumentsRequest, GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::PlatformVersion;
use drive::config::DriveConfig;
use drive::query::{
    resolve_time_range_bucket_clause, DriveDocumentQuery, OrderClause, ResolvedTimeRange,
    SelectProjection, TimeRangeGridSpec, TimeRangeSelector, WhereClause,
};
use platform_query_wire::proto_conversions as wire;
use std::sync::Arc;

/// The [`GetDocumentsRequest`] fields a proved plain-document response is
/// verified against, decoded off the wire but not yet bound to a contract.
///
/// Built by [`TryFromRequest`] from either wire version. The data contract is
/// resolved separately (through the [`ContextProvider`]) because a
/// [`DriveDocumentQuery`] borrows it.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentWireQuery {
    /// Contract the request targets.
    pub data_contract_id: Identifier,
    /// Document type name on that contract.
    pub document_type_name: String,
    /// Decoded `WHERE` clauses, with any `IN_TIME_RANGE` selections still
    /// pending in `time_ranges`.
    pub where_clauses: Vec<WhereClause>,
    /// `IN_TIME_RANGE` selections (v1 only), resolved against the
    /// quorum-signed block time once the response is known.
    pub time_ranges: Vec<TimeRangeSelection>,
    /// Decoded `ORDER BY` clauses.
    pub order_by: Vec<OrderClause>,
    /// Wire limit; `None` means "server default". A v0 request's `0` is
    /// normalised to `None` here, since on that wire `0` is the only way to
    /// leave the limit unset; a v1 `Some(0)` is refused at decode.
    pub limit: Option<u32>,
    /// Pagination cursor and whether it is inclusive.
    pub start_at: Option<[u8; 32]>,
    /// `true` for `start_at`, `false` for `start_after`; the server default
    /// when no cursor is given.
    pub start_at_included: bool,
}

/// A pending `IN_TIME_RANGE` selection: `(field, selector, grid)`.
pub type TimeRangeSelection = (String, TimeRangeSelector, Option<TimeRangeGridSpec>);

impl TryFromRequest<GetDocumentsRequest> for DocumentWireQuery {
    fn try_from_request(grpc_request: GetDocumentsRequest) -> Result<Self, Error> {
        match grpc_request.version.ok_or(Error::EmptyVersion)? {
            Version::V0(v0) => Self::try_from_v0(v0),
            Version::V1(v1) => Self::try_from_v1(v1),
        }
    }

    /// The wire encoder for document queries lives with the rich
    /// `DocumentQuery` builder in `dash-platform-queries`
    /// (`GetDocumentsRequest::try_from_platform_versioned`); this type only
    /// runs the decode direction, and refuses rather than duplicating it.
    fn try_to_request(&self) -> Result<GetDocumentsRequest, Error> {
        Err(request_error(
            "DocumentWireQuery is decode-only; encode a dash_platform_queries DocumentQuery",
        ))
    }
}

fn request_error(error: impl std::fmt::Display) -> Error {
    Error::RequestError {
        error: error.to_string(),
    }
}

fn cursor(bytes: Vec<u8>, what: &str) -> Result<[u8; 32], Error> {
    bytes
        .try_into()
        .map_err(|_| request_error(format!("{what} should be a 32 byte identifier")))
}

impl DocumentWireQuery {
    fn try_from_v0(request: GetDocumentsRequestV0) -> Result<Self, Error> {
        let GetDocumentsRequestV0 {
            data_contract_id,
            document_type,
            r#where,
            order_by,
            limit,
            prove,
            start,
        } = request;
        reject_unproved(prove)?;

        // The v0 wire carries CBOR arrays of `[field, operator, value]` /
        // `[field, "asc"|"desc"]`; decode them through the same
        // `from_components` parsers the server's v0 handler uses.
        let where_clauses = match cbor_array(&r#where, "where")? {
            None => Vec::new(),
            Some(clauses) => clauses
                .iter()
                .map(|clause| match clause {
                    Value::Array(components) => {
                        WhereClause::from_components(components).map_err(request_error)
                    }
                    _ => Err(request_error("where clause must be an array")),
                })
                .collect::<Result<_, _>>()?,
        };
        let order_by = match cbor_array(&order_by, "order_by")? {
            None => Vec::new(),
            Some(clauses) => clauses
                .iter()
                .map(|clause| match clause {
                    Value::Array(components) => OrderClause::from_components(components)
                        .map_err(|_| request_error("invalid order_by clause components")),
                    _ => Err(request_error("order_by clause must be an array")),
                })
                .collect::<Result<_, _>>()?,
        };
        let (start_at, start_at_included) = match start {
            None => (None, true),
            Some(V0Start::StartAt(at)) => (Some(cursor(at, "start at")?), true),
            Some(V0Start::StartAfter(after)) => (Some(cursor(after, "start after")?), false),
        };
        Ok(Self {
            data_contract_id: Identifier::from_bytes(&data_contract_id).map_err(request_error)?,
            document_type_name: document_type,
            where_clauses,
            time_ranges: Vec::new(),
            order_by,
            limit: (limit != 0).then_some(limit),
            start_at,
            start_at_included,
        })
    }

    fn try_from_v1(request: GetDocumentsRequestV1) -> Result<Self, Error> {
        // Destructured non-exhaustively: the generated request gains fields
        // as the query surface grows (e.g. `sub_queries` behind a feature),
        // and any field this verifier does not understand is checked below
        // through `Default` rather than silently ignored.
        let GetDocumentsRequestV1 {
            data_contract_id,
            document_type,
            where_clauses,
            order_by,
            limit,
            start,
            prove,
            selects,
            group_by,
            having,
            offset,
            chained,
            ..
        } = request.clone();
        reject_unproved(prove)?;
        let recognised = GetDocumentsRequestV1 {
            data_contract_id: request.data_contract_id.clone(),
            document_type: request.document_type.clone(),
            where_clauses: request.where_clauses.clone(),
            order_by: request.order_by.clone(),
            limit: request.limit,
            start: request.start.clone(),
            prove: request.prove,
            selects: request.selects.clone(),
            group_by: request.group_by.clone(),
            having: request.having.clone(),
            offset: request.offset,
            chained: request.chained.clone(),
            ..Default::default()
        };
        if recognised != request {
            return Err(request_error(
                "request carries fields this verifier does not understand (e.g. sub-queries); \
                 no proved plain-document response can be verified against it",
            ));
        }

        // Shapes the server routes anywhere but the plain document fetch.
        // Each mirrors a gate in rs-drive-abci's `query_documents_v1`; a
        // proved plain-document response can never belong to a request that
        // trips one, so refuse before touching proof machinery.
        if chained.is_some() {
            return Err(request_error(
                "chained document requests are verified through ChainedDocumentQuery",
            ));
        }
        if selects.len() > 1 {
            return Err(request_error(
                "multi-projection SELECT is not served; no proved response can belong to it",
            ));
        }
        if let Some(select) = selects.into_iter().next() {
            let select = wire::select_from_proto(select).map_err(request_error)?;
            if select != SelectProjection::documents() {
                return Err(request_error(format!(
                    "only SELECT DOCUMENTS is verified here; {select:?} is an aggregate \
                     projection with its own proof shape"
                )));
            }
        }
        if !group_by.is_empty() {
            return Err(request_error(
                "GROUP BY is refused by the server under SELECT DOCUMENTS",
            ));
        }
        if !having.is_empty() {
            return Err(request_error(
                "HAVING is refused by the server for a non-aggregate SELECT",
            ));
        }
        if let Some(offset) = offset {
            return Err(request_error(format!(
                "OFFSET {offset} is only served on the ranked surface, never for a document fetch"
            )));
        }

        let (time_range_proto, normal_proto): (Vec<_>, Vec<_>) = where_clauses
            .into_iter()
            .partition(wire::is_time_range_clause);
        let time_ranges = time_range_proto
            .into_iter()
            .map(|clause| wire::time_range_clause_from_proto(clause).map_err(request_error))
            .collect::<Result<_, _>>()?;
        let where_clauses = wire::where_clauses_from_proto(normal_proto).map_err(request_error)?;
        let order_by = wire::order_clauses_from_proto(order_by).map_err(request_error)?;
        let (start_at, start_at_included) = match start {
            None => (None, true),
            Some(V1Start::StartAt(at)) => (Some(cursor(at, "start at")?), true),
            Some(V1Start::StartAfter(after)) => (Some(cursor(after, "start after")?), false),
        };
        // The v1 wire has `optional uint32 limit`, so "use the default" is
        // spelled `None`; the server refuses an explicit `Some(0)` outright
        // (`validate_and_route`), unlike v0 where `0` is the only way to
        // leave the limit unset.
        if limit == Some(0) {
            return Err(request_error(
                "limit = 0 is not a valid v1 wire value; omit the limit for the server default",
            ));
        }
        Ok(Self {
            data_contract_id: Identifier::from_bytes(&data_contract_id).map_err(request_error)?,
            document_type_name: document_type,
            where_clauses,
            time_ranges,
            order_by,
            limit,
            start_at,
            start_at_included,
        })
    }

    /// Lower into the [`DriveDocumentQuery`] the server ran, exactly as
    /// rs-drive-abci's `query_documents_typed` does: the same
    /// `from_typed_clauses` constructor, the default [`DriveConfig`] limit
    /// contract, and the platform version the response was produced under.
    ///
    /// `block_time_ms` is the quorum-signed response time; `IN_TIME_RANGE`
    /// selections resolve against it to the same bucket the server used.
    pub fn to_drive_query<'a>(
        &self,
        contract: &'a DataContract,
        block_time_ms: Option<u64>,
        platform_version: &PlatformVersion,
    ) -> Result<DriveDocumentQuery<'a>, Error> {
        if contract.id() != self.data_contract_id {
            return Err(request_error(format!(
                "request targets data contract {} but the supplied contract is {}",
                self.data_contract_id,
                contract.id()
            )));
        }
        let document_type = contract
            .document_type_for_name(&self.document_type_name)
            .map_err(|e| request_error(format!("document type: {e}")))?;

        let mut where_clauses = self.where_clauses.clone();
        let mut resolved_time_ranges: Vec<ResolvedTimeRange> = Vec::new();
        if !self.time_ranges.is_empty() {
            let block_time_ms = block_time_ms.ok_or_else(|| {
                request_error("time range query needs the response block time to resolve")
            })?;
            for (field, selector, grid) in &self.time_ranges {
                let (clause, resolved) = resolve_time_range_bucket_clause(
                    field,
                    *selector,
                    *grid,
                    document_type,
                    block_time_ms,
                )?;
                where_clauses.push(clause);
                resolved_time_ranges.push(resolved);
            }
        }

        // Same translation as the server: `None` falls back to the config
        // default inside `from_typed_clauses`; values above `u16::MAX` are
        // refused before the cast.
        let limit = match self.limit {
            Some(n) if n > u32::from(u16::MAX) => {
                return Err(request_error(format!("limit {n} out of bounds")));
            }
            None => None,
            Some(n) => Some(n as u16),
        };

        let mut query = DriveDocumentQuery::from_typed_clauses(
            where_clauses,
            self.order_by.clone(),
            limit,
            self.start_at,
            self.start_at_included,
            None,
            contract,
            document_type,
            &DriveConfig::default(),
            platform_version,
        )?;
        query.resolved_time_ranges = resolved_time_ranges;
        Ok(query)
    }
}

fn reject_unproved(prove: bool) -> Result<(), Error> {
    if prove {
        Ok(())
    } else {
        Err(request_error(
            "request carries prove=false, so an honest server answered it unproved; a proved \
             response cannot belong to it",
        ))
    }
}

/// Decode a v0 CBOR clause field into its top-level array, `None` when the
/// field is empty or CBOR null (both mean "no clauses" to the server).
fn cbor_array(bytes: &[u8], field: &str) -> Result<Option<Vec<Value>>, Error> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let value: Value = ciborium::de::from_reader(bytes)
        .map_err(|_| request_error(format!("unable to decode '{field}' query from cbor")))?;
    match value {
        Value::Null => Ok(None),
        Value::Array(clauses) => Ok(Some(clauses)),
        _ => Err(request_error(format!("{field} must be an array"))),
    }
}

/// Reject a request whose wire version (`V0`/`V1`) is outside the
/// `document_query` feature-version bounds the given platform version's
/// server serves. The server answers such a request with
/// `UnsupportedQueryVersion`, so no proved response can belong to it.
fn check_wire_version_is_served(
    request: &GetDocumentsRequest,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let feature_version: u16 = match &request.version {
        Some(Version::V0(_)) => 0,
        Some(Version::V1(_)) => 1,
        None => return Err(Error::EmptyVersion),
    };
    let bounds = &platform_version.drive_abci.query.document_query;
    if !bounds.check_version(feature_version) {
        return Err(request_error(format!(
            "GetDocumentsRequest wire version V{feature_version} is outside the document_query \
             bounds {}..={} served at platform version {}",
            bounds.min_version, bounds.max_version, platform_version.protocol_version
        )));
    }
    Ok(())
}

/// Documents verified against the wire request that fetched them.
///
/// A distinct type from [`Documents`] because that one carries a blanket
/// `FromProof<Q>` for every `Q: TryInto<DriveDocumentQuery>`, which
/// coherence will not let a `GetDocumentsRequest` impl sit beside.
/// `Deref`s to the underlying [`Documents`] map.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RequestedDocuments(pub Documents);

impl std::ops::Deref for RequestedDocuments {
    type Target = Documents;
    fn deref(&self) -> &Documents {
        &self.0
    }
}

impl From<RequestedDocuments> for Documents {
    fn from(value: RequestedDocuments) -> Self {
        value.0
    }
}

impl Length for RequestedDocuments {
    fn count_some(&self) -> usize {
        self.0.count_some()
    }
    fn count(&self) -> usize {
        self.0.count()
    }
}

impl FromProof<GetDocumentsRequest> for RequestedDocuments {
    type Request = GetDocumentsRequest;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        check_wire_version_is_served(&request, platform_version)?;
        let wire_query = DocumentWireQuery::try_from_request(request)?;

        let contract: Arc<DataContract> = provider
            .get_data_contract(&wire_query.data_contract_id, platform_version)?
            .ok_or_else(|| {
                request_error(format!(
                    "context provider has no data contract {}",
                    wire_query.data_contract_id
                ))
            })?;

        // The block time is read off the response *before* verification only
        // to resolve time-range buckets; it is bound by the quorum signature
        // checked inside the delegated `FromProof`, so a lie fails there.
        let block_time_ms = response.metadata().ok().map(|mtd| mtd.time_ms);
        let drive_query = wire_query.to_drive_query(&contract, block_time_ms, platform_version)?;

        let (documents, mtd, proof) =
            <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
                drive_query,
                response,
                network,
                platform_version,
                provider,
            )?;
        Ok((documents.map(RequestedDocuments), mtd, proof))
    }
}
