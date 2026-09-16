//! `FromProof` + `Fetch` for [`DocumentHavingEntries`] — the
//! **having-range** (`GROUP BY … HAVING <aggregate> <op> <value>
//! LIMIT n`) view of the unified `getDocuments` endpoint.
//!
//! A having-range query answers "which groups' aggregate falls inside a
//! value bound?" — *hashtags with more than 100 posts* — in
//! `O(log n + k)`, with a proof whose Merk range boundaries also attest
//! **completeness**: a node cannot silently omit a matching group. It
//! reads the same pre-sorted per-axis *secondary* Merk the ranked
//! surface walks (grovedb PR #657), addressed by value bound instead of
//! by rank.
//!
//! Per-request resolution (which axis, which bounds the operator
//! translates to, which index covers them) lives in
//! [`super::having_proof_helpers`]; this module is the thin
//! `Fetch`-side wrapper.
//!
//! ## Request shape
//!
//! Exactly one aggregate `select`, exactly one `group_by` property,
//! exactly one `having` clause **bounding the selected aggregate** with
//! a contiguous-range operator (`=`, `>`, `>=`, `<`, `<=`, `BETWEEN*` —
//! `!=` and `IN` are rejected), and a `LIMIT`. `ORDER BY` is optional:
//! omitted means ascending by the aggregate; naming the selected
//! aggregate sets the direction. `where` clauses are pins on a covering
//! compound ranked index's leading properties (one per leading
//! property, selecting which prefix's groups the bound reads) — absent
//! for a single-property index. Each pin is an equality, except that
//! **at most one** may be an `IN` of 2..=10 distinct elements: the
//! bound fans out across one prefix branch per element and merges,
//! entries carrying the encoded branch segment in `in_key` (unset on
//! single-branch responses; a single-element `IN` normalizes to the
//! equality pin; a `null` pin on another property cannot combine with
//! the `IN`). No `offset`, no `start_at`.
//!
//! ## Contract prerequisites
//!
//! Same as the ranked surface: the index must opt in with
//! `rankedCountable` / `rankedSummable` / `rankedAverageable`
//! (meta-schema v3, **protocol version 14+**). The index may be
//! single-property (`group_by` its property, no `where`) or compound
//! (`group_by` its trailing property, pin every leading one — equality
//! pins, at most one of them an `IN`).
//! Against a pre-v14 node the request is refused with "HAVING clause
//! is not yet implemented" — the intended activation gate.
//!
//! ## Reading the result
//!
//! Entries come back in axis order in the walk direction; **do not
//! re-sort**. Fewer than `n` entries means fewer groups matched.
//! **Exactly `n` may mean the match set was cut at the limit.**
//! Tightening the bound past the last aggregate value seen continues
//! past *distinct* values only: a cut inside a tie (several groups
//! sharing the boundary aggregate) cannot be continued — the tied
//! groups past the limit stay unreachable until a composite-key cursor
//! exists — so size the limit above the widest expected tie. Averages
//! are fixed-point integers, exact on this (proved) path; see the
//! ranked module's notes, which apply verbatim.
//!
//! ## Example: hashtags with more than 100 posts
//!
//! `SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100 ORDER BY $count DESC LIMIT 100`
//!
//! ```rust,ignore
//! use dash_sdk::{Sdk, platform::{DataContract, DocumentQuery, Fetch, Identifier}};
//! use dash_sdk::drive::query::{
//!     HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator,
//!     HavingRightOperand, SelectProjection,
//! };
//! use dash_sdk::platform::documents::document_query::RankingDirection;
//! use dpp::platform_value::Value;
//! use drive_proof_verifier::DocumentHavingEntries;
//! use futures::executor::block_on;
//!
//! # const POSTS_CONTRACT_ID: [u8; 32] = [0; 32];
//! let sdk = Sdk::new_mock();
//! let contract = block_on(DataContract::fetch(&sdk, Identifier::new(POSTS_CONTRACT_ID)))
//!     .expect("fetch contract")
//!     .expect("contract exists");
//!
//! let query = DocumentQuery::new(contract, "post")
//!     .expect("document type exists")
//!     .with_select(SelectProjection::count_star())
//!     .with_group_by("hashtag")
//!     .with_having(vec![HavingClause {
//!         aggregate: HavingAggregate {
//!             function: HavingAggregateFunction::Count,
//!             field: String::new(),
//!         },
//!         operator: HavingOperator::GreaterThan,
//!         right: HavingRightOperand::Value(Value::U64(100)),
//!     }])
//!     .order_by_selected_aggregate(RankingDirection::Descending)
//!     .with_limit(100);
//!
//! let matching = block_on(DocumentHavingEntries::fetch(&sdk, query))
//!     .expect("fetch succeeds")
//!     .expect("a well-formed having query always answers");
//!
//! for entry in &matching.entries {
//!     let hashtag = String::from_utf8_lossy(&entry.key);
//!     println!("#{hashtag}: {} posts", entry.value.as_f64());
//! }
//! ```

use crate::documents::document_query::DocumentQuery;
use crate::documents::having_proof_helpers::verify_having_query;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::{DocumentHavingEntries, FromProof};

impl FromProof<DocumentQuery> for DocumentHavingEntries {
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
        // Same single-pass design as the ranked impl: the grammar check
        // is the first step of resolution, inside the helper.
        let (entries, mtd, proof) =
            verify_having_query(request, response, platform_version, provider)?;
        Ok((
            entries.map(DocumentHavingEntries::from_verified),
            mtd,
            proof,
        ))
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the having client surface: the request→wire
    //! encoding and the client-side grammar mirror. Proof verification
    //! is exercised end-to-end in rs-drive's
    //! `drive_document_having_query::tests` and rs-drive-abci's
    //! `having_range_tests`, where a populated Drive exists.

    use super::*;
    use crate::documents::document_query::RankingDirection;
    use crate::documents::having_proof_helpers::assert_having_shape;
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::select as proto_select;
    use dapi_grpc::platform::v0::get_documents_request::{
        having_aggregate, having_clause, GetDocumentsRequestV1, Version as RequestVersion,
    };
    use dapi_grpc::platform::v0::GetDocumentsRequest;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::Value;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::version::TryFromPlatformVersioned;
    use drive::query::{
        AxisRangeBounds, HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator,
        HavingRightOperand, SelectProjection,
    };
    use std::sync::Arc;

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn contract() -> Arc<DataContract> {
        Arc::new(
            get_data_contract_fixture(None, 0, platform_version().protocol_version)
                .data_contract_owned(),
        )
    }

    fn count_over_100() -> HavingClause {
        HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Count,
                field: String::new(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(Value::U64(100)),
        }
    }

    /// `SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100 LIMIT 100`.
    fn hashtags_over_100() -> DocumentQuery {
        DocumentQuery::new(contract(), "niceDocument")
            .expect("the fixture has this document type")
            .with_select(SelectProjection::count_star())
            .with_group_by("hashtag")
            .with_having(vec![count_over_100()])
            .with_limit(100)
    }

    fn v1_of(query: DocumentQuery) -> GetDocumentsRequestV1 {
        let request = GetDocumentsRequest::try_from_platform_versioned(query, platform_version())
            .expect("a having query encodes onto the V1 wire");
        match request.version.expect("the encoder always sets a version") {
            RequestVersion::V1(v1) => v1,
            RequestVersion::V0(_) => {
                panic!("a having query must encode onto the V1 wire; V0 has no `having` field")
            }
        }
    }

    /// The headline round-trip: the wire shape must be exactly what the
    /// server's routing accepts — one select, one group_by, one having
    /// clause, a limit, nothing else.
    #[test]
    fn having_query_encodes_the_expected_wire_shape() {
        let v1 = v1_of(hashtags_over_100());

        assert_eq!(v1.selects.len(), 1);
        assert_eq!(v1.selects[0].function, proto_select::Function::Count as i32);
        assert_eq!(v1.selects[0].field, "");
        assert_eq!(v1.group_by, vec!["hashtag".to_string()]);

        assert_eq!(v1.having.len(), 1, "exactly one having clause");
        let clause = &v1.having[0];
        let aggregate = clause.aggregate.as_ref().expect("aggregate is set");
        assert_eq!(aggregate.function, having_aggregate::Function::Count as i32);
        assert_eq!(aggregate.field, "");
        assert_eq!(clause.operator, having_clause::Operator::GreaterThan as i32);
        assert!(clause.right.is_some(), "the right operand rides the oneof");

        assert_eq!(v1.limit, Some(100));
        assert!(v1.where_clauses.is_empty());
        assert!(
            v1.order_by.is_empty(),
            "order_by is optional and unset here"
        );
        assert_eq!(v1.offset, None);
        assert!(v1.start.is_none());
        assert!(v1.prove, "the Fetch path always requests a proof");
    }

    /// The client-side grammar must resolve the same bounds the server
    /// (and therefore the prover) resolves — the bounds are rebuilt
    /// into the proof's Merk query at verification time, so a client
    /// that translated `> 100` differently could not verify an honest
    /// proof.
    #[test]
    fn assert_having_shape_resolves_the_bounds() {
        let mode = assert_having_shape(&hashtags_over_100(), platform_version())
            .expect("the headline query is well-formed");
        assert_eq!(
            mode.bounds,
            AxisRangeBounds::Count {
                lo: 101,
                hi: u64::MAX
            }
        );
        assert!(!mode.descending, "no order_by means ascending");
        assert_eq!(mode.limit, 100);
        assert_eq!(mode.group_by_property, "hashtag");
    }

    /// An explicit descending ordering on the selected aggregate flips
    /// the walk; biggest matching groups come first.
    #[test]
    fn ordering_by_the_aggregate_sets_the_direction() {
        let query = hashtags_over_100().order_by_selected_aggregate(RankingDirection::Descending);
        let mode = assert_having_shape(&query, platform_version())
            .expect("having + ORDER BY the aggregate is well-formed");
        assert!(mode.descending);
    }

    /// Every knob the range walk cannot honour is rejected client side,
    /// before a round trip — mirroring the server's rejections.
    #[test]
    fn assert_having_shape_rejects_what_the_range_cannot_honour() {
        let base = hashtags_over_100();

        // No having at all: a plain grouped aggregate.
        let mut no_having = base.clone();
        no_having.having = Vec::new();
        assert!(assert_having_shape(&no_having, platform_version()).is_err());

        // Two clauses: implicit AND is a future capability.
        let two = base
            .clone()
            .with_having(vec![count_over_100(), count_over_100()]);
        assert!(assert_having_shape(&two, platform_version()).is_err());

        // A clause on a different aggregate than the select.
        let cross = base.clone().with_having(vec![HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Sum,
                field: "amount".to_string(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(Value::I64(100)),
        }]);
        assert!(assert_having_shape(&cross, platform_version()).is_err());

        // An offset: the range walk has no skip.
        let with_offset = base.clone().with_offset(4);
        assert!(assert_having_shape(&with_offset, platform_version()).is_err());

        // Non-contiguous operators.
        for operator in [HavingOperator::NotEqual, HavingOperator::In] {
            let mut clause = count_over_100();
            clause.operator = operator;
            let query = base.clone().with_having(vec![clause]);
            assert!(assert_having_shape(&query, platform_version()).is_err());
        }
    }

    /// HAVING limits are a hard inclusive range, `1..=100`: `0` (the
    /// unset sentinel) and anything above `MAX_HAVING_LIMIT` are
    /// rejected client side rather than clamped, because the limit
    /// bounds the coverage the verifier's rebuilt `PathQuery` demands
    /// of the proof.
    #[test]
    fn limit_is_required_and_capped_client_side() {
        for limit in [0u32, 101] {
            let query = hashtags_over_100().with_limit(limit);
            assert!(
                assert_having_shape(&query, platform_version()).is_err(),
                "LIMIT {limit} is outside 1..=100 and must be rejected, not clamped"
            );
        }
    }
}
