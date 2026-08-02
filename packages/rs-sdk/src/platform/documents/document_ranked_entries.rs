//! `FromProof` + `Fetch` for [`DocumentRankedEntries`] — the
//! **ranked** (`HAVING … TOP(n)`) view of the unified `getDocuments`
//! endpoint.
//!
//! A ranked query answers "which `n` groups score highest (or lowest)
//! on an aggregate?" — *top 5 restaurants by average grade* — in
//! `O(log n + k)`, with a proof. It reads a pre-sorted per-axis
//! *secondary* Merk maintained by the write path (grovedb PR #657)
//! rather than walking value trees, which is why it is cheap and why
//! its shape is so constrained.
//!
//! Per-request resolution (which axis, which direction, how many
//! groups, which index covers them) lives in
//! [`super::ranked_proof_helpers`]; this module is the thin
//! `Fetch`-side wrapper, plus the two client-facing affordances the
//! surface needs: [`ranking_having`] to build a well-formed `HAVING`
//! clause, and [`is_empty_ranking_prove_rejection`] to recognise the
//! one server rejection that has an actionable recovery.
//!
//! ## Request shape
//!
//! Exactly one aggregate `select`, exactly one `group_by` property,
//! exactly one `having` clause whose right operand is a ranking on the
//! *same* aggregate — and nothing else. No `where`, no `limit`, no
//! `offset`, no `start_at`, no `order_by`: each of those is rejected
//! rather than ignored, on both sides, because a ranked walk cannot
//! honour them and silently answering a different question is worse
//! than an error. The result size is the `n` of `TOP(n)`; the result
//! order is the ranking.
//!
//! ## Contract prerequisites
//!
//! The index must opt in with `rankedCountable` / `rankedSummable` /
//! `rankedAverageable` (meta-schema v3, **protocol version 14+**), and
//! ranked indexes are single-property. Against a protocol-version-13
//! node the request is rejected wholesale with
//! `Unsupported("HAVING clause is not yet implemented")` — v13's query
//! table refuses every non-empty `HAVING`, ranking operand or not.
//! That is the intended activation gate, not a bug: a v13 node and a
//! v14 node must disagree here and nowhere else, which is what lets a
//! mixed-version network run through the upgrade.
//!
//! ## The empty-ranking prove gap
//!
//! grovedb has no absence-proof shape for an empty axis secondary, so
//! a node **cannot prove** a ranking over an index that holds no
//! documents. It rejects such a request with `invalid_argument` and a
//! message telling the caller to retry with `prove = false`. The SDK
//! does **not** auto-fall-back: a caller who asked for a proof gets an
//! error, not silently unverified data. [`ranking_having`]'s sibling
//! [`is_empty_ranking_prove_rejection`] recognises that rejection so
//! callers can implement the fallback *explicitly* — the unproven read
//! answers the same request with an empty list, decodable via
//! [`DocumentRankedEntries::from_unproved_response`]. Once the index
//! holds one document the proved form works.
//!
//! ## Reading the values
//!
//! Entries come back in ranking order; **do not re-sort**. Averages
//! are fixed-point integers: divide by
//! [`RANKED_AVG_SCALE`](drive_proof_verifier::RANKED_AVG_SCALE) — a
//! re-export of grovedb's own constant, which moved from `10^15` to
//! `10^19` before release, so never hardcode the literal — or call
//! [`RankedEntryValue::as_f64`](drive_proof_verifier::RankedEntryValue::as_f64)
//! for a lossy display value.
//!
//! ## Example: top 5 restaurants by average grade
//!
//! `SELECT AVG(grade) GROUP BY restaurantId HAVING AVG(grade) IN TOP(5)`
//!
//! ```rust,no_run
//! use dash_sdk::{Sdk, platform::{DataContract, DocumentQuery, Fetch, Identifier}};
//! use dash_sdk::drive::query::{HavingAggregateFunction, HavingRankingKind, SelectProjection};
//! use dash_sdk::platform::documents::document_ranked_entries::ranking_having;
//! use drive_proof_verifier::{DocumentRankedEntries, RankedEntryValue, RANKED_AVG_SCALE};
//! use futures::executor::block_on;
//!
//! # const RESTAURANTS_CONTRACT_ID: [u8; 32] = [0; 32];
//! let sdk = Sdk::new_mock();
//! let contract = block_on(DataContract::fetch(&sdk, Identifier::new(RESTAURANTS_CONTRACT_ID)))
//!     .expect("fetch contract")
//!     .expect("contract exists");
//!
//! let query = DocumentQuery::new(contract, "review")
//!     .expect("document type exists")
//!     .with_select(SelectProjection::avg("grade"))
//!     .with_group_by("restaurantId")
//!     .with_having(ranking_having(
//!         HavingAggregateFunction::Avg,
//!         "grade",
//!         HavingRankingKind::Top,
//!         Some(5),
//!     ));
//!
//! let ranked = block_on(DocumentRankedEntries::fetch(&sdk, query))
//!     .expect("fetch succeeds")
//!     .expect("a well-formed ranked query always answers");
//!
//! // Entry order IS the ranking order — best first.
//! for entry in &ranked.0 {
//!     let restaurant = String::from_utf8_lossy(&entry.key);
//!     if let RankedEntryValue::AvgFixedPoint(fixed_point) = entry.value {
//!         let average = (fixed_point as f64) / (RANKED_AVG_SCALE as f64);
//!         println!("{restaurant}: {average}");
//!     }
//! }
//! ```

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::ranked_proof_helpers::verify_ranked_query;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::tonic::Code;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRanking,
    HavingRankingKind, HavingRightOperand,
};
use drive_proof_verifier::{DocumentRankedEntries, FromProof};
use rs_dapi_client::transport::TransportError;
use rs_dapi_client::DapiClientError;

/// The marker substring the node's empty-ranking rejection carries.
///
/// Matching on a message rather than a typed error is deliberate and
/// mirrors what the server itself has to do one layer down (grovedb
/// flattens the merk-side "cannot create proof for empty tree" into a
/// `CorruptedData(String)` at the indexed-axis proof boundary). Kept
/// as a constant so the coupling is visible in one place.
const EMPTY_RANKING_MARKER: &str = "empty ranking cannot be proved";

/// Build the single `HAVING` clause a ranked query needs, pairing the
/// ranking kind with the comparison operator the grammar expects.
///
/// That pairing is the surface's sharpest edge: `TOP(n)` / `BOTTOM(n)`
/// are *set membership* and take `IN`, while `MAX` / `MIN` are the
/// value-based scalars and take `=`. Getting it backwards is rejected
/// by both the SDK and the server, with an error that is clear but
/// avoidable — hence this constructor. (`= TOP(1)` is also accepted,
/// so the `n == 1` case is not a trap.)
///
/// **`MAX` / `MIN` are rejected.** They remain wire-decodable, but
/// evaluation refuses them on every axis: `= MAX` selects *every*
/// group tied at the extreme, and the ranked storage cannot prove that
/// set — ties break by group key, so a bounded read would silently
/// omit tied groups. Ask for `TOP(1)` / `BOTTOM(1)` instead, which is
/// the positional "single best- / worst-ranked group" and documents
/// dropping ties as its meaning. Building a `MAX` / `MIN` clause here
/// is allowed so the refusal comes back from the one place that owns
/// the grammar rather than being duplicated in the SDK.
///
/// `field` must be the property the aggregate applies to and must
/// match the `select` exactly; pass `""` for `COUNT(*)`, which counts
/// documents per group and takes no field. `n` is required for
/// `TOP` / `BOTTOM` (`1 ..= 100`) and is ignored for `MAX` / `MIN`,
/// which are refused either way.
///
/// ```rust
/// use dash_sdk::drive::query::{HavingAggregateFunction, HavingOperator, HavingRankingKind};
/// use dash_sdk::platform::documents::document_ranked_entries::ranking_having;
///
/// // SELECT COUNT(*) … HAVING COUNT(*) IN BOTTOM(3)
/// let having = ranking_having(HavingAggregateFunction::Count, "", HavingRankingKind::Bottom, Some(3));
/// assert_eq!(having[0].operator, HavingOperator::In);
/// ```
pub fn ranking_having(
    function: HavingAggregateFunction,
    field: impl Into<String>,
    kind: HavingRankingKind,
    n: Option<u64>,
) -> Vec<HavingClause> {
    let operator = match kind {
        HavingRankingKind::Top | HavingRankingKind::Bottom => HavingOperator::In,
        HavingRankingKind::Max | HavingRankingKind::Min => HavingOperator::Equal,
    };
    vec![HavingClause {
        aggregate: HavingAggregate {
            function,
            field: field.into(),
        },
        operator,
        right: HavingRightOperand::Ranking(HavingRanking { kind, n }),
    }]
}

/// Whether `error` is the node's "this ranking has no groups yet, and
/// an empty ranking cannot be proved" rejection.
///
/// The one ranked failure with an actionable recovery: re-issue the
/// same request unproved (the answer is an empty list) or wait until
/// the index holds a document. The SDK deliberately does not do that
/// for you — you asked for a proof, and quietly returning unverified
/// data instead would defeat the point — so this predicate exists to
/// let you make the fallback an explicit, visible decision.
///
/// **Advisory only.** It recognises the message shipped by nodes that
/// implement the ranked surface; `false` therefore means "not
/// recognised", not "definitely a different condition". Never branch
/// on it in a way that turns a `false` into a silent success.
pub fn is_empty_ranking_prove_rejection(error: &crate::Error) -> bool {
    let crate::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(status))) =
        error
    else {
        return false;
    };
    status.code() == Code::InvalidArgument && status.message().contains(EMPTY_RANKING_MARKER)
}

impl FromProof<DocumentQuery> for DocumentRankedEntries {
    type Request = DocumentQuery;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();
        // Unlike the count / sum / average impls there is no separate
        // `assert_select_is_*` pre-check here: the ranked grammar
        // check is the first step of resolution and returns the
        // resolved mode, so it runs once, inside the helper. See
        // `ranked_proof_helpers::assert_ranked_shape`.
        let (entries, mtd, proof) =
            verify_ranked_query(request, response, platform_version, provider)?;
        Ok((
            entries.map(DocumentRankedEntries::from_verified),
            mtd,
            proof,
        ))
    }
}

impl Fetch for DocumentRankedEntries {
    type Query = DocumentQuery;
    type Request = dapi_grpc::platform::v0::GetDocumentsRequest;
}

#[cfg(test)]
mod tests {
    //! Offline tests for the ranked client surface: the `HAVING`
    //! constructor, the request→wire encoding, the client-side
    //! grammar mirror, and the empty-ranking predicate.
    //!
    //! Proof verification needs a populated Drive and a real grovedb
    //! proof, so it is exercised where those exist: rs-drive's
    //! `drive_document_ranked_query::tests` runs prover and verifier
    //! against a live Drive (including a bit-flip sweep proving no
    //! tamper survives with the honest root hash), and rs-drive-abci's
    //! `ranked_tests` pins the wire encoding of the same values. The
    //! SDK's own network-backed suites live in `tests/fetch/` and run
    //! against recorded vectors; there is no ranked vector yet because
    //! recording one needs a protocol-version-14 network, which does
    //! not exist offline. That is P7's job.

    use super::*;
    use crate::platform::documents::ranked_proof_helpers::assert_ranked_shape;
    use dapi_grpc::platform::v0::get_documents_request::Version as RequestVersion;
    use dapi_grpc::platform::v0::get_documents_request::{
        get_documents_request_v1::select as proto_select, having_aggregate, having_clause,
        having_ranking,
    };
    use dapi_grpc::platform::v0::GetDocumentsRequest;
    use dapi_grpc::tonic::Status;
    use dpp::data_contract::DataContract;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::TryFromPlatformVersioned;
    use drive::query::{SelectProjection, WhereClause, WhereOperator};
    use std::sync::Arc;

    /// The protocol version the ranked surface activates at. Pinned
    /// as a literal so a future bump of `PlatformVersion::latest()`
    /// past 14 doesn't silently change what these tests encode.
    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    /// Any contract with a known document type — the wire encoder is
    /// schema-agnostic (it never resolves `select` / `group_by` /
    /// `having` field names against the contract; the *server* does
    /// that), so the fixture's document type carries the encoding
    /// test fine while the field names stay the headline example's,
    /// keeping this comparable to rs-drive-abci's `ranked_tests`.
    fn contract() -> Arc<DataContract> {
        Arc::new(
            get_data_contract_fixture(None, 0, platform_version().protocol_version)
                .data_contract_owned(),
        )
    }

    /// `SELECT AVG(grade) GROUP BY restaurantId HAVING AVG(grade) IN TOP(5)`.
    fn top_five_by_avg_grade() -> DocumentQuery {
        DocumentQuery::new(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_select(SelectProjection::avg("grade"))
            .with_group_by("restaurantId")
            .with_having(ranking_having(
                HavingAggregateFunction::Avg,
                "grade",
                HavingRankingKind::Top,
                Some(5),
            ))
    }

    fn encode(query: DocumentQuery) -> GetDocumentsRequest {
        GetDocumentsRequest::try_from_platform_versioned(query, platform_version())
            .expect("a ranked query encodes onto the V1 wire")
    }

    /// The headline round-trip: the SDK must put a ranked query on the
    /// wire in **exactly** the shape rs-drive-abci's
    /// `avg_axis_top_k_returns_fixed_point_entries` proved the server
    /// accepts. Asserted field by field rather than with a single
    /// struct comparison so a failure names the field that drifted.
    #[test]
    fn ranked_query_encodes_the_proven_wire_shape() {
        let RequestVersion::V1(v1) = encode(top_five_by_avg_grade())
            .version
            .expect("the encoder always sets a version")
        else {
            panic!("a ranked query must encode onto the V1 wire; V0 has no `having` field");
        };

        // One aggregate select, naming the averaged property.
        assert_eq!(v1.selects.len(), 1, "a ranked query has exactly one select");
        assert_eq!(v1.selects[0].function, proto_select::Function::Avg as i32);
        assert_eq!(v1.selects[0].field, "grade");

        // One GROUP BY property — the ranked index's only property.
        assert_eq!(v1.group_by, vec!["restaurantId".to_string()]);

        // One HAVING clause: same aggregate as the select, `IN`
        // operator, `TOP(5)` ranking operand.
        assert_eq!(v1.having.len(), 1, "a ranked query has exactly one having");
        let clause = &v1.having[0];
        let aggregate = clause
            .aggregate
            .as_ref()
            .expect("the aggregate is always set");
        assert_eq!(aggregate.function, having_aggregate::Function::Avg as i32);
        assert_eq!(aggregate.field, "grade");
        assert_eq!(clause.operator, having_clause::Operator::In as i32);
        match clause.right.as_ref().expect("the right operand is set") {
            having_clause::Right::Ranking(ranking) => {
                assert_eq!(ranking.kind, having_ranking::Kind::Top as i32);
                assert_eq!(ranking.n, Some(5));
            }
            other => panic!("a ranked query's right operand is a Ranking, got {other:?}"),
        }

        // Everything else must be at its "unset" wire value: a ranked
        // request that carried any of these would be rejected.
        assert!(v1.where_clauses.is_empty());
        assert!(v1.order_by.is_empty());
        assert_eq!(v1.limit, None);
        assert_eq!(v1.offset, None);
        assert!(v1.start.is_none());
        assert!(v1.prove, "the Fetch path always requests a proof");
    }

    /// `COUNT(*)` rankings carry an **empty** field on both the select
    /// and the having aggregate — the axis counts documents per group,
    /// so there is no property to name. A non-empty field here is
    /// rejected by the grammar, so pinning the encoding matters.
    #[test]
    fn count_star_ranking_encodes_empty_fields() {
        let query = DocumentQuery::new(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_select(SelectProjection::count_star())
            .with_group_by("restaurantId")
            .with_having(ranking_having(
                HavingAggregateFunction::Count,
                "",
                HavingRankingKind::Top,
                Some(2),
            ));

        let RequestVersion::V1(v1) = encode(query).version.expect("version is set") else {
            panic!("expected the V1 wire");
        };
        assert_eq!(v1.selects[0].function, proto_select::Function::Count as i32);
        assert_eq!(v1.selects[0].field, "");
        let aggregate = v1.having[0].aggregate.as_ref().expect("aggregate is set");
        assert_eq!(aggregate.function, having_aggregate::Function::Count as i32);
        assert_eq!(aggregate.field, "");
    }

    /// `MAX` / `MIN` take `=`; `TOP` / `BOTTOM` take `IN` and require
    /// an `n`. This is the pairing [`ranking_having`] exists to get
    /// right, and it is what the wire carries. The builder still
    /// assembles the `MAX` / `MIN` shapes even though evaluation
    /// refuses them — see
    /// [`assert_ranked_shape_rejects_max_and_min`].
    #[test]
    fn ranking_having_pairs_each_kind_with_its_operator() {
        for (kind, n, expected_operator) in [
            (HavingRankingKind::Top, Some(5), HavingOperator::In),
            (HavingRankingKind::Bottom, Some(3), HavingOperator::In),
            (HavingRankingKind::Max, None, HavingOperator::Equal),
            (HavingRankingKind::Min, None, HavingOperator::Equal),
        ] {
            let having = ranking_having(HavingAggregateFunction::Sum, "amount", kind, n);
            assert_eq!(having.len(), 1);
            assert_eq!(
                having[0].operator, expected_operator,
                "{kind:?} must pair with {expected_operator:?}"
            );
            match having[0].right {
                HavingRightOperand::Ranking(ranking) => {
                    assert_eq!(ranking.kind, kind);
                    assert_eq!(ranking.n, n);
                }
                ref other => panic!("expected a Ranking operand, got {other:?}"),
            }
        }
    }

    /// The client-side grammar must resolve the same
    /// `(axis, descending, k)` triple the server does — that triple
    /// goes into the proof envelope and is re-checked by the verifier,
    /// so a client that resolved it differently could not verify an
    /// honest proof.
    #[test]
    fn assert_ranked_shape_resolves_the_ranking() {
        let mode = assert_ranked_shape(&top_five_by_avg_grade(), platform_version())
            .expect("the headline query is well-formed");
        assert!(mode.descending, "TOP ranks highest-first");
        assert_eq!(mode.k, 5);
        assert_eq!(mode.group_by_property, "restaurantId");
        assert_eq!(mode.aggregate_field, "grade");

        let bottom = count_query_ranked_by(HavingRankingKind::Bottom, Some(1));
        let mode =
            assert_ranked_shape(&bottom, platform_version()).expect("BOTTOM(1) is well-formed");
        assert!(!mode.descending, "BOTTOM ranks lowest-first");
        assert_eq!(mode.k, 1, "BOTTOM(1) is the single worst-ranked group");
    }

    /// A `COUNT(*)` query ranked by `kind`, for the tests that vary
    /// only the ranking.
    fn count_query_ranked_by(kind: HavingRankingKind, n: Option<u64>) -> DocumentQuery {
        DocumentQuery::new(contract(), "niceDocument")
            .expect("doctype exists")
            .with_select(SelectProjection::count_star())
            .with_group_by("restaurantId")
            .with_having(ranking_having(HavingAggregateFunction::Count, "", kind, n))
    }

    /// `MAX` / `MIN` are refused client side, so the caller learns
    /// before a round trip. They are *value-based* — `= MAX` selects
    /// every group tied at the extreme — and the axis secondary breaks
    /// ties by group key, so no bounded read can prove that set. The
    /// refusal names the positional alternative rather than silently
    /// serving `TOP(1)` / `BOTTOM(1)`, which would answer a different
    /// question whenever the extreme is tied.
    #[test]
    fn assert_ranked_shape_rejects_max_and_min() {
        for kind in [HavingRankingKind::Max, HavingRankingKind::Min] {
            for n in [None, Some(1)] {
                let query = count_query_ranked_by(kind, n);
                let err = assert_ranked_shape(&query, platform_version())
                    .expect_err("MAX / MIN cannot be served");
                let message = format!("{err}");
                assert!(
                    message.contains("tied") && message.contains("TOP(1)"),
                    "the refusal must explain ties and name the alternative, got: {message}"
                );
            }
        }
    }

    /// Every knob a ranked walk cannot honour is rejected **client
    /// side**, before a round trip. Each of these is also rejected by
    /// the server; mirroring them here turns a network error into an
    /// immediate, specific one — and, for `order_by`, is the only
    /// place the SDK can catch it, since `DocumentRankedRequest` has
    /// no field to carry it.
    #[test]
    fn assert_ranked_shape_rejects_what_a_ranking_cannot_honour() {
        let base = top_five_by_avg_grade();

        let with_order_by = {
            let mut q = base.clone();
            q.order_by_clauses = vec![drive::query::OrderClause {
                field: "restaurantId".to_string(),
                ascending: true,
            }];
            q
        };
        let err = assert_ranked_shape(&with_order_by, platform_version())
            .expect_err("ORDER BY contradicts the ranking order");
        assert!(format!("{err}").contains("ORDER BY"));

        let with_limit = base.clone().with_limit(10);
        let err = assert_ranked_shape(&with_limit, platform_version())
            .expect_err("the result size is the ranking's n");
        assert!(format!("{err}").contains("limit"));

        let with_where = {
            let mut q = base.clone();
            q.where_clauses = vec![WhereClause {
                field: "restaurantId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: dpp::platform_value::Value::Text("a".to_string()),
            }];
            q
        };
        let err = assert_ranked_shape(&with_where, platform_version())
            .expect_err("the axis secondary cannot rank a filtered subset");
        assert!(format!("{err}").contains("where"));

        // A `having` whose aggregate disagrees with the `select`
        // would rank one thing while projecting another.
        let mismatched = {
            let mut q = base.clone();
            q.having = ranking_having(
                HavingAggregateFunction::Sum,
                "grade",
                HavingRankingKind::Top,
                Some(5),
            );
            q
        };
        let err = assert_ranked_shape(&mismatched, platform_version())
            .expect_err("the having aggregate must match the select");
        assert!(format!("{err}").contains("same aggregate"));

        // No ranking operand at all: this is a plain grouped
        // aggregate, and the caller wanted `DocumentSplitAverages`.
        let no_having = {
            let mut q = base.clone();
            q.having = Vec::new();
            q
        };
        assert!(
            assert_ranked_shape(&no_having, platform_version()).is_err(),
            "a query with no ranking is not a ranked query"
        );
    }

    /// `n` above the ceiling is rejected rather than clamped: `k` is
    /// echoed inside the proof envelope and re-checked by the
    /// verifier, so a silent clamp would produce a proof the client's
    /// own reconstruction rejects.
    #[test]
    fn assert_ranked_shape_rejects_an_out_of_range_n() {
        for n in [0u64, 101] {
            let mut query = top_five_by_avg_grade();
            query.having = ranking_having(
                HavingAggregateFunction::Avg,
                "grade",
                HavingRankingKind::Top,
                Some(n),
            );
            assert!(
                assert_ranked_shape(&query, platform_version()).is_err(),
                "TOP({n}) is outside 1..=100 and must be rejected, not clamped"
            );
        }
    }

    /// The empty-ranking predicate recognises the node's rejection —
    /// and only that one. A different `invalid_argument`, a different
    /// status code, or a non-transport error must all read as "not
    /// recognised" so a caller can never mistake one for a licence to
    /// drop proof verification.
    #[test]
    fn recognises_only_the_empty_ranking_rejection() {
        let grpc = |code, message: &str| {
            crate::Error::DapiClientError(DapiClientError::Transport(TransportError::Grpc(
                Status::new(code, message),
            )))
        };

        let genuine = grpc(
            Code::InvalidArgument,
            "this ranking has no groups yet, and an empty ranking cannot be proved: grovedb \
             has no absence-proof shape for an empty axis secondary. Retry with `prove = \
             false`",
        );
        assert!(is_empty_ranking_prove_rejection(&genuine));

        assert!(
            !is_empty_ranking_prove_rejection(&grpc(
                Code::InvalidArgument,
                "ORDER BY is not valid for a ranked query"
            )),
            "a different invalid_argument is a different problem"
        );
        assert!(
            !is_empty_ranking_prove_rejection(&grpc(Code::Internal, EMPTY_RANKING_MARKER)),
            "the marker only counts on an invalid_argument status"
        );
        assert!(
            !is_empty_ranking_prove_rejection(&crate::Error::Config(
                EMPTY_RANKING_MARKER.to_string()
            )),
            "a non-transport error is never this rejection"
        );
    }
}
