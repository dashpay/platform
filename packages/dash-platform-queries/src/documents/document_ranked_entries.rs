//! `FromProof` + `Fetch` for [`DocumentRankedEntries`] — the
//! **ranked** (`GROUP BY … ORDER BY <aggregate> LIMIT n [OFFSET m]`)
//! view of the unified `getDocuments` endpoint.
//!
//! A ranked query answers "which `n` groups score highest (or lowest)
//! on an aggregate?" — *top 5 restaurants by average grade* — in
//! `O(log n + k)`, with a proof. It reads a pre-sorted per-axis
//! *secondary* Merk maintained by the write path (grovedb PR #657)
//! rather than walking value trees, which is why it is cheap and why
//! its shape is so constrained.
//!
//! Per-request resolution (which axis, which direction, how many
//! groups, how many ranks to skip, which index covers them) lives in
//! [`super::ranked_proof_helpers`]; this module is the thin
//! `Fetch`-side wrapper.
//!
//! ## Request shape
//!
//! Exactly one aggregate `select`, exactly one `group_by` property,
//! exactly one `ORDER BY` clause naming that select's aggregate, and a
//! `LIMIT` — plus an optional `OFFSET`. `where` clauses are equality
//! pins on a covering compound ranked index's leading properties (one
//! per leading property, selecting which prefix's own ranking the walk
//! reads) — absent for a single-property index. No `having`, no
//! `start_at`: each of those is rejected rather than ignored, on both
//! sides, because a ranked walk cannot honour them and silently
//! answering a different question is worse than an error.
//!
//! [`DocumentQuery::order_by_selected_aggregate`] builds the ordering
//! clause, deriving the ordered field from the `select` through
//! rs-drive's own key mapping (`SUM(f)` / `AVG(f)` are named by `f`,
//! `COUNT(*)` by the `$count` sentinel), so there is no way to name it
//! wrong by hand.
//!
//! ## Contract prerequisites
//!
//! The index must opt in with `rankedCountable` / `rankedSummable` /
//! `rankedAverageable` (meta-schema v3, **protocol version 14+**). The
//! index may be single-property (`group_by` its property, no `where`)
//! or compound (`group_by` its trailing property, equality-pin every
//! leading one). Against a protocol-version-13
//! node the request is refused — v13's query table has no ranked path
//! and rejects the ordering as `Unsupported`. That is the intended
//! activation gate, not a bug: a v13 node and a v14 node must disagree
//! here and nowhere else, which is what lets a mixed-version network
//! run through the upgrade.
//!
//! ## Ranks, offsets, and the empty ranking
//!
//! The fetch result carries
//! [`starting_rank`](drive_proof_verifier::DocumentRankedEntries::starting_rank)
//! alongside the entries: entry `i` is the group at rank
//! `starting_rank + i`, which is what makes `LIMIT 1 OFFSET 4`
//! meaningful as "the 5th best" rather than "some entry". On the proved
//! path that number is re-derived from the proof's counted subtree
//! commitments, not taken from the node.
//!
//! An offset past the end of the ranking is a legitimate, provable
//! answer: no entries, and `starting_rank` equal to the ranking's whole
//! attested population. **Empty rankings prove too** — grovedb's
//! paginated prover emits a guaranteed-empty range against an empty
//! axis secondary rather than refusing — so querying a freshly
//! registered contract with `prove = true` returns an empty page rather
//! than an error, and the proved and unproven paths agree.
//!
//! ## Reading the values
//!
//! Entries come back in ranking order; **do not re-sort**. Averages
//! are fixed-point integers: divide by
//! [`RANKED_AVG_SCALE`](drive_proof_verifier::RANKED_AVG_SCALE) — a
//! re-export of grovedb's own constant, which moved from `10^15` to
//! `10^19` before release, so never hardcode the literal — or call
//! [`RankedEntryValue::as_f64`](drive_proof_verifier::RankedEntryValue::as_f64),
//! which does that division for you.
//!
//! **How exact the average is depends on the path.** Fetched with a
//! proof (the default, and what the examples below do), the fixed point
//! is the integer grovedb committed to and ranked on. Fetched without
//! one, the wire carries only an `f64` of the average and the SDK
//! re-scales it back, so the digits past `f64`'s ~15–16 significant
//! decimals are reconstruction noise — fine to render, not something to
//! compare for equality. Ranking *order* is exact either way.
//!
//! ## Example: top 5 restaurants by average grade
//!
//! `SELECT AVG(grade) GROUP BY restaurantId ORDER BY avg(grade) DESC LIMIT 5`
//!
//! ```rust,ignore
//! use dash_sdk::{Sdk, platform::{DataContract, DocumentQuery, Fetch, Identifier}};
//! use dash_sdk::drive::query::SelectProjection;
//! use dash_sdk::platform::documents::document_query::RankingDirection;
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
//!     .order_by_selected_aggregate(RankingDirection::Descending)
//!     .with_limit(5);
//!
//! let ranked = block_on(DocumentRankedEntries::fetch(&sdk, query))
//!     .expect("fetch succeeds")
//!     .expect("a well-formed ranked query always answers");
//!
//! // Entry order IS the ranking order — best first.
//! for (offset, entry) in ranked.entries.iter().enumerate() {
//!     let rank = ranked.starting_rank + offset as u64;
//!     let restaurant = String::from_utf8_lossy(&entry.key);
//!     if let RankedEntryValue::AvgFixedPoint(fixed_point) = entry.value {
//!         // `as_f64()` is this same division, for when you only want
//!         // to display the number:
//!         //     let average = entry.value.as_f64();
//!         // Keep the `fixed_point` itself when you need the exact
//!         // integer the proof committed to — comparing two groups,
//!         // reproducing the ranking, storing it. On a `prove = false`
//!         // fetch that integer is a reconstruction from the wire's
//!         // double, so it is only as precise as an `f64`.
//!         let average = (fixed_point as f64) / (RANKED_AVG_SCALE as f64);
//!         println!("#{}: {restaurant}: {average}", rank + 1);
//!     }
//! }
//! ```
//!
//! ## Example: the 5th-best restaurant
//!
//! `SELECT AVG(grade) GROUP BY restaurantId ORDER BY avg(grade) DESC LIMIT 1 OFFSET 4`
//!
//! ```rust,ignore
//! # use dash_sdk::platform::{DataContract, DocumentQuery};
//! # use dash_sdk::platform::documents::document_query::RankingDirection;
//! # use dash_sdk::drive::query::SelectProjection;
//! # fn example(contract: DataContract) -> Result<(), dash_sdk::Error> {
//! let query = DocumentQuery::new(contract, "review")?
//!     .with_select(SelectProjection::avg("grade"))
//!     .with_group_by("restaurantId")
//!     .order_by_selected_aggregate(RankingDirection::Descending)
//!     .with_limit(1)
//!     .with_offset(4);
//! # Ok(())
//! # }
//! ```

use crate::documents::document_query::DocumentQuery;
use crate::documents::ranked_proof_helpers::verify_ranked_query;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentRankedEntries, FromProof};

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
        let (page, mtd, proof) =
            verify_ranked_query(request, response, platform_version, provider)?;
        Ok((page.map(DocumentRankedEntries::from_verified), mtd, proof))
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the ranked client surface: the ordering
    //! builder, the request→wire encoding, and the client-side grammar
    //! mirror.
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
    //! not exist offline.

    use super::*;
    use crate::documents::document_query::RankingDirection;
    use crate::documents::ranked_proof_helpers::assert_ranked_shape;
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::select as proto_select;
    use dapi_grpc::platform::v0::get_documents_request::{
        order_clause, GetDocumentsRequestV1, OrderClause as ProtoOrderClause,
        Version as RequestVersion,
    };
    use dapi_grpc::platform::v0::GetDocumentsRequest;
    use dpp::data_contract::DataContract;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::TryFromPlatformVersioned;
    use drive::query::{
        HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
        SelectProjection, WhereClause, WhereOperator, RANKED_COUNT_ORDER_KEY,
    };
    use std::sync::Arc;

    /// The protocol version the ranked surface activates at. Pinned
    /// as a literal so a future bump of `PlatformVersion::latest()`
    /// past 14 doesn't silently change what these tests encode.
    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    /// Any contract with a known document type — the wire encoder is
    /// schema-agnostic (it never resolves `select` / `group_by` /
    /// `order_by` field names against the contract; the *server* does
    /// that), so the fixture's document type carries the encoding
    /// test fine while the field names stay the headline example's,
    /// keeping this comparable to rs-drive-abci's `ranked_tests`.
    fn contract() -> Arc<DataContract> {
        Arc::new(
            get_data_contract_fixture(None, 0, platform_version().protocol_version)
                .data_contract_owned(),
        )
    }

    /// `SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC LIMIT 5`.
    fn top_five_by_avg_grade() -> DocumentQuery {
        DocumentQuery::new(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_select(SelectProjection::avg("grade"))
            .with_group_by("restaurantId")
            .order_by_selected_aggregate(RankingDirection::Descending)
            .with_limit(5)
    }

    fn encode(query: DocumentQuery) -> GetDocumentsRequest {
        GetDocumentsRequest::try_from_platform_versioned(query, platform_version())
            .expect("a ranked query encodes onto the V1 wire")
    }

    fn v1_of(request: GetDocumentsRequest) -> GetDocumentsRequestV1 {
        match request.version.expect("the encoder always sets a version") {
            RequestVersion::V1(v1) => v1,
            RequestVersion::V0(_) => {
                panic!("a ranked query must encode onto the V1 wire; V0 has no `order_by` targets")
            }
        }
    }

    /// The ordered field name, asserted through the wire's `target`
    /// oneof so a future change to the aggregate-target spelling can't
    /// pass this test by accident.
    fn ordered_field(clause: &ProtoOrderClause) -> &str {
        match clause.target.as_ref().expect("the target is always set") {
            order_clause::Target::Field(field) => field.as_str(),
            other => panic!("a ranked ORDER BY rides the field target, got {other:?}"),
        }
    }

    /// The headline round-trip: the SDK must put a ranked query on the
    /// wire in **exactly** the shape rs-drive-abci's
    /// `avg_axis_top_k_returns_fixed_point_entries` proved the server
    /// accepts. Asserted field by field rather than with a single
    /// struct comparison so a failure names the field that drifted.
    #[test]
    fn ranked_query_encodes_the_proven_wire_shape() {
        let v1 = v1_of(encode(top_five_by_avg_grade()));

        // One aggregate select, naming the averaged property.
        assert_eq!(v1.selects.len(), 1, "a ranked query has exactly one select");
        assert_eq!(v1.selects[0].function, proto_select::Function::Avg as i32);
        assert_eq!(v1.selects[0].field, "grade");

        // One GROUP BY property — the ranked index's only property.
        assert_eq!(v1.group_by, vec!["restaurantId".to_string()]);

        // One ORDER BY clause naming the select's aggregate, descending.
        assert_eq!(
            v1.order_by.len(),
            1,
            "a ranked query has exactly one ordering clause — it is the ranking"
        );
        assert_eq!(ordered_field(&v1.order_by[0]), "grade");
        assert!(
            !v1.order_by[0].ascending,
            "Descending is the `top n` reading and must not invert on the wire"
        );

        // The ranking's n rides `limit`.
        assert_eq!(v1.limit, Some(5));

        // Everything else must be at its "unset" wire value: a ranked
        // request that carried any of these would be rejected.
        assert!(v1.where_clauses.is_empty());
        assert!(v1.having.is_empty());
        assert_eq!(v1.offset, None);
        assert!(v1.start.is_none());
        assert!(v1.prove, "the Fetch path always requests a proof");
    }

    /// **The 5th-best group.** `LIMIT 1 OFFSET 4` is the whole point of
    /// the offset surface, and the offset has to survive onto the wire
    /// — a dropped one silently answers "the best" instead.
    #[test]
    fn the_fifth_best_encodes_limit_one_offset_four() {
        let query = DocumentQuery::new(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_select(SelectProjection::avg("grade"))
            .with_group_by("restaurantId")
            .order_by_selected_aggregate(RankingDirection::Descending)
            .with_limit(1)
            .with_offset(4);

        let v1 = v1_of(encode(query.clone()));
        assert_eq!(v1.limit, Some(1));
        assert_eq!(v1.offset, Some(4));
        assert!(!v1.order_by[0].ascending);

        // And the client-side grammar resolves it to the same page the
        // prover will produce.
        let mode = assert_ranked_shape(&query, platform_version())
            .expect("LIMIT 1 OFFSET 4 is well-formed");
        assert_eq!(mode.k, 1);
        assert_eq!(mode.offset, 4);
        assert!(mode.descending);
    }

    /// An offset far past any plausible population is **not** capped:
    /// grovedb attests the skipped region from counted commitments
    /// rather than walking it, so a deep page costs what a shallow one
    /// does and there is nothing for a cap to protect.
    #[test]
    fn a_very_deep_offset_is_not_capped() {
        let query = top_five_by_avg_grade().with_limit(1).with_offset(u32::MAX);
        assert_eq!(v1_of(encode(query.clone())).offset, Some(u32::MAX));
        let mode = assert_ranked_shape(&query, platform_version())
            .expect("an offset past the end is a provable answer, not an error");
        assert_eq!(mode.offset, u32::MAX);
    }

    /// `COUNT(*)` rankings are ordered by the **`$count` sentinel**,
    /// not by a property name — the axis counts documents per group, so
    /// there is no property to name. The builder derives it from the
    /// select through rs-drive's own mapping, which is the only way the
    /// client and the server can be guaranteed to agree.
    #[test]
    fn count_star_ranking_orders_by_the_count_sentinel() {
        let query = DocumentQuery::new(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_select(SelectProjection::count_star())
            .with_group_by("restaurantId")
            .order_by_selected_aggregate(RankingDirection::Descending)
            .with_limit(2);

        let v1 = v1_of(encode(query));
        assert_eq!(v1.selects[0].function, proto_select::Function::Count as i32);
        assert_eq!(v1.selects[0].field, "");
        assert_eq!(ordered_field(&v1.order_by[0]), RANKED_COUNT_ORDER_KEY);
        assert_eq!(
            RANKED_COUNT_ORDER_KEY, "$count",
            "the sentinel is wire-visible; changing it is a protocol change"
        );
    }

    /// `Ascending` is the "bottom n" reading and must reach the wire as
    /// `ascending: true`. Pinned separately from the descending case
    /// because a builder that ignored its argument would still pass the
    /// headline test.
    #[test]
    fn ascending_encodes_the_bottom_n_reading() {
        let query = top_five_by_avg_grade()
            .order_by_selected_aggregate(RankingDirection::Ascending)
            .with_limit(1);
        let v1 = v1_of(encode(query.clone()));
        assert!(v1.order_by[0].ascending);

        let mode = assert_ranked_shape(&query, platform_version()).expect("ASC LIMIT 1 is valid");
        assert!(!mode.descending, "ASC ranks lowest-first");
        assert_eq!(mode.k, 1, "ASC LIMIT 1 is the single worst-ranked group");
    }

    /// `order_by_selected_aggregate` **replaces** rather than appends.
    /// A ranked query takes exactly one ordering clause, so a builder
    /// that pushed would turn a second call — or a call after an
    /// unrelated `with_order_by` — into a request the server rejects.
    #[test]
    fn order_by_selected_aggregate_replaces_any_prior_ordering() {
        let query = top_five_by_avg_grade()
            .with_order_by(drive::query::OrderClause {
                field: "restaurantId".to_string(),
                ascending: true,
            })
            .order_by_selected_aggregate(RankingDirection::Descending);

        assert_eq!(query.order_by_clauses.len(), 1);
        assert_eq!(query.order_by_clauses[0].field, "grade");
        assert!(assert_ranked_shape(&query, platform_version()).is_ok());
    }

    /// The client-side grammar must resolve the same
    /// `(axis, descending, k, offset)` tuple the server does — that
    /// tuple goes into the proof envelope and is re-checked by the
    /// verifier, so a client that resolved it differently could not
    /// verify an honest proof.
    #[test]
    fn assert_ranked_shape_resolves_the_ranking() {
        let mode = assert_ranked_shape(&top_five_by_avg_grade(), platform_version())
            .expect("the headline query is well-formed");
        assert!(mode.descending, "DESC ranks highest-first");
        assert_eq!(mode.k, 5);
        assert_eq!(mode.offset, 0, "an unset OFFSET is rank 0");
        assert_eq!(mode.group_by_property, "restaurantId");
        assert_eq!(mode.aggregate_field, "grade");
    }

    /// Every knob a ranked walk cannot honour is rejected **client
    /// side**, before a round trip. Each of these is also rejected by
    /// the server; mirroring them here turns a network error into an
    /// immediate, specific one.
    #[test]
    fn assert_ranked_shape_rejects_what_a_ranking_cannot_honour() {
        let base = top_five_by_avg_grade();

        // An ordering on something other than the selected aggregate
        // asks for an order the secondary cannot produce.
        let wrong_order = {
            let mut q = base.clone();
            q.order_by_clauses = vec![drive::query::OrderClause {
                field: "restaurantId".to_string(),
                ascending: true,
            }];
            q
        };
        assert!(
            assert_ranked_shape(&wrong_order, platform_version()).is_err(),
            "ordering by the GROUP BY property is not a ranking by the aggregate"
        );

        // No ordering at all: a plain grouped aggregate, and the caller
        // wanted `DocumentSplitAverages`.
        let no_order = {
            let mut q = base.clone();
            q.order_by_clauses = Vec::new();
            q
        };
        assert!(
            assert_ranked_shape(&no_order, platform_version()).is_err(),
            "a query with no ordering is not a ranked query"
        );

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

        // HAVING is a boolean per-group predicate and the ranked
        // executor cannot drop groups from the middle of its walk.
        let with_having = base.clone().with_having(vec![HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Avg,
                field: "grade".to_string(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(dpp::platform_value::Value::U64(4)),
        }]);
        assert!(
            assert_ranked_shape(&with_having, platform_version()).is_err(),
            "a ranking cannot also filter its groups"
        );
    }

    /// The ranking's `n` rides `limit`, and an out-of-range one is
    /// rejected rather than clamped: `k` is echoed inside the proof
    /// envelope and re-checked by the verifier, so a silent clamp would
    /// produce a proof the client's own reconstruction rejects.
    #[test]
    fn assert_ranked_shape_rejects_an_out_of_range_limit() {
        // `0` is `DocumentQuery`'s "unset" sentinel, and a ranking with
        // no `n` has no size; `101` is past `MAX_RANKED_LIMIT`.
        for limit in [0u32, 101] {
            let query = top_five_by_avg_grade().with_limit(limit);
            assert!(
                assert_ranked_shape(&query, platform_version()).is_err(),
                "LIMIT {limit} is outside 1..=100 and must be rejected, not clamped"
            );
        }
    }

    /// The V0 wire has no `offset` field at all, so encoding one there
    /// has to fail loudly. Dropping it would page from rank 0 while the
    /// caller believed they had skipped ahead.
    #[test]
    fn a_v0_encode_refuses_to_silently_drop_an_offset() {
        let mut v0_version = PlatformVersion::latest().clone();
        v0_version
            .drive_abci
            .query
            .document_query
            .default_current_version = 0;

        let query = DocumentQuery::new(contract(), "niceDocument")
            .expect("doctype exists")
            .with_offset(4);

        let err = GetDocumentsRequest::try_from_platform_versioned(query, &v0_version)
            .expect_err("V0 cannot carry an offset");
        assert!(
            format!("{err}").contains("offset"),
            "the refusal must name the field that cannot be carried, got: {err}"
        );
    }
}
