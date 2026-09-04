//! Method to query documents from the Drive.

use std::sync::Arc;

use crate::error::Error;
use dapi_grpc::platform::v0::get_documents_request::Version::{V0, V1};
use dapi_grpc::platform::v0::{
    self as platform_proto,
    get_documents_request::{
        document_field_value,
        get_documents_request_v0::Start,
        get_documents_request_v1::{select, Select as ProtoSelect, Start as V1Start},
        having_aggregate, having_clause, order_clause,
        time_range_selection::{Grid as ProtoTimeRangeGrid, Selector as ProtoTimeRangeSelector},
        DocumentFieldValue as ProtoDocumentFieldValue, GetDocumentsRequestV0,
        GetDocumentsRequestV1, HavingAggregate as ProtoHavingAggregate,
        HavingClause as ProtoHavingClause, OrderClause as ProtoOrderClause,
        TimeRangeSelection as ProtoTimeRangeSelection, WhereClause as ProtoWhereClause,
        WhereOperator as ProtoWhereOperator,
    },
    GetDocumentsRequest, Proof, ResponseMetadata,
};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    document::Document,
    platform_value::{platform_value, Value},
    prelude::{DataContract, Identifier},
    InvalidVectorSizeError, ProtocolError,
};
use drive::config::DEFAULT_QUERY_LIMIT;
use drive::query::drive_document_ranked_query::mode_detection::ranked_order_key;
use drive::query::{
    resolve_time_range_bucket_clause, validate_resolved_time_range_clause_shapes,
    DriveDocumentQuery, HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator,
    HavingRightOperand, InternalClauses, OrderClause, ResolvedTimeRange, SelectFunction,
    SelectProjection, TimeRangeGridSpec, TimeRangeSelector, WhereClause, WhereOperator,
};
use drive_proof_verifier::{types::Documents, FromProof};

// TODO: remove DocumentQuery once ContextProvider that provides data contracts is merged.

/// One pending `IN_TIME_RANGE` selection: the timestamp field, the
/// `"newest"` / `"oldest"` selector, and — when the contract buckets the
/// field with more than one `timeRange` grid — the grid it targets (`None`
/// means the field's sole grid, and the server rejects the bare form on a
/// multi-grid field as ambiguous).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeRangeClause {
    /// The bucketed timestamp field the selection is on.
    pub field: String,
    /// Which active window to resolve to.
    pub selector: TimeRangeSelector,
    /// The grid targeted, in the contract's declared seconds; `None` when
    /// the field carries a single grid.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub grid: Option<TimeRangeGridSpec>,
}

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
    /// Time-range (`IN_TIME_RANGE`) selections on a timestamp field covered
    /// by a `timeRange` index. These are emitted as `IN_TIME_RANGE` clauses
    /// on the v1 wire and resolved server-side from the current block time;
    /// the verifier re-derives the same bucket from the quorum-signed
    /// response metadata time. v1-only (the v0 wire has no `IN_TIME_RANGE`
    /// operator). See [`Self::with_time_range`] and
    /// [`Self::with_time_range_grid`].
    #[cfg_attr(feature = "mocks", serde(default))]
    pub time_range_clauses: Vec<TimeRangeClause>,
    /// SQL `GROUP BY` field names, in left-to-right order. Empty =
    /// no explicit grouping (aggregate count for `select=Count`).
    /// Only meaningful when `select=Count`; non-empty with
    /// `select=Documents` is rejected by the server as unsupported.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub group_by: Vec<String>,
    /// SQL `HAVING` clauses — **boolean** aggregate filters that apply
    /// to the grouped rows produced by `select = Count | Sum | Avg`
    /// with a non-empty `group_by`. Unlike `where_clauses`, the left
    /// side is an aggregate (`COUNT(*)`, `SUM(field)`, `AVG(field)`)
    /// rather than a raw row field. See [`HavingClause`] /
    /// [`drive::query::HavingAggregate`] /
    /// [`drive::query::HavingOperator`] for the catalogs. Multiple
    /// entries combine with implicit `AND`.
    ///
    /// **Served from protocol version 14, for exactly one clause
    /// bounding the selected aggregate** with a contiguous-range
    /// operator (`=`, `>`, `>=`, `<`, `<=`, `BETWEEN*`) — the
    /// having-range surface, fetched as
    /// [`DocumentHavingEntries`](drive_proof_verifier::DocumentHavingEntries)
    /// and served as a value-bounded range read of the covering ranked
    /// index's axis secondary (the index must declare the matching
    /// `rankedCountable` / `rankedSummable` / `rankedAverageable`
    /// keyword). Everything else — multiple clauses (implicit AND), a
    /// clause on an aggregate the select does not project, `!=` / `IN`
    /// — is still rejected with `QuerySyntaxError::Unsupported`, as is
    /// any non-empty value at protocol version 13 and earlier.
    ///
    /// **`having` does not express ranking.** "The n highest-scoring
    /// groups" is [`Self::order_by_selected_aggregate`] +
    /// [`Self::with_limit`] — SQL's own `ORDER BY <agg> DESC LIMIT n`
    /// — which is also served from protocol version 14. The two
    /// compose only in the one shape the having grammar allows: an
    /// `ORDER BY` naming the selected aggregate sets the having
    /// range's walk direction.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub having: Vec<HavingClause>,
    /// `order_by` clauses for the query.
    ///
    /// For `select = Documents` these order the matched rows. For the
    /// **ranked** surface a single clause naming the selected
    /// aggregate orders the *groups* — see
    /// [`Self::order_by_selected_aggregate`], which builds it.
    pub order_by_clauses: Vec<OrderClause>,
    /// queryset limit. `0` is the sentinel for "unset / default" and
    /// is translated to `None` on the V1 wire (`optional uint32`).
    pub limit: u32,
    /// SQL `OFFSET` — how many result rows to skip before the returned
    /// page. `None` leaves the field unset on the wire.
    ///
    /// Served on exactly one path: the **ranked** surface (protocol
    /// version 14+), where it skips that many *ranks*, so
    /// `.order_by_selected_aggregate(Descending).with_limit(1)
    /// .with_offset(4)` is the 5th-best group. Everywhere else the
    /// server rejects a set offset with `Unsupported("OFFSET
    /// pagination is not yet implemented")`.
    ///
    /// `#[serde(default)]` for the same mock-vector compatibility
    /// reason as `select` / `group_by` / `having`: a fixture captured
    /// before offsets existed deserializes to `None`.
    #[cfg_attr(feature = "mocks", serde(default))]
    pub offset: Option<u32>,
    /// first object to start with
    pub start: Option<Start>,
}

/// Which end of a ranking a
/// [`DocumentQuery::order_by_selected_aggregate`] call walks from.
///
/// A named pair rather than a bare `ascending: bool`, because the two
/// readings of a ranked query — "the best n" and "the worst n" — are
/// what callers actually think in, and `false` meaning "best first" at
/// a call site is exactly the sort of thing that gets flipped in
/// review without anyone noticing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingDirection {
    /// `ORDER BY <aggregate> DESC` — walk from the largest aggregate
    /// down. The "top n" reading: entry 0 is the highest-scoring group.
    Descending,
    /// `ORDER BY <aggregate> ASC` — walk from the smallest aggregate
    /// up. The "bottom n" reading: entry 0 is the lowest-scoring group.
    Ascending,
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
            time_range_clauses: vec![],
            group_by: Vec::new(),
            having: Vec::new(),
            order_by_clauses: vec![],
            limit: 0,
            offset: None,
            start: None,
        })
    }

    /// Create new document query based on a [DriveDocumentQuery].
    ///
    /// Fails when the drive query carries time-range resolution provenance
    /// (`resolved_time_ranges`): the resolved bucket equality cannot be
    /// represented without it — see the `TryFrom` impl. Build the query
    /// with [`Self::with_time_range`] / [`Self::with_time_range_grid`]
    /// instead for time-range selections.
    pub fn new_with_drive_query(d: &DriveDocumentQuery) -> Result<Self, crate::error::Error> {
        Self::try_from(d)
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

    /// Restrict the query to a single time-range bucket of `field`
    /// (a timestamp covered by a `timeRange` index). The relative selectors
    /// ([`TimeRangeSelector::Newest`] / [`TimeRangeSelector::Oldest`]) pick a
    /// currently active range, resolved server-side from the current block
    /// time; the proof verifier re-derives the identical bucket from the
    /// quorum-signed response metadata time. A
    /// [`TimeRangeSelector::ByStart`] selection names a window absolutely —
    /// current or historic — by its grid-aligned start, which both sides
    /// read straight from the query. Emitted as an `IN_TIME_RANGE` clause
    /// on the v1 wire. Requires protocol version 14+ — the first version
    /// whose contract grammar hosts `timeRange` indexes.
    ///
    /// The bare selector is unambiguous only while exactly one grid buckets
    /// `field`; when the contract declares several grids over it, use
    /// [`Self::with_time_range_grid`] to name one.
    ///
    /// Existing time-range selections are preserved.
    pub fn with_time_range(
        mut self,
        field: impl Into<String>,
        selector: TimeRangeSelector,
    ) -> Self {
        self.time_range_clauses.push(TimeRangeClause {
            field: field.into(),
            selector,
            grid: None,
        });
        self
    }

    /// [`Self::with_time_range`] naming a specific grid — required when the
    /// contract buckets `field` with more than one `timeRange` grid. The
    /// spec's `range` / `step` / `phase` are the contract's own declared
    /// seconds, verbatim.
    ///
    /// Existing time-range selections are preserved.
    pub fn with_time_range_grid(
        mut self,
        field: impl Into<String>,
        selector: TimeRangeSelector,
        grid: TimeRangeGridSpec,
    ) -> Self {
        self.time_range_clauses.push(TimeRangeClause {
            field: field.into(),
            selector,
            grid: Some(grid),
        });
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
    /// Server capability today: `Documents`, `COUNT(*)`,
    /// `SUM(<field>)`, and `AVG(<field>)` are evaluated
    /// end-to-end. `COUNT(<field>)`, `MIN(<field>)`, and
    /// `MAX(<field>)` are accepted by the SDK but rejected by the
    /// server with `Unsupported("SELECT … is not yet
    /// implemented")` — the surface is shipped first and
    /// execution lands later.
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
    /// From protocol version 14, a grouped aggregate query carrying
    /// **exactly one** clause that bounds the selected aggregate with
    /// a contiguous-range operator (`=`, `>`, `>=`, `<`, `<=`, the
    /// `BETWEEN` variants) is served as a value-bounded range read of
    /// the covering ranked index's axis secondary — fetch the result
    /// through `DocumentHavingEntries::fetch`, which verifies the
    /// proof including its completeness. The server still rejects
    /// multiple clauses, a clause on a different aggregate than the
    /// select's, and the non-contiguous operators (`!=`, `IN`);
    /// protocol version 13 and earlier reject every non-empty
    /// `having`.
    ///
    /// This is **not** how you ask for a ranking — see
    /// [`Self::order_by_selected_aggregate`].
    pub fn with_having(mut self, having: Vec<HavingClause>) -> Self {
        self.having = having;
        self
    }

    /// Order the `GROUP BY` groups by the aggregate this query
    /// selects — the **ranked** surface, `ORDER BY <the selected
    /// aggregate> [ASC|DESC]` (protocol version 14+).
    ///
    /// Replaces any previously set `order_by`, because a ranked query
    /// takes exactly one ordering clause and a second one is rejected
    /// rather than combined.
    ///
    /// The ordered field name is derived from the current
    /// [`Self::select`] by rs-drive's own
    /// [`ranked_order_key`] — `SUM(f)` / `AVG(f)` are named by `f`,
    /// and `COUNT(*)` by the `$count` sentinel. Calling
    /// [`Self::with_select`] *after* this method leaves a stale field
    /// name behind and the server will refuse the request; set the
    /// select first, which is also how the query reads.
    ///
    /// Pair with [`Self::with_limit`] (the ranking's `n`, `1 ..= 100`)
    /// and optionally [`Self::with_offset`], then fetch with
    /// [`DocumentRankedEntries`](drive_proof_verifier::DocumentRankedEntries).
    ///
    /// # The 5th-best group
    ///
    /// ```rust,ignore
    /// # use dash_sdk::platform::{DataContract, DocumentQuery};
    /// # use dash_sdk::platform::documents::document_query::RankingDirection;
    /// # use dash_sdk::drive::query::SelectProjection;
    /// # fn example(contract: DataContract) -> Result<(), dash_sdk::Error> {
    /// // SELECT avg(grade) GROUP BY restaurantId
    /// //   ORDER BY avg(grade) DESC LIMIT 1 OFFSET 4
    /// let query = DocumentQuery::new(contract, "review")?
    ///     .with_select(SelectProjection::avg("grade"))
    ///     .with_group_by("restaurantId")
    ///     .order_by_selected_aggregate(RankingDirection::Descending)
    ///     .with_limit(1)
    ///     .with_offset(4);
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_by_selected_aggregate(mut self, direction: RankingDirection) -> Self {
        self.order_by_clauses = vec![OrderClause {
            field: ranked_order_key(&self.select).to_string(),
            ascending: matches!(direction, RankingDirection::Ascending),
        }];
        self
    }

    /// Set the SQL `OFFSET` — how many ranks to skip before the
    /// returned page.
    ///
    /// Only the ranked surface honours it (see
    /// [`Self::order_by_selected_aggregate`]); on every other path the
    /// server rejects a set offset with `Unsupported`. There is no
    /// ceiling: grovedb counts the skipped region from the subtree
    /// aggregates instead of walking it, on both `prove` settings, so
    /// the cost of a deep offset does not scale with the offset. It is
    /// not identical to a shallow one — `offset = 0` keeps a sequential
    /// fast path, a positive offset descends the tree in `O(log n)`,
    /// and an offset at or past the population is answered from the
    /// root without descending at all — but nothing here grows with how
    /// far you page, which is why there is no ceiling. Only a proved
    /// response additionally attests the count.
    ///
    /// An offset past the end of the ranking is a legitimate answer
    /// rather than an error — the page comes back empty, and on a
    /// proved fetch its `starting_rank` is the ranking's attested total
    /// population.
    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
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

    /// Convert into the wire-format [`GetDocumentsRequest`] using a
    /// specific [`PlatformVersion`] to pick V0 vs V1. The dispatch
    /// boundary is the document_query feature-version on the
    /// platform_version: `0` → V0, `1` → V1.
    pub fn try_into_request_for_version(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<GetDocumentsRequest, Error> {
        GetDocumentsRequest::try_from_platform_versioned(self, platform_version)
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
        let mut request: Self::Request = request.into();
        let response: Self::Response = response.into();

        // A time-range (`IN_TIME_RANGE`) selection is resolved to a concrete
        // bucket using the **quorum-signed** response metadata time — the same
        // authoritative block time the server used to resolve it — so the
        // reconstructed query matches the proof exactly. Resolve (and run the
        // provenance-vs-shape guard, via the one shared normalization helper
        // the aggregate verifiers also use) before the `DriveDocumentQuery`
        // conversion so the engine sees ordinary equality clauses.
        let mut resolved_time_ranges = Vec::new();
        if !request.time_range_clauses.is_empty() {
            // The generated `VersionedGrpcResponse::metadata()` handles both
            // response envelopes (and any future one), so no hand-written
            // version match is needed here.
            use dapi_grpc::platform::VersionedGrpcResponse;
            let time_ms = response
                .metadata()
                .map(|metadata| metadata.time_ms)
                .map_err(|_| drive_proof_verifier::Error::ResponseDecodeError {
                    error: "time range query proof response is missing block-time metadata"
                        .to_string(),
                })?;
            resolved_time_ranges =
                normalize_time_range_clauses_with_metadata_time(&mut request, time_ms)?;
        }

        let mut drive_query: DriveDocumentQuery =
            (&request)
                .try_into()
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!("Failed to convert DocumentQuery to DriveQuery: {}", e),
                })?;
        // The conversion cannot recover which equalities came from resolution,
        // so the provenance is carried across here; index selection reads it
        // to pin the query to the index that buckets the field.
        drive_query.resolved_time_ranges = resolved_time_ranges;

        <drive_proof_verifier::types::Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
            drive_query,
            response,
            network,
            platform_version,
            provider,
        )
    }
}

/// Resolve a request's pending time-range (`IN_TIME_RANGE`) selections into
/// concrete bucket-equality clauses on `request.where_clauses`, using the
/// **quorum-signed** response metadata block time — the same authoritative
/// time the server used — so the reconstructed query matches the proof
/// exactly.
///
/// Every proof-verification path that rebuilds a drive query from a
/// [`DocumentQuery`] must call this (or perform the identical resolution)
/// *before* reading `request.where_clauses` for mode detection, covering-index
/// selection, or query reconstruction: the documents path does it inline in
/// its `FromProof` impl, and the count / sum / average aggregate helpers call
/// this before resolving their mode. Skipping it would rebuild the query from
/// a different shape than the prover used.
///
/// Returns the resolution provenance — one [`ResolvedTimeRange`] (field +
/// exact grid) per selection, whose pushed clause is a bucket equality
/// rather than a raw-timestamp one. Callers must carry them into index
/// selection (`DriveDocumentQuery::resolved_time_ranges`, or the
/// `resolved_time_ranges` argument of the aggregate index pickers):
/// the pushed clause is an ordinary equality and nothing downstream can
/// otherwise tell that it must be matched against the resolved grid's
/// bucket starts.
/// [`resolve_time_range_clauses_with_metadata_time`] followed immediately by
/// the provenance-vs-shape guard — the two-step normalization every
/// proof-verification path must run, in this order, before mode detection,
/// covering-index selection, or query reconstruction. One definition so a
/// future verifier path cannot omit either step or run them out of order:
/// the aggregate paths once omitted the resolution entirely (valid proofs
/// were rejected), and a path that resolved without the guard would let a
/// caller-provided `In`/range clause on the resolved field reach the index
/// pickers as if its raw values were bucket starts.
pub(super) fn normalize_time_range_clauses_with_metadata_time(
    request: &mut DocumentQuery,
    time_ms: u64,
) -> Result<Vec<ResolvedTimeRange>, drive_proof_verifier::Error> {
    let resolved_time_ranges = resolve_time_range_clauses_with_metadata_time(request, time_ms)?;
    validate_resolved_time_range_clause_shapes(&request.where_clauses, &resolved_time_ranges)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!("invalid time range query shape: {}", e),
        })?;
    Ok(resolved_time_ranges)
}

pub(super) fn resolve_time_range_clauses_with_metadata_time(
    request: &mut DocumentQuery,
    time_ms: u64,
) -> Result<Vec<ResolvedTimeRange>, drive_proof_verifier::Error> {
    if request.time_range_clauses.is_empty() {
        return Ok(Vec::new());
    }
    let data_contract = Arc::clone(&request.data_contract);
    let document_type = data_contract
        .document_type_for_name(&request.document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!("document type not found for time range query: {}", e),
        })?;
    let time_range_clauses = std::mem::take(&mut request.time_range_clauses);
    let mut resolved_time_ranges = Vec::with_capacity(time_range_clauses.len());
    for TimeRangeClause {
        field,
        selector,
        grid,
    } in time_range_clauses
    {
        let (clause, resolved) =
            resolve_time_range_bucket_clause(&field, selector, grid, document_type, time_ms)
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!("failed to resolve time range clause: {}", e),
                })?;
        request.where_clauses.push(clause);
        resolved_time_ranges.push(resolved);
    }
    Ok(resolved_time_ranges)
}

/// Version-aware encoder. The dispatch is driven by the
/// `drive_abci.query.document_query` feature-version on
/// [`PlatformVersion`]: `0` → V0 wire (used by v3.0 testnet), `1` →
/// V1 wire (introduced in v3.1).
///
/// V0 lacks `selects` / `group_by` / `having` / `offset` and the
/// optional-limit semantics — callers that set those features get
/// `Error::Config` with a clear "requires Platform v3.1+" message
/// rather than a silently-truncated request. Time-range clauses are
/// additionally gated on the v14 contract grammar — see the `1 =>` arm.
impl TryFromPlatformVersioned<DocumentQuery> for GetDocumentsRequest {
    type Error = Error;

    fn try_from_platform_versioned(
        value: DocumentQuery,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let DocumentQuery {
            select,
            data_contract,
            document_type_name,
            where_clauses,
            time_range_clauses,
            group_by,
            having,
            order_by_clauses,
            limit,
            offset,
            start,
        } = value;

        let feature_version = platform_version
            .drive_abci
            .query
            .document_query
            .default_current_version;

        tracing::debug!(
            target: "dash_sdk::query_encoder",
            feature_version,
            protocol_version = platform_version.protocol_version,
            "encoding GetDocumentsRequest"
        );

        match feature_version {
            0 => {
                if !time_range_clauses.is_empty() {
                    return Err(Error::Config(
                        "time range (IN_TIME_RANGE) queries require protocol version 14+; the \
                         v0 getDocuments wire has no time-range operator"
                            .to_string(),
                    ));
                }
                encode_v0(
                    data_contract.id().to_vec(),
                    document_type_name,
                    where_clauses,
                    order_by_clauses,
                    limit,
                    offset,
                    start,
                    &select,
                    &group_by,
                    &having,
                )
            }
            1 => {
                // The v1 wire predates time-range indexes: protocol
                // versions 12 and 13 also serve it, but their contract
                // grammar (document meta-schema generations 1 and 2)
                // cannot host a `timeRange` index. Gate on the grammar
                // generation — the same table the server's parser reads —
                // rather than emitting an operator a pre-v14 server
                // rejects as an unknown discriminant.
                let grammar_generation = platform_version
                    .dpp
                    .contract_versions
                    .document_type_versions
                    .schema
                    .document_type_schema;
                if !time_range_clauses.is_empty() && grammar_generation < 3 {
                    return Err(Error::Config(format!(
                        "time range (IN_TIME_RANGE) queries require protocol version 14+ — the \
                         first version whose contract grammar hosts `timeRange` indexes; this \
                         network runs protocol version {}",
                        platform_version.protocol_version
                    )));
                }
                encode_v1(
                    data_contract.id().to_vec(),
                    document_type_name,
                    where_clauses,
                    time_range_clauses,
                    order_by_clauses,
                    limit,
                    offset,
                    start,
                    select,
                    group_by,
                    having,
                )
            }
            n => Err(Error::Config(format!(
                "GetDocumentsRequest wire encoder does not support feature_version={n} \
                 (drive_abci.query.document_query) on PlatformVersion v{}",
                platform_version.protocol_version
            ))),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_v1(
    data_contract_id: Vec<u8>,
    document_type: String,
    where_clauses: Vec<WhereClause>,
    time_range_clauses: Vec<TimeRangeClause>,
    order_by_clauses: Vec<OrderClause>,
    limit: u32,
    offset: Option<u32>,
    start: Option<Start>,
    select: SelectProjection,
    group_by: Vec<String>,
    having: Vec<HavingClause>,
) -> Result<GetDocumentsRequest, Error> {
    let mut where_clauses = where_clauses
        .into_iter()
        .map(where_clause_to_proto)
        .collect::<Result<Vec<_>, _>>()?;
    // Append time-range selections as `IN_TIME_RANGE` clauses carrying the
    // typed `time_range` operand (`value` stays unset — the selection is
    // not a field value). The grid, when named, repeats the contract's
    // declared seconds verbatim; a zero phase is proto3's default, so a
    // phaseless grid has exactly one wire spelling by construction. The
    // server resolves the relative selectors to a concrete bucket from
    // current block time and the verifier re-derives the same bucket from
    // the signed response metadata time; a `ByStart` selection carries its
    // window's start in the query itself, so both sides read it verbatim.
    for TimeRangeClause {
        field,
        selector,
        grid,
    } in time_range_clauses
    {
        let (proto_selector, start_ms) = match selector {
            TimeRangeSelector::Newest => (ProtoTimeRangeSelector::Newest, None),
            TimeRangeSelector::Oldest => (ProtoTimeRangeSelector::Oldest, None),
            TimeRangeSelector::ByStart { start_ms } => {
                (ProtoTimeRangeSelector::ByStart, Some(start_ms))
            }
        };
        where_clauses.push(ProtoWhereClause {
            field,
            operator: ProtoWhereOperator::InTimeRange as i32,
            value: None,
            time_range: Some(ProtoTimeRangeSelection {
                selector: proto_selector as i32,
                start_ms,
                grid: grid.map(|spec| ProtoTimeRangeGrid {
                    range: spec.range_seconds,
                    step: spec.step_seconds,
                    phase: spec.phase_seconds,
                }),
            }),
        });
    }
    let order_by = order_by_clauses
        .into_iter()
        .map(order_clause_to_proto)
        .collect();
    let having = having
        .into_iter()
        .map(having_clause_to_proto)
        .collect::<Result<Vec<_>, _>>()?;
    // `limit: u32` with `0` sentinel → `optional uint32` on the V1
    // wire. `None` lets the server apply its own default; explicit
    // `0` would be a strange "return zero rows" request.
    let limit = if limit == 0 { None } else { Some(limit) };
    // V0 and V1 ship separate `Start` enums even though the shape
    // is identical. Translate at the wire boundary so the
    // `DocumentQuery.start` field stays stable for callers already
    // using the V0 type.
    let start_v1 = start.map(|s| match s {
        Start::StartAfter(b) => V1Start::StartAfter(b),
        Start::StartAt(b) => V1Start::StartAt(b),
    });

    Ok(GetDocumentsRequest {
        version: Some(V1(GetDocumentsRequestV1 {
            data_contract_id,
            document_type,
            where_clauses,
            order_by,
            limit,
            // Document fetch always proves via this conversion.
            // Count fetch uses the same wire shape; both paths go
            // through the `FromProof` decoders which expect the
            // `Proof(...)` response variant. `SdkBuilder::with_proofs(false)`
            // is consequently a no-op for both — see the blanket
            // `Query<T> for T` impl in `packages/rs-sdk/src/platform/query.rs`
            // for the `tracing::warn!` emitted at fetch time when
            // proofs are disabled.
            prove: true,
            start: start_v1,
            // `repeated Select selects` on the wire — single
            // projection wraps in a one-element vec; the SDK's
            // `DocumentQuery` carries a single `SelectProjection`
            // because multi-projection is wire-only today.
            selects: vec![select_to_proto(select)],
            group_by,
            having,
            // Honoured on the ranked path (it is the `OFFSET` of
            // `ORDER BY <agg> DESC LIMIT n OFFSET m`) and rejected by
            // the server everywhere else. Passed straight through:
            // deciding here which paths may carry an offset would put
            // a second copy of that rule in the SDK.
            offset,
            chained: None,
            sub_queries: Vec::new(),
        })),
    })
}

#[allow(clippy::too_many_arguments)]
fn encode_v0(
    data_contract_id: Vec<u8>,
    document_type: String,
    where_clauses: Vec<WhereClause>,
    order_by_clauses: Vec<OrderClause>,
    limit: u32,
    offset: Option<u32>,
    start: Option<Start>,
    select: &SelectProjection,
    group_by: &[String],
    having: &[HavingClause],
) -> Result<GetDocumentsRequest, Error> {
    // V0 only carries plain `getDocuments` semantics — reject the
    // v1-only SQL-shaped surfaces with a typed error rather than
    // letting the server reject them after a round-trip.
    if !matches!(select.function, SelectFunction::Documents) {
        return Err(Error::Config(format!(
            "select={:?} requires Platform v3.1+ (V1 documents wire); pin/upgrade \
             to a v3.1+ network or rebuild the query with SelectProjection::documents()",
            select.function
        )));
    }
    if !group_by.is_empty() {
        return Err(Error::Config(
            "group_by requires Platform v3.1+ (V1 documents wire); not supported on V0".to_string(),
        ));
    }
    if !having.is_empty() {
        return Err(Error::Config(
            "having clauses require Platform v3.1+ (V1 documents wire); not supported on V0"
                .to_string(),
        ));
    }
    if offset.is_some() {
        // The V0 request message has no `offset` field at all, so
        // silently dropping it would page from rank 0 while the caller
        // believed they had skipped ahead — the one failure mode worth
        // an extra branch here.
        return Err(Error::Config(
            "offset requires Platform v3.1+ (V1 documents wire); not supported on V0".to_string(),
        ));
    }

    // V0 carries CBOR-serialized arrays of clause components. The
    // server decodes them via `ciborium::de::from_reader` into a
    // `Value`, then expects `Value::Array(clauses)` where each
    // inner clause is `[field_text, operator_text, value]` (where)
    // or `[field_text, "asc"|"desc"]` (order_by). Build the same
    // shape via the existing `From<WhereClause> for Value` /
    // `From<OrderClause> for Value` impls, then serialize the
    // top-level array.
    let where_bytes = if where_clauses.is_empty() {
        Vec::new()
    } else {
        let where_value = Value::Array(where_clauses.into_iter().map(Value::from).collect());
        where_value.to_cbor_buffer().map_err(|e| {
            Error::Protocol(dpp::ProtocolError::EncodingError(format!(
                "failed to CBOR-encode v0 where clauses: {e}"
            )))
        })?
    };
    let order_by_bytes = if order_by_clauses.is_empty() {
        Vec::new()
    } else {
        let order_value = Value::Array(order_by_clauses.into_iter().map(Value::from).collect());
        order_value.to_cbor_buffer().map_err(|e| {
            Error::Protocol(dpp::ProtocolError::EncodingError(format!(
                "failed to CBOR-encode v0 order_by clauses: {e}"
            )))
        })?
    };

    Ok(GetDocumentsRequest {
        version: Some(V0(GetDocumentsRequestV0 {
            data_contract_id,
            document_type,
            r#where: where_bytes,
            order_by: order_by_bytes,
            // V0's `limit` is a plain u32 with 0 = "server default".
            // V1's `optional uint32` keeps 0 as a structurally
            // meaningless explicit-zero; we translate by clamping
            // to 0 only when the caller meant "unset".
            limit,
            start,
            // `prove: true` hardcoded — same rationale as `encode_v1`:
            // document fetch always proves; `SdkBuilder::with_proofs(false)`
            // is a no-op for this path because the `FromProof` decoder
            // expects the `Proof(...)` response variant.
            prove: true,
        })),
    })
}

impl<'a> TryFrom<&'a DriveDocumentQuery<'a>> for DocumentQuery {
    type Error = crate::error::Error;

    /// Fallible by necessity: a drive query carrying `resolved_time_ranges`
    /// holds bucket-start equalities whose meaning lives in the provenance,
    /// and `DocumentQuery` has no field to carry it — the original
    /// `IN_TIME_RANGE` selector cannot be reconstructed from the resolved
    /// query. Serializing such a query would silently demote the bucket
    /// equality to a raw-timestamp predicate: a transformed-index-only
    /// contract then rejects the request, while a contract with a competing
    /// plain index returns a different — but validly proven — result.
    fn try_from(value: &'a DriveDocumentQuery<'a>) -> Result<Self, Self::Error> {
        if !value.resolved_time_ranges.is_empty() {
            return Err(Error::Config(
                "a drive query carrying time-range resolution provenance cannot be \
                 converted to a DocumentQuery: the resolved bucket equality would be \
                 demoted to a raw-timestamp predicate. Build the DocumentQuery with \
                 `with_time_range` / `with_time_range_grid` instead, so the selector \
                 is resolved against the signed response metadata"
                    .to_string(),
            ));
        }
        let data_contract = value.contract.clone();
        let document_type_name = value.document_type.name();
        let where_clauses = value.internal_clauses.clone().into();
        let order_by_clauses = value.order_by.iter().map(|(_, v)| v.clone()).collect();
        let limit = value.limit.unwrap_or(0) as u32;
        let offset = value.offset.map(u32::from);

        let start = if let Some(start_at) = value.start_at {
            match value.start_at_included {
                true => Some(Start::StartAt(start_at.to_vec())),
                false => Some(Start::StartAfter(start_at.to_vec())),
            }
        } else {
            None
        };

        Ok(Self {
            // `DriveDocumentQuery` has no SELECT/GROUP BY/HAVING/time-range
            // concept — it's a documents-only query. Default to the
            // v1 documents shape.
            select: SelectProjection::documents(),
            data_contract: Arc::new(data_contract),
            document_type_name: document_type_name.to_string(),
            where_clauses,
            time_range_clauses: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            order_by_clauses,
            limit,
            offset,
            start,
        })
    }
}

impl<'a> TryFrom<DriveDocumentQuery<'a>> for DocumentQuery {
    type Error = crate::error::Error;

    /// By-value twin of the by-reference conversion above — same
    /// provenance rejection, same rationale.
    fn try_from(value: DriveDocumentQuery<'a>) -> Result<Self, Self::Error> {
        DocumentQuery::try_from(&value)
    }
}

impl<'a> TryFrom<&'a DocumentQuery> for DriveDocumentQuery<'a> {
    type Error = crate::error::Error;

    fn try_from(request: &'a DocumentQuery) -> Result<Self, Self::Error> {
        // A pending (unresolved) time-range selection MUST be resolved into a
        // concrete bucket-equality clause before a drive query can be built —
        // see `resolve_time_range_clauses_with_metadata_time`. Silently
        // dropping it here would rebuild (and verify against) a strictly
        // broader query than the prover ran, so refuse instead: this makes
        // "forgot to resolve" a loud error on every present and future call
        // path rather than a silent verification hole.
        if !request.time_range_clauses.is_empty() {
            return Err(Error::Config(
                "the query's time range (IN_TIME_RANGE) selections have not been resolved into \
                 bucket equalities; resolve them against the response's quorum-signed metadata \
                 time before building a drive query"
                    .to_string(),
            ));
        }

        // let data_contract = request.data_contract.clone();
        let document_type = request
            .data_contract
            .document_type_for_name(&request.document_type_name)
            .map_err(ProtocolError::DataContractError)?;

        // Client-side construction groups under the latest grammar; the
        // server and the proof verifier enforce the network's protocol
        // version at path-query lowering.
        let internal_clauses = InternalClauses::extract_from_clauses(
            request.where_clauses.clone(),
            PlatformVersion::latest(),
        )
        .map_err(Error::Drive)?;

        // Mirror the limit contract of the server's
        // `DriveDocumentQuery::from_typed_clauses` exactly: `0` (this
        // struct's "unset" sentinel — V0's `limit: 0`, V1's
        // `limit: None`) falls back to the server default, and anything
        // above `DEFAULT_QUERY_LIMIT` (the `config.default_query_limit`
        // every deployed server runs with) is refused with the server's
        // own `QuerySyntaxError::InvalidLimit` rather than truncated or
        // passed through. A `u16::try_from` alone would not do: limits
        // 101..=65535 fit a `u16` but the server refuses them, so a raw
        // `DriveDocumentQuery` carrying one would verify a proof no
        // honest server could have produced.
        let limit = match request.limit {
            0 => Some(DEFAULT_QUERY_LIMIT),
            limit if limit > u32::from(DEFAULT_QUERY_LIMIT) => {
                return Err(Error::Drive(drive::error::Error::Query(
                    drive::error::query::QuerySyntaxError::InvalidLimit(format!(
                        "limit {} greater than max limit {}",
                        limit, DEFAULT_QUERY_LIMIT
                    )),
                )));
            }
            limit => Some(limit as u16),
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

        // `DriveDocumentQuery`'s offset is a `u16`; the wire's is a
        // `u32` because the ranked path takes an unbounded one. A
        // documents query that overflows `u16` is refused rather than
        // truncated — silently paging from a different rank than the
        // caller asked for is the worst available outcome.
        let offset = request
            .offset
            .map(|offset| {
                u16::try_from(offset).map_err(|_| {
                    Error::Config(format!(
                        "offset {offset} does not fit a documents query's u16 offset \
                         (max {}); offsets above that are only meaningful on the ranked \
                         surface, which does not route through DriveDocumentQuery",
                        u16::MAX
                    ))
                })
            })
            .transpose()?;

        let query = Self {
            contract: &request.data_contract,
            document_type,
            internal_clauses,
            offset,
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
            // A `DocumentQuery` reaching here carries no unresolved
            // time-range selection (rejected above) and cannot tell which of
            // its equalities came from resolution. Callers that resolved
            // selections assign the fields they resolved onto the returned
            // query; everything else is a raw query.
            resolved_time_ranges: vec![],
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
        // The typed IN_TIME_RANGE operand; never set on an ordinary value
        // clause (time-range selections are encoded by `encode_v1` itself,
        // from `time_range_clauses`).
        time_range: None,
    })
}

fn order_clause_to_proto(clause: OrderClause) -> ProtoOrderClause {
    // Drive's `OrderClause` carries a plain `field: String` —
    // emit the field-target variant of the wire's `target` oneof.
    // The aggregate-target variant (`ORDER BY COUNT(*)`) is
    // wire-only today; when drive's `OrderClause` gains an
    // aggregate target the SDK gets a parallel builder.
    ProtoOrderClause {
        target: Some(order_clause::Target::Field(clause.field)),
        ascending: clause.ascending,
    }
}

/// Convert a drive [`HavingClause`] into its wire-format proto
/// counterpart. The inverse of `rs-drive-abci`'s
/// `having_clause_from_proto`. Errors only on `Value` variants
/// the underlying `value_to_proto` can't represent — every
/// `HavingOperator` / `HavingAggregateFunction` discriminant has a
/// 1:1 wire counterpart and is always convertible.
fn having_clause_to_proto(clause: HavingClause) -> Result<ProtoHavingClause, Error> {
    let right = match clause.right {
        HavingRightOperand::Value(v) => having_clause::Right::Value(value_to_proto(v)?),
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
        SelectFunction::Min => select::Function::Min,
        SelectFunction::Max => select::Function::Max,
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
/// - `U128`/`I128` → `Text` (decimal string). **Not yet
///   round-trippable against `U128`/`I128`-typed indexed fields**:
///   the v1 typed-decode path (`v1/conversions.rs::value_from_proto`)
///   passes the text through as `Value::Text`, and the
///   downstream executor's strict `Value::to_integer()` then
///   rejects it. Schema-aware coercion (the
///   `DocumentPropertyType::value_from_string` path the v0 SQL
///   parser uses) hasn't been threaded through to the typed
///   path yet. The encoding is shipped because the proto needs a
///   home for 128-bit values; no production system contract
///   indexes `U128`/`I128` today. Tracked in the v1 follow-up
///   issue.
/// - `Array` → `List` (recursive, but only one level deep —
///   `value_to_proto` rejects nested arrays with
///   `EncodingError("nested DocumentFieldValue.list …")` to
///   match the server-side depth cap in
///   `v1/conversions.rs::value_from_proto_at_depth`, so wire-
///   malformed shapes fail at request-construction time with a
///   deterministic local error rather than after a transport
///   round-trip.
/// - `Null` → `NullValue(true)` (the `bool` payload is a
///   placeholder per the proto-side comment; only the variant
///   discriminant carries meaning)
/// - `Map`/`EnumU8`/`EnumString` → `Error` (no wire-format
///   counterpart for these shapes in a WhereClause operand)
fn value_to_proto(value: Value) -> Result<ProtoDocumentFieldValue, Error> {
    value_to_proto_at_depth(value, 0)
}

/// Recursion-bounded form of [`value_to_proto`]. Mirrors the
/// server-side `value_from_proto_at_depth` contract so encoder
/// and decoder agree on the supported `Value` subset: `depth = 0`
/// is the clause-level operand; `Array` is legal once (the flat
/// list of scalars for `IN` / `BETWEEN*`); any deeper nesting
/// rejects locally instead of producing a request the server
/// would round-trip just to reject.
fn value_to_proto_at_depth(value: Value, depth: u8) -> Result<ProtoDocumentFieldValue, Error> {
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
        // encode as a decimal string. See the function-level
        // docstring for the U128/I128 round-trip caveat.
        Value::U128(u) => document_field_value::Variant::Text(u.to_string()),
        Value::I128(i) => document_field_value::Variant::Text(i.to_string()),
        Value::Array(items) => {
            if depth >= 1 {
                return Err(Error::Protocol(dpp::ProtocolError::EncodingError(
                    "nested DocumentFieldValue.list is not supported on the v1 \
                     query surface; `IN` / `BETWEEN*` candidate lists are flat \
                     scalars only"
                        .to_string(),
                )));
            }
            document_field_value::Variant::List(document_field_value::ValueList {
                values: items
                    .into_iter()
                    .map(|v| value_to_proto_at_depth(v, depth + 1))
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

#[cfg(test)]
mod encode_version_gate_tests {
    //! The `IN_TIME_RANGE` operator is emitted only for protocol versions
    //! whose contract grammar hosts `timeRange` indexes (document
    //! meta-schema generation 3, first pinned by protocol version 14).
    //! Protocol versions 12 and 13 serve the same v1 getDocuments wire but
    //! predate the grammar — encoding for them must refuse up front
    //! instead of sending an operator the server rejects as an unknown
    //! discriminant.

    use super::*;
    use dpp::data_contract::DataContractFactory;
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, Identifier};
    use drive::query::TimeRangeSelector;

    fn post_contract() -> Arc<DataContract> {
        let schemas = platform_value!({
            "post": {
                "type": "object",
                "properties": {
                    "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
                },
                "required": ["hashtag"],
                "additionalProperties": false,
            }
        });
        let contract = DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .expect("expected a factory")
            .create_with_value_config(Identifier::new([7u8; 32]), 0, schemas, None, None)
            .expect("the post contract is well-formed")
            .data_contract_owned();
        Arc::new(contract)
    }

    fn newest_time_range_query() -> DocumentQuery {
        DocumentQuery::new(post_contract(), "post")
            .expect("the fixture has this document type")
            .with_time_range("$createdAt", TimeRangeSelector::Newest)
    }

    #[test]
    fn a_time_range_query_refuses_to_encode_for_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let error = GetDocumentsRequest::try_from_platform_versioned(
            newest_time_range_query(),
            platform_version,
        )
        .expect_err("protocol version 13's contract grammar has no timeRange indexes");
        let message = match error {
            Error::Config(message) => message,
            other => panic!("expected Error::Config, got {other:?}"),
        };
        assert!(
            message.contains("protocol version 14"),
            "the refusal must name the real version floor, got: {message}"
        );
    }

    #[test]
    fn a_time_range_query_encodes_the_typed_operand_for_protocol_version_14() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        let request = GetDocumentsRequest::try_from_platform_versioned(
            newest_time_range_query(),
            platform_version,
        )
        .expect("protocol version 14 hosts timeRange indexes");
        let Some(V1(v1)) = request.version else {
            panic!("protocol version 14 encodes on the v1 wire");
        };
        let [clause] = v1.where_clauses.as_slice() else {
            panic!("the pending selector must ride as exactly one IN_TIME_RANGE clause");
        };
        assert_eq!(clause.operator, ProtoWhereOperator::InTimeRange as i32);
        assert_eq!(
            clause.value, None,
            "the selection is not a field value; the generic operand stays unset"
        );
        let selection = clause
            .time_range
            .as_ref()
            .expect("the typed operand carries the selection");
        assert_eq!(selection.selector, ProtoTimeRangeSelector::Newest as i32);
        assert_eq!(selection.start_ms, None);
        assert_eq!(selection.grid, None, "a grid-less selection names no grid");
    }

    #[test]
    fn a_by_start_selection_encodes_its_window_start_and_grid() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        let query = DocumentQuery::new(post_contract(), "post")
            .expect("the fixture has this document type")
            .with_time_range_grid(
                "$createdAt",
                TimeRangeSelector::ByStart {
                    start_ms: 1_756_684_800_000,
                },
                TimeRangeGridSpec {
                    range_seconds: 86_400,
                    step_seconds: 86_400,
                    phase_seconds: 0,
                },
            );
        let request = GetDocumentsRequest::try_from_platform_versioned(query, platform_version)
            .expect("protocol version 14 hosts timeRange indexes");
        let Some(V1(v1)) = request.version else {
            panic!("protocol version 14 encodes on the v1 wire");
        };
        let [clause] = v1.where_clauses.as_slice() else {
            panic!("the selection must ride as exactly one IN_TIME_RANGE clause");
        };
        let selection = clause
            .time_range
            .as_ref()
            .expect("the typed operand carries the selection");
        assert_eq!(selection.selector, ProtoTimeRangeSelector::ByStart as i32);
        assert_eq!(
            selection.start_ms,
            Some(1_756_684_800_000),
            "the absolute window start rides in the query itself"
        );
        assert_eq!(
            selection.grid,
            Some(ProtoTimeRangeGrid {
                range: 86_400,
                step: 86_400,
                phase: 0,
            }),
            "the grid repeats the contract's declared seconds verbatim"
        );
    }

    #[test]
    fn a_query_without_time_range_clauses_still_encodes_for_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let query = DocumentQuery::new(post_contract(), "post")
            .expect("the fixture has this document type");
        GetDocumentsRequest::try_from_platform_versioned(query, platform_version)
            .expect("the gate only refuses queries that carry a time-range selection");
    }
}
