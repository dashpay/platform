//! Average-query dispatcher entry point.
//!
//! Routes a [`DocumentAverageRequest`] to one of two backends:
//! - **No-prove path** → delegates to the joint count-and-sum
//!   dispatcher
//!   [`Drive::execute_document_count_and_sum_request`], which walks
//!   grovedb ONCE and reads both metrics from each visited
//!   count-sum-bearing element via
//!   [`grovedb::Element::count_sum_value_or_default`]. See its module
//!   docstring for the routing / atomicity contract.
//! - **Prove path** → dispatched to
//!   [`Drive::execute_document_average_prove`] (defined below), which
//!   routes to one of the PCPS / direct-read prove executors based on
//!   `(mode, where_clauses)`. The prove path's per-shape rules are
//!   unchanged.
//!
//! ## Joint dispatch
//!
//! The no-prove dispatcher at
//! [`crate::query::drive_document_count_and_sum_query`] reads
//! `(count, sum)` together — via grovedb's combined
//! `query_aggregate_count_and_sum` accumulator on the aggregate range
//! branch, and via a single PCPS walk on the distinct-grouped branch.
//! Routing reuses sum's versioned mode-detection table so the
//! `(where_clauses × mode)` → executor decision has a single source
//! of truth shared with the count and sum surfaces.
//!
//! ## Prove path shapes (unchanged)
//!
//! The prove-path routing table at
//! [`Self::execute_document_average_prove`] picks one of:
//!     - empty-where + `documentsCountable + documentsSummable`
//!       doctype → primary-key count-sum tree direct read
//!     - range AVG on a `rangeAverageable` index → PCPS
//!       `AggregateCountAndSumOnRange` proof
//!     - In + range AVG on a `rangeAverageable` index → carrier-PCPS
//!       proof
//!     - GroupByRange / GroupByCompound + range on a
//!       `rangeAverageable` index → per-distinct-key
//!       count-and-sum proof (walks `ProvableCountProvableSumTree`
//!       terminators)
//!     - Equal/In + no range on a summable + countable index →
//!       point-lookup count-and-sum proof (walks count-sum-bearing
//!       terminator elements)
//!   The client verifies with the matching
//!   `verify_*_count_and_sum_proof` helpers in `drive-proof-verifier`.

use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_average_query::{
    AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use crate::query::drive_document_sum_query::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use crate::query::drive_document_sum_query::{is_range_operator, DriveDocumentSumQuery};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

#[cfg(feature = "server")]
impl Drive {
    /// Server-side entry point for the average surface.
    ///
    /// Splits prove vs. no-prove at the top level:
    /// - `prove = true` → routes to
    ///   [`Self::execute_document_average_prove`].
    /// - `prove = false` → routes to
    ///   [`Self::execute_document_count_and_sum_request`], the joint
    ///   dispatcher that reads `(count, sum)` together.
    pub fn execute_document_average_request(
        &self,
        request: DocumentAverageRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        if request.prove {
            return self.execute_document_average_prove(request, transaction, platform_version);
        }
        self.execute_document_count_and_sum_request(request, transaction, platform_version)
    }

    /// Prove path of [`Self::execute_document_average_request`].
    ///
    /// Routes the `(where_clauses × mode)` pair to one of the
    /// available PCPS / direct-read prove executors and returns
    /// proof bytes the client verifies with the matching
    /// `verify_*_count_and_sum_proof` helper.
    ///
    /// Supported prove shapes:
    /// - `Aggregate` + empty where + doctype's primary key tree is a
    ///   count-sum-bearing variant (`CountSumTree` /
    ///   `ProvableCountSumTree` /
    ///   `ProvableCountProvableSumTree`) — proves the primary-key
    ///   element directly via `primary_key_sum_path_query`. Client
    ///   verifies with `verify_primary_key_count_sum_tree_proof`.
    /// - `Aggregate` + range clause on a PCPS-eligible index
    ///   (`rangeCountable: true` AND `rangeSummable: true`) — proves
    ///   via `execute_aggregate_count_and_sum_with_proof`. Client
    ///   verifies with `verify_aggregate_count_and_sum_proof`.
    /// - `Aggregate` + Equal/In, no range, on a count+sum index
    ///   (or doctype's count-sum primary key) — proves via
    ///   `execute_point_lookup_sum_with_proof`. Client verifies
    ///   with `verify_point_lookup_count_and_sum_proof`.
    /// - `GroupByIn` + In + range on a PCPS-eligible index — proves
    ///   via `execute_carrier_aggregate_count_and_sum_with_proof`.
    ///   Client verifies with
    ///   `verify_carrier_aggregate_count_and_sum_proof`.
    /// - `GroupByRange` / `GroupByCompound` + range on a PCPS-
    ///   eligible index — proves via
    ///   `execute_distinct_sum_with_proof` against a path query
    ///   whose terminator value trees are
    ///   `ProvableCountProvableSumTree`. Client verifies with
    ///   `verify_distinct_count_and_sum_proof`.
    fn execute_document_average_prove(
        &self,
        request: DocumentAverageRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentAverageResponse, Error> {
        let contract_id = request.contract.id().to_buffer();
        let document_type_name = request.document_type.name().to_string();
        let has_range = request
            .where_clauses
            .iter()
            .any(|wc| is_range_operator(wc.operator));
        let order_by_ascending = request
            .order_clauses
            .first()
            .map(|c| c.ascending)
            .unwrap_or(true);

        // Empty-where AVG fast path: prove the primary-key
        // count-sum-bearing element directly when the doctype
        // declares both `documents_countable: true` (implied by
        // having a CountSumTree primary key) and a matching
        // `documents_summable`. The verifier extracts `(count,
        // sum)` from one element.
        if matches!(request.mode, AverageMode::Aggregate)
            && request.where_clauses.is_empty()
            && request.document_type.documents_countable()
            && request
                .document_type
                .documents_summable()
                .map(|p| p == request.sum_property)
                .unwrap_or(false)
        {
            let path_query =
                DriveDocumentSumQuery::primary_key_sum_path_query(contract_id, &document_type_name);
            let proof = self
                .grove
                .get_proved_path_query(
                    &path_query,
                    None,
                    transaction,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Range AVG: pick a PCPS-eligible index (range_countable
        // AND range_summable) covering the where clauses. Mirror of
        // sum's `find_range_summable_index_for_where_clauses` with
        // an additional `range_countable` filter.
        if has_range
            && matches!(
                request.mode,
                AverageMode::Aggregate | AverageMode::GroupByIn
            )
        {
            let index = find_range_summable_index_for_where_clauses(
                request.document_type.indexes(),
                &request.where_clauses,
                &request.sum_property,
            )
            .filter(|idx| idx.range_countable)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "prove AVG requires an index that declares BOTH `rangeCountable: \
                     true` AND `rangeSummable: true` (a `rangeAverageable: true` \
                     index is the shorthand) whose last property matches the range \
                     field and whose summable property matches the request's \
                     `sum_property`"
                        .to_string(),
                ))
            })?;
            let sum_query = DriveDocumentSumQuery {
                document_type: request.document_type,
                contract_id,
                document_type_name,
                index,
                where_clauses: request.where_clauses.clone(),
                sum_property: request.sum_property.clone(),
            };

            let proof = match request.mode {
                AverageMode::Aggregate => sum_query.execute_aggregate_count_and_sum_with_proof(
                    self,
                    transaction,
                    platform_version,
                )?,
                AverageMode::GroupByIn => {
                    // Carrier-PCPS: one (count, sum) per In branch.
                    // Validate-don't-clamp limit policy on the prove
                    // path — `SizedQuery::limit` is bytes-of-proof
                    // material; silent clamping would byte-differ the
                    // SDK's reconstruction and break verification.
                    // Same contract as sum's `RangeAggregateCarrierProof`
                    // arm. `None` stays `None` (unbounded outer walk).
                    let limit_u16 = request
                        .limit
                        .map(|l| {
                            if l > request.drive_config.max_query_limit as u32 {
                                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                                    "limit {} exceeds max_query_limit {} on the prove + \
                                         carrier-aggregate path (GROUP BY In + range, AVG); \
                                         reduce the requested limit or use prove = false",
                                    l, request.drive_config.max_query_limit
                                ))));
                            }
                            u16::try_from(l).map_err(|_| {
                                Error::Query(QuerySyntaxError::Unsupported(format!(
                                    "limit {} exceeds u16::MAX for carrier-aggregate \
                                     count+sum (AVG) proof",
                                    l
                                )))
                            })
                        })
                        .transpose()?;
                    sum_query.execute_carrier_aggregate_count_and_sum_with_proof(
                        self,
                        limit_u16,
                        order_by_ascending,
                        transaction,
                        platform_version,
                    )?
                }
                _ => unreachable!("outer matches! gate filters out non-Aggregate/GroupByIn"),
            };
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Distinct AVG (GroupByRange / GroupByCompound + range) —
        // per-distinct-key (count, sum) proof against a PCPS-
        // eligible index (rangeCountable + rangeSummable, i.e. a
        // `rangeAverageable: true` index). The prover uses sum's
        // `execute_distinct_sum_with_proof` against a path query
        // whose terminators are `ProvableCountProvableSumTree`; the
        // verifier extracts `count_sum_value_or_default()` from
        // each emitted element.
        if has_range
            && matches!(
                request.mode,
                AverageMode::GroupByRange | AverageMode::GroupByCompound
            )
        {
            let index = find_range_summable_index_for_where_clauses(
                request.document_type.indexes(),
                &request.where_clauses,
                &request.sum_property,
            )
            .filter(|idx| idx.range_countable)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "prove distinct AVG requires an index that declares BOTH \
                     `rangeCountable: true` AND `rangeSummable: true` (a \
                     `rangeAverageable: true` index is the shorthand) whose last \
                     property matches the range field and whose summable property \
                     matches the request's `sum_property`"
                        .to_string(),
                ))
            })?;
            // Validate-don't-clamp limit policy on the prove path —
            // see sum's `RangeDistinctProof` arm for the full
            // rationale. Limit fallback uses
            // [`crate::config::DEFAULT_QUERY_LIMIT`] (compile-time
            // constant) so the SDK's reconstruction lands on the same
            // `SizedQuery::limit` value; `max_query_limit` still
            // gates as a DoS ceiling.
            let effective_limit = request
                .limit
                .unwrap_or(crate::config::DEFAULT_QUERY_LIMIT as u32);
            if effective_limit > request.drive_config.max_query_limit as u32 {
                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                    "limit {} exceeds max_query_limit {} on the prove + distinct-walk \
                     path (GROUP BY a range field, AVG); reduce the requested limit \
                     or use prove = false",
                    effective_limit, request.drive_config.max_query_limit
                ))));
            }
            let limit_u16 = u16::try_from(effective_limit).map_err(|_| {
                Error::Query(QuerySyntaxError::Unsupported(format!(
                    "limit {} exceeds u16::MAX for distinct AVG proof",
                    effective_limit
                )))
            })?;
            let sum_query = DriveDocumentSumQuery {
                document_type: request.document_type,
                contract_id,
                document_type_name,
                index,
                where_clauses: request.where_clauses.clone(),
                sum_property: request.sum_property.clone(),
            };
            let proof = sum_query.execute_distinct_sum_with_proof(
                self,
                limit_u16,
                order_by_ascending,
                transaction,
                platform_version,
            )?;
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Point-lookup AVG: Equal/In on a count+sum index (whose
        // `summable.is_some()` AND `countable.is_countable()`) OR
        // doctype-level documentsSummable + documentsCountable for
        // the empty-where case (handled by the fast path above —
        // this arm handles the non-empty-where Equal/In shape).
        //
        // Accepts both `Aggregate` (caller wants one aggregate row
        // collapsed across all matched In branches — folded
        // client-side by `DocumentAverage`) and `GroupByIn` (caller
        // wants per-In-branch entries — `DocumentSplitAverages`
        // shape). The grovedb-side proof is identical: one walk
        // through the point-lookup `subquery` per In key emits one
        // count-sum-bearing element per branch.
        //
        // Mirrors the sum router's resolved-mode table
        // (`mode_detection/v0/mod.rs`) which maps both
        // `(SumMode::Aggregate, !range, _, true)` and
        // `(SumMode::GroupByIn, !range, _, true)` to
        // `DocumentSumMode::PointLookupProof`. Before adding
        // `GroupByIn` here the SDK could ask drive for a no-range
        // GroupByIn AVG proof, drive would 500 with `Unsupported`,
        // and the SDK's `verify_point_lookup_count_and_sum_proof`
        // arm (gated on the same resolved mode) would never get
        // proof bytes to verify.
        if !has_range
            && matches!(
                request.mode,
                AverageMode::Aggregate | AverageMode::GroupByIn
            )
        {
            let index = find_summable_index_for_where_clauses(
                request.document_type.indexes(),
                &request.where_clauses,
                &request.sum_property,
            )
            .filter(|idx| idx.countable.is_countable())
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "prove point-lookup AVG requires an index that declares BOTH \
                     `summable: \"<prop>\"` AND a countable terminator (`countable: \
                     \"countable\"` or `\"countableAllowingOffset\"`) whose properties \
                     exactly match the where clause fields"
                        .to_string(),
                ))
            })?;
            let sum_query = DriveDocumentSumQuery {
                document_type: request.document_type,
                contract_id,
                document_type_name,
                index,
                where_clauses: request.where_clauses.clone(),
                sum_property: request.sum_property.clone(),
            };
            let proof = sum_query.execute_point_lookup_sum_with_proof(
                self,
                transaction,
                platform_version,
            )?;
            return Ok(DocumentAverageResponse::Proof(proof));
        }

        // Unreachable in practice — the matches!() gates above
        // cover every (mode × has_range) combination today. Kept as
        // a typed error in case a future AverageMode variant lands
        // without a corresponding prove arm.
        Err(Error::Query(QuerySyntaxError::Unsupported(format!(
            "execute_document_average_request prove=true: the (mode = {:?}, has_range \
             = {}) combination is not yet supported on the prove path. \
             This is likely a new AverageMode variant that hasn't been wired \
             into the prove dispatcher.",
            request.mode, has_range,
        ))))
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    // ── Dispatcher limit-policy regression tests ───────────────────
    //
    // AVG-side analogs of count's
    // `test_range_distinct_proof_uses_compile_time_default_query_limit_not_operator_config`
    // and the sum-side tests in `drive_document_sum_query/tests.rs`'s
    // `limit_policy_regression` module. The AVG dispatcher's
    // `RangeDistinctProof` arm mirrors the same validate-don't-clamp
    // policy on the prove path; these tests pin that the dispatcher
    // uses [`crate::config::DEFAULT_QUERY_LIMIT`] (compile-time
    // constant) rather than the operator-tunable
    // `drive_config.default_query_limit`, AND that an explicit
    // `limit > max_query_limit` returns a typed
    // `QuerySyntaxError::InvalidLimit` instead of silently clamping.
    //
    // The AVG distinct path internally calls
    // `execute_distinct_sum_with_proof` (the same primitive sum's
    // RangeDistinctProof uses — see `drive_document_average_query/
    // drive_dispatcher.rs::execute_document_average_prove`); the
    // distinction is the index requirement (`rangeCountable +
    // rangeSummable`, i.e. PCPS / `rangeAverageable`) and the
    // verifier helper (`verify_aggregate_count_and_sum_query`).

    use crate::config::{DriveConfig, DEFAULT_QUERY_LIMIT};
    use crate::drive::Drive;
    use crate::error::query::QuerySyntaxError;
    use crate::query::drive_document_average_query::{
        AverageMode, DocumentAverageRequest, DocumentAverageResponse,
    };
    use crate::query::{WhereClause, WhereOperator};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use grovedb::GroveDb;
    use std::borrow::Cow;
    use std::collections::BTreeMap as StdBTreeMap;

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// v12 contract with a `widget` doctype carrying a single
    /// `(color, amount)` `rangeAverageable: true` (= `rangeCountable +
    /// rangeSummable`) index. The PCPS combined `byColor` index is
    /// what the AVG `RangeDistinctProof` arm walks.
    fn build_widget_contract_pcps() -> dpp::data_contract::DataContract {
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                // rangeAverageable is shorthand for rangeCountable +
                // rangeSummable on the same summable property. The
                // DPP parser desugars it into both flags; the picker
                // routes it through the PCPS path.
                "summable":        "amount",
                "rangeSummable":   true,
                "countable":       "countable",
                "rangeCountable":  true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned()
    }

    fn insert_widget(
        drive: &Drive,
        contract: &dpp::data_contract::DataContract,
        i: usize,
        color: &str,
        amount: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget type exists");
        let mut properties = StdBTreeMap::new();
        properties.insert("color".to_string(), Value::Text(color.to_string()));
        properties.insert("amount".to_string(), Value::U64(amount));
        let document: Document = DocumentV0 {
            id: Identifier::from([(i + 1) as u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
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
        }
        .into();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("insert widget");
    }

    /// AVG mirror of the SUM/count regression: with
    /// `drive_config.default_query_limit = 1` and a `limit = None`
    /// request, the dispatcher must use `DEFAULT_QUERY_LIMIT` (= 100)
    /// for the prove path's `SizedQuery::limit`. If it regressed to
    /// using the runtime `default_query_limit`, the reconstructed
    /// path query would byte-differ and `verify_aggregate_count_and_sum_query`
    /// would return Err — exactly the silent-verify-failure surface
    /// this test guards.
    #[test]
    fn range_distinct_avg_proof_uses_compile_time_default_query_limit_not_operator_config() {
        const OPERATOR_TUNED_LIMIT: u16 = 1;
        assert_ne!(
            DEFAULT_QUERY_LIMIT, OPERATOR_TUNED_LIMIT,
            "test invariant: OPERATOR_TUNED_LIMIT must differ from DEFAULT_QUERY_LIMIT"
        );

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 5),
            ("green", 7),
            ("green", 7),
            ("green", 7),
            ("blue", 2),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");

        let drive_config = DriveConfig {
            default_query_limit: OPERATOR_TUNED_LIMIT,
            ..Default::default()
        };

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue.clone()],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: None,
            prove: true,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed on distinct AVG path");
        let proof_bytes = match response {
            DocumentAverageResponse::Proof(p) => p,
            other => panic!("expected Proof response, got {:?}", other),
        };
        assert!(!proof_bytes.is_empty(), "non-empty proof bytes expected");

        // Reconstruct the path query the way the SDK verifier does
        // — anchored to DEFAULT_QUERY_LIMIT.
        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&color_gt_blue),
            "amount",
        )
        .filter(|idx| idx.range_countable)
        .expect("byColor rangeAverageable index covers `color > blue`");
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: vec![color_gt_blue],
            sum_property: "amount".to_string(),
        };
        let verifier_path_query = sum_query
            .distinct_sum_path_query(Some(DEFAULT_QUERY_LIMIT), true, platform_version)
            .expect("path query builder accepts the same shape the prover used");

        // AVG distinct path's proof verifies via the same
        // `GroveDb::verify_query` shape sum uses — the difference is
        // the PCPS terminator the proof commits, and the SDK extracts
        // (count, sum) from each via `count_sum_value_or_default()`.
        // For this regression test we only need to confirm root-hash
        // recomputation succeeds against the DEFAULT_QUERY_LIMIT-anchored
        // path query; any limit mismatch surfaces as Err here.
        let (_root_hash, _elements) = GroveDb::verify_query(
            &proof_bytes,
            &verifier_path_query,
            &platform_version.drive.grove_version,
        )
        .expect(
            "expected proof to verify against a path query rebuilt with DEFAULT_QUERY_LIMIT; \
             a failure here means the dispatcher signed the AVG proof with the \
             operator-tunable default_query_limit — a consensus-adjacent silent-verify \
             regression",
        );
    }

    /// AVG `RangeDistinctProof` over-max rejection: explicit
    /// `limit > max_query_limit` MUST surface as `InvalidLimit`,
    /// not a silent clamp.
    #[test]
    fn range_distinct_avg_proof_rejects_limit_over_max() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        insert_widget(&drive, &data_contract, 0, "red", 5);

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();
        let over_max = drive_config.max_query_limit as u32 + 1;

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: Some(over_max),
            prove: true,
            drive_config: &drive_config,
        };

        let err = drive
            .execute_document_average_request(request, None, platform_version)
            .expect_err("limit > max_query_limit must reject, not clamp");

        assert!(
            matches!(err, Error::Query(QuerySyntaxError::InvalidLimit(_))),
            "expected QuerySyntaxError::InvalidLimit, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("exceeds max_query_limit"),
            "error must name the rejected limit; got: {msg}"
        );
    }

    /// AVG no-range `GroupByIn` + prove MUST hit the point-lookup
    /// arm and emit proof bytes — the sum router resolves this
    /// shape to `DocumentSumMode::PointLookupProof` and the SDK
    /// helper at `verify_point_lookup_count_and_sum_proof` is the
    /// matching verifier. Before the fix this fell through every
    /// arm in `execute_document_average_prove` and returned
    /// `QuerySyntaxError::Unsupported`, leaving the SDK unable to
    /// finish what it had already started: encode + dispatch a
    /// valid AVG `GroupByIn` request.
    ///
    /// This regression test pins both halves of the contract:
    ///   1. The server returns proof bytes (no fallthrough error).
    ///   2. The proof bytes are bincode-decodable as a `GroveDBProof`
    ///      (sanity-check that it's a real point-lookup payload
    ///      rather than an empty placeholder).
    #[test]
    fn no_range_group_by_in_avg_prove_routes_to_point_lookup() {
        use grovedb::operations::proof::GroveDBProof;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // A `summable + countable` (non-range) index is what the
        // point-lookup AVG arm walks. Build a `widget` doctype with
        // `byColor` index: `summable: "amount" + countable:
        // "countable"`. (No rangeSummable / rangeCountable — those
        // are for the range arms.)
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        insert_widget(&drive, &data_contract, 0, "red", 5);
        insert_widget(&drive, &data_contract, 1, "red", 7);
        insert_widget(&drive, &data_contract, 2, "green", 3);

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        // GroupByIn shape: `color IN ["red", "green"]`, no range,
        // no order. The router maps this to PointLookupProof and
        // the dispatcher must hand back proof bytes (NOT
        // QuerySyntaxError::Unsupported).
        let color_in = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("red".to_string()),
                Value::Text("green".to_string()),
            ]),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByIn,
            limit: None,
            prove: true,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect(
                "no-range GroupByIn AVG + prove must hit the point-lookup arm \
                 (router resolves this shape to DocumentSumMode::PointLookupProof); \
                 a failure here means execute_document_average_prove regressed to \
                 the pre-fix gap that rejected this combination with Unsupported",
            );
        let proof_bytes = match response {
            DocumentAverageResponse::Proof(p) => p,
            other => panic!("expected Proof response, got {:?}", other),
        };
        assert!(
            !proof_bytes.is_empty(),
            "non-empty proof bytes expected from point-lookup AVG path"
        );

        // Decode as a GroveDBProof — sanity-checks that it's a real
        // payload rather than a placeholder. Verification (root-hash
        // recomputation) is exercised end-to-end in the SDK
        // FromProof tests; the dispatcher-level test here just pins
        // the routing decision.
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let _: (GroveDBProof, _) = bincode::decode_from_slice(&proof_bytes, bincode_config)
            .expect("proof bytes must bincode-decode as a GroveDBProof");
    }

    // ── Joint count-and-sum no-prove executor cross-checks ────────
    //
    // Acceptance criterion 4 of issue #3687: "one [test] per joint
    // executor confirming `(count, sum)` match what the current
    // double-dispatch produces, against the same grades-contract
    // fixture."
    //
    // Strategy: for each joint executor (Total / PerInValue /
    // RangeNoProof — and RangeNoProof's distinct branch), issue the
    // AVG no-prove request via `execute_document_average_request`
    // AND independently issue separate count + sum requests under
    // the same transaction. Assert the joint executor's
    // `(count, sum)` matches the zipped pair from the independent
    // count + sum surfaces — a cross-check the joint and per-surface
    // dispatchers cannot silently disagree.

    use crate::query::drive_document_average_query::AverageEntry;
    use crate::query::drive_document_count_query::{
        CountMode, DocumentCountRequest, DocumentCountResponse,
    };
    use crate::query::drive_document_sum_query::{
        DocumentSumRequest, DocumentSumResponse, SumMode,
    };

    /// Issue an independent count + sum pair via the per-surface
    /// dispatchers and return the zipped `(count, sum)` aggregate.
    /// Used as the source of truth for cross-checking the joint
    /// executor's output.
    fn independent_count_sum_aggregate(
        drive: &Drive,
        contract: &dpp::data_contract::DataContract,
        document_type: dpp::data_contract::document_type::DocumentTypeRef,
        sum_property: &str,
        where_clauses: Vec<WhereClause>,
        drive_config: &DriveConfig,
        platform_version: &PlatformVersion,
    ) -> (u64, i64) {
        let count_request = DocumentCountRequest {
            contract,
            document_type,
            where_clauses: where_clauses.clone(),
            order_clauses: Vec::new(),
            mode: CountMode::Aggregate,
            limit: None,
            prove: false,
            drive_config,
        };
        let sum_request = DocumentSumRequest {
            contract,
            document_type,
            sum_property: sum_property.to_string(),
            where_clauses,
            order_clauses: Vec::new(),
            mode: SumMode::Aggregate,
            limit: None,
            prove: false,
            drive_config,
        };
        let count_resp = drive
            .execute_document_count_request(count_request, None, platform_version)
            .expect("independent count");
        let sum_resp = drive
            .execute_document_sum_request(sum_request, None, platform_version)
            .expect("independent sum");
        let count = match count_resp {
            DocumentCountResponse::Aggregate(c) => c,
            other => panic!("expected count Aggregate, got {:?}", other),
        };
        let sum = match sum_resp {
            DocumentSumResponse::Aggregate(s) => s,
            other => panic!("expected sum Aggregate, got {:?}", other),
        };
        (count, sum)
    }

    /// `execute_document_count_and_sum_total_no_proof` cross-check:
    /// empty-where total on a doctype with `documents_summable +
    /// documents_countable`. Goes through the primary-key fast path.
    #[test]
    fn joint_total_executor_matches_independent_count_plus_sum() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // The empty-where Total path requires the doctype's
        // documents_summable + documents_countable to be set, but a
        // covering `summable + countable` byColor index also works
        // for the Equal-only-fully-covered sub-path. Use the latter
        // since the test factory above doesn't easily produce
        // doctype-level summable+countable. The Equal-only branch
        // of execute_document_count_and_sum_total_no_proof still
        // routes through `DocumentSumMode::Total` per sum's table.
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 5),
            ("red", 7),
            ("green", 3),
            ("green", 4),
            ("blue", 1),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        // Aggregate, no where → empty-where Total path. The doctype
        // doesn't declare documents_summable here so the executor
        // fall-through is the picker path on the byColor index. But
        // the empty-where branch requires documents_summable; if the
        // doctype lacks it, the picker is invoked with empty where,
        // which `find_summable_index_for_where_clauses` rejects
        // (zero indexable fields). So we test Equal-only-fully-
        // covered instead — same `DocumentSumMode::Total`
        // resolution.
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("red".to_string()),
        }];

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: where_clauses.clone(),
            order_clauses: Vec::new(),
            mode: AverageMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let joint_response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("joint total dispatch");
        let (joint_count, joint_sum) = match joint_response {
            DocumentAverageResponse::Aggregate { count, sum } => (count, sum),
            other => panic!("expected Aggregate, got {:?}", other),
        };

        let (indep_count, indep_sum) = independent_count_sum_aggregate(
            &drive,
            &data_contract,
            document_type,
            "amount",
            where_clauses,
            &drive_config,
            platform_version,
        );

        assert_eq!(
            (joint_count, joint_sum),
            (indep_count, indep_sum),
            "joint total executor must produce the same (count, sum) as \
             independent count + sum dispatch (red == 3 docs / sum 17)"
        );
        // Sanity check against the fixture: red docs are 5+5+7 = 17 / count 3.
        assert_eq!((joint_count, joint_sum), (3, 17));
    }

    /// `execute_document_count_and_sum_per_in_value_no_proof`
    /// cross-check: In on a `summable + countable` index.
    #[test]
    fn joint_per_in_value_executor_matches_independent_count_plus_sum() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 7),
            ("green", 3),
            ("green", 4),
            ("blue", 1),
            ("blue", 2),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        let color_in = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("red".to_string()),
                Value::Text("green".to_string()),
            ]),
        };

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in.clone()],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let joint_response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("joint per-in-value dispatch");
        let joint_entries = match joint_response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };

        // Cross-check via independent count + sum per-In dispatch.
        let count_request = DocumentCountRequest {
            contract: &data_contract,
            document_type,
            where_clauses: vec![color_in.clone()],
            order_clauses: Vec::new(),
            mode: CountMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let sum_request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in],
            order_clauses: Vec::new(),
            mode: SumMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let count_resp = drive
            .execute_document_count_request(count_request, None, platform_version)
            .expect("independent count");
        let sum_resp = drive
            .execute_document_sum_request(sum_request, None, platform_version)
            .expect("independent sum");
        let count_entries = match count_resp {
            DocumentCountResponse::Entries(e) => e,
            other => panic!("expected count Entries, got {:?}", other),
        };
        let sum_entries = match sum_resp {
            DocumentSumResponse::Entries(e) => e,
            other => panic!("expected sum Entries, got {:?}", other),
        };

        // Zip by key and assert joint matches.
        assert_eq!(joint_entries.len(), count_entries.len());
        assert_eq!(joint_entries.len(), sum_entries.len());
        for ((joint, count), sum) in joint_entries
            .iter()
            .zip(count_entries.iter())
            .zip(sum_entries.iter())
        {
            assert_eq!(joint.key, count.key);
            assert_eq!(joint.key, sum.key);
            assert_eq!(joint.count, count.count);
            assert_eq!(joint.sum, sum.sum);
        }
        // Two entries — red and green.
        assert_eq!(joint_entries.len(), 2);
        // red: 2 docs, sum = 12.
        // green: 2 docs, sum = 7.
        // BTreeMap orders by serialized key bytes (lex on string
        // bytes since color is Text). "green" < "red" lex.
        let mut by_key: Vec<&AverageEntry> = joint_entries.iter().collect();
        by_key.sort_by(|a, b| a.key.cmp(&b.key));
        let red_entry = by_key
            .iter()
            .find(|e| e.key.windows(3).any(|w| w == b"red"))
            .expect("red entry");
        let green_entry = by_key
            .iter()
            .find(|e| e.key.windows(5).any(|w| w == b"green"))
            .expect("green entry");
        assert_eq!(red_entry.count, Some(2));
        assert_eq!(red_entry.sum, Some(12));
        assert_eq!(green_entry.count, Some(2));
        assert_eq!(green_entry.sum, Some(7));
    }

    /// `execute_document_count_and_sum_range_no_proof` cross-check:
    /// distinct GroupByRange on a `rangeAverageable` (PCPS) index.
    #[test]
    fn joint_range_no_proof_executor_matches_independent_count_plus_sum() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 7),
            ("green", 3),
            ("green", 4),
            ("green", 6),
            ("blue", 2),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        // `color > "blue"` on the byColor rangeAverageable index.
        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue.clone()],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let joint_response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("joint range distinct dispatch");
        let joint_entries = match joint_response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };

        // Cross-check via independent count + sum distinct dispatch.
        let count_request = DocumentCountRequest {
            contract: &data_contract,
            document_type,
            where_clauses: vec![color_gt_blue.clone()],
            order_clauses: Vec::new(),
            mode: CountMode::GroupByRange,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let sum_request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue],
            order_clauses: Vec::new(),
            mode: SumMode::GroupByRange,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let count_resp = drive
            .execute_document_count_request(count_request, None, platform_version)
            .expect("independent count");
        let sum_resp = drive
            .execute_document_sum_request(sum_request, None, platform_version)
            .expect("independent sum");
        let count_entries = match count_resp {
            DocumentCountResponse::Entries(e) => e,
            other => panic!("expected count Entries, got {:?}", other),
        };
        let sum_entries = match sum_resp {
            DocumentSumResponse::Entries(e) => e,
            other => panic!("expected sum Entries, got {:?}", other),
        };

        // Both executors emit per-distinct-key entries in ascending
        // serialized-key order; the lengths must match and per-key
        // (count, sum) must zip to the same values.
        assert_eq!(joint_entries.len(), count_entries.len());
        assert_eq!(joint_entries.len(), sum_entries.len());
        for ((joint, count), sum) in joint_entries
            .iter()
            .zip(count_entries.iter())
            .zip(sum_entries.iter())
        {
            assert_eq!(joint.key, count.key);
            assert_eq!(joint.key, sum.key);
            assert_eq!(joint.count, count.count);
            assert_eq!(joint.sum, sum.sum);
        }
        // Two distinct keys (green, red); blue is filtered out by
        // the range. green: 3 docs, sum=13; red: 2 docs, sum=12.
        assert_eq!(joint_entries.len(), 2);
    }

    /// Flat-summed range cross-check: `Aggregate + range` on a PCPS
    /// index resolves to `DocumentSumMode::RangeNoProof` with
    /// `return_distinct_sums_in_range = false`. The joint executor
    /// folds visited PCPS elements via `count_sum_value_or_default()`
    /// in Rust (no engine-side combined accumulator exists). Pin parity
    /// vs. the independent count + sum aggregate dispatch — this is the
    /// path where the issue's perf win lands.
    #[test]
    fn joint_range_aggregate_executor_matches_independent_count_plus_sum() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 7),
            ("green", 3),
            ("green", 4),
            ("green", 6),
            ("blue", 2),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue.clone()],
            order_clauses: Vec::new(),
            mode: AverageMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let joint_response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("joint range aggregate dispatch");
        let (joint_count, joint_sum) = match joint_response {
            DocumentAverageResponse::Aggregate { count, sum } => (count, sum),
            other => panic!("expected Aggregate, got {:?}", other),
        };

        let (indep_count, indep_sum) = independent_count_sum_aggregate(
            &drive,
            &data_contract,
            document_type,
            "amount",
            vec![color_gt_blue],
            &drive_config,
            platform_version,
        );

        assert_eq!(
            (joint_count, joint_sum),
            (indep_count, indep_sum),
            "joint range-aggregate executor must produce the same (count, sum) \
             as independent count + sum range dispatch"
        );
        // Sanity check: color > "blue" matches green (3,4,6 = sum 13)
        // + red (5,7 = sum 12); total 5 docs / sum 25.
        assert_eq!((joint_count, joint_sum), (5, 25));
    }

    /// Compound-summed range cross-check: `GroupByIn + In + range` on
    /// a PCPS index resolves to `DocumentSumMode::RangeNoProof` with
    /// `return_distinct_sums_in_range = false`. The joint executor's
    /// distinct path query expresses the multi-In outer walk as a
    /// single grovedb call (atomicity inherent) and folds each
    /// In-branch's PCPS elements into one `(count, sum)` pair via
    /// `count_sum_value_or_default()`.
    ///
    /// Pin parity vs. the independent count + sum dispatch. This is
    /// the second untested-flat-summed branch the agent's three tests
    /// don't cover.
    #[test]
    fn joint_range_group_by_in_executor_matches_independent_count_plus_sum() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // PCPS index keyed on (color, amount) so In on color + range
        // on amount fits the rangeCountable + rangeSummable shape.
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColorAmount",
                "properties": [{"color": "asc"}, {"amount": "asc"}],
                "summable":        "amount",
                "rangeSummable":   true,
                "countable":       "countable",
                "rangeCountable":  true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("red", 7),
            ("red", 9),
            ("green", 3),
            ("green", 4),
            ("blue", 8),
            ("blue", 9),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        // In on color (red, green) + range on amount (≥ 4).
        let color_in = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("red".to_string()),
                Value::Text("green".to_string()),
            ]),
        };
        let amount_ge_4 = WhereClause {
            field: "amount".to_string(),
            operator: WhereOperator::GreaterThanOrEquals,
            value: Value::U64(4),
        };

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in.clone(), amount_ge_4.clone()],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let joint_response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("joint range GroupByIn dispatch");
        let joint_entries = match joint_response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };

        // Independent count + sum GroupByIn dispatch.
        let count_request = DocumentCountRequest {
            contract: &data_contract,
            document_type,
            where_clauses: vec![color_in.clone(), amount_ge_4.clone()],
            order_clauses: Vec::new(),
            mode: CountMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let sum_request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in, amount_ge_4],
            order_clauses: Vec::new(),
            mode: SumMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };
        let count_resp = drive
            .execute_document_count_request(count_request, None, platform_version)
            .expect("independent count");
        let sum_resp = drive
            .execute_document_sum_request(sum_request, None, platform_version)
            .expect("independent sum");
        let count_entries = match count_resp {
            DocumentCountResponse::Entries(e) => e,
            other => panic!("expected count Entries, got {:?}", other),
        };
        let sum_entries = match sum_resp {
            DocumentSumResponse::Entries(e) => e,
            other => panic!("expected sum Entries, got {:?}", other),
        };

        // The independent count and sum dispatches both produce entries
        // for every In branch (with `count`/`sum` reflecting the In
        // branch's value); the joint executor must produce the same
        // shape. Build a key-keyed map for each and assert pairwise
        // equality on the (count, sum) pair.
        use std::collections::BTreeMap;
        let count_by_key: BTreeMap<Vec<u8>, Option<u64>> = count_entries
            .iter()
            .map(|e| (e.key.clone(), e.count))
            .collect();
        let sum_by_key: BTreeMap<Vec<u8>, Option<i64>> =
            sum_entries.iter().map(|e| (e.key.clone(), e.sum)).collect();
        let joint_by_key: BTreeMap<Vec<u8>, (Option<u64>, Option<i64>)> = joint_entries
            .iter()
            .map(|e| (e.key.clone(), (e.count, e.sum)))
            .collect();

        assert_eq!(
            count_by_key.keys().collect::<Vec<_>>(),
            joint_by_key.keys().collect::<Vec<_>>(),
            "joint executor must emit the same In-branch keys as independent count"
        );
        for (key, (joint_count, joint_sum)) in joint_by_key.iter() {
            assert_eq!(joint_count, count_by_key.get(key).unwrap());
            assert_eq!(joint_sum, sum_by_key.get(key).unwrap());
        }
    }

    /// Distinct AVG no-proof MUST honor the request's `limit` —
    /// `GroupByRange` over a wide range should truncate to the
    /// caller's `limit` rather than enumerate every distinct in-range
    /// terminator. Regression test for the joint dispatcher's
    /// `RangeNoProof` distinct branch: prior to the P2 fix the
    /// dispatcher hard-coded `None` into `distinct_sum_path_query`,
    /// silently returning every matching key.
    #[test]
    fn distinct_avg_no_proof_honors_explicit_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // Five distinct color buckets so a `limit = 2` request must
        // truncate the result set; otherwise the executor would emit
        // all five.
        let docs = [
            ("red", 5u64),
            ("green", 7),
            ("blue", 2),
            ("yellow", 4),
            ("purple", 9),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        let color_ge_a = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThanOrEquals,
            value: Value::Text("a".to_string()),
        };

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_ge_a],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: Some(2),
            prove: false,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed");
        let entries = match response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };
        assert_eq!(
            entries.len(),
            2,
            "distinct AVG no-proof must apply the request's `limit = 2` and \
             return exactly 2 entries; got {entries:?}"
        );
    }

    /// Distinct AVG no-proof with `limit = None` must default to
    /// `drive_config.default_query_limit`, not enumerate every
    /// distinct key. Regression test for the same hard-coded `None`
    /// the prior implementation passed.
    #[test]
    fn distinct_avg_no_proof_defaults_limit_to_operator_default_query_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // Five distinct buckets and an operator-tuned
        // `default_query_limit = 3`. The dispatcher must honor the
        // operator's runtime default on the no-proof path (this is
        // explicitly documented as DIFFERENT from the prove path,
        // which uses the compile-time constant for byte-stability of
        // proof reconstruction). A regression that leaves limit as
        // `None` would emit all 5 entries.
        let docs = [
            ("red", 5u64),
            ("green", 7),
            ("blue", 2),
            ("yellow", 4),
            ("purple", 9),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig {
            default_query_limit: 3,
            ..Default::default()
        };

        let color_ge_a = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThanOrEquals,
            value: Value::Text("a".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_ge_a],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed");
        let entries = match response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };
        assert_eq!(
            entries.len(),
            3,
            "distinct AVG no-proof with `limit = None` must default to \
             `drive_config.default_query_limit` (= 3 here) rather than \
             enumerating all 5 distinct keys; got {entries:?}"
        );
    }

    /// Distinct AVG no-proof with `limit > max_query_limit` must
    /// clamp to `max_query_limit`, not return an error. Mirrors
    /// count's no-proof distinct-walk clamp policy (documented in
    /// `DocumentAverageRequest::limit`).
    #[test]
    fn distinct_avg_no_proof_clamps_limit_to_max_query_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let docs = [
            ("red", 5u64),
            ("green", 7),
            ("blue", 2),
            ("yellow", 4),
            ("purple", 9),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        // Operator-tuned `max_query_limit = 2`. An explicit `limit =
        // 4` MUST clamp to 2 (no-proof policy; the prove path errors
        // on this combination instead — see the
        // `range_distinct_avg_proof_rejects_limit_over_max` test
        // above for the prove counterpart).
        let drive_config = DriveConfig {
            default_query_limit: 100,
            max_query_limit: 2,
            ..Default::default()
        };

        let color_ge_a = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThanOrEquals,
            value: Value::Text("a".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_ge_a],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByRange,
            limit: Some(4),
            prove: false,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed (no-proof clamps, never errors)");
        let entries = match response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };
        assert_eq!(
            entries.len(),
            2,
            "distinct AVG no-proof must clamp `limit = 4` to \
             `max_query_limit = 2`; got {entries:?}"
        );
    }

    /// `execute_document_count_and_sum_request` must reject a direct
    /// caller passing `prove = true`. The wrapper
    /// `execute_document_average_request` is the only legitimate entry
    /// that routes prove requests (to the prove-side dispatcher);
    /// reaching the joint dispatcher with `prove = true` would
    /// otherwise silently produce a no-proof response. Regression for
    /// the CodeRabbit "enforce no-prove precondition" finding.
    #[test]
    fn joint_dispatcher_rejects_prove_true_request() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: Vec::new(),
            order_clauses: Vec::new(),
            mode: AverageMode::Aggregate,
            limit: None,
            prove: true,
            drive_config: &drive_config,
        };

        let err = drive
            .execute_document_count_and_sum_request(request, None, platform_version)
            .expect_err("prove=true direct call must reject");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("no-prove"),
            "expected the prove=true guard to fire; got: {msg}"
        );
    }

    /// AVG no-proof dispatcher must run
    /// `validate_and_canonicalize_where_clauses` so it shares the same
    /// accept/reject contract as the count and document-query
    /// surfaces. Pin a representative rejection: a duplicate Equal on
    /// the same field. Without the validator the executor would
    /// either succeed with a silently-collapsed shape or fail
    /// downstream with a less precise error.
    #[test]
    fn joint_dispatcher_runs_validate_and_canonicalize_where_clauses() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract_pcps();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        // Duplicate Equal on `color` — validator rejects via
        // `WhereClause::group_clauses`.
        let dup_color_a = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("red".to_string()),
        };
        let dup_color_b = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("green".to_string()),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![dup_color_a, dup_color_b],
            order_clauses: Vec::new(),
            mode: AverageMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let err = drive
            .execute_document_average_request(request, None, platform_version)
            .expect_err(
                "AVG no-proof must reject duplicate Equal on the same field via \
                 validate_and_canonicalize_where_clauses",
            );
        // The exact error variant comes from `WhereClause::group_clauses` —
        // pin only that the call returned `Err` and the error mentions
        // the problematic shape rather than a generic index-picker miss.
        let msg = format!("{err:?}");
        assert!(
            !msg.contains("WhereClauseOnNonIndexedProperty"),
            "validator should reject before the index picker would: {msg}"
        );
    }

    /// `PerInValue` no-proof AVG must honor `request.limit` on the
    /// returned entry list. Regression for the reviewer's "joint
    /// dispatcher drops `request.limit`" finding on the PerInValue
    /// arm. Count's per-In executor truncates at this same point.
    #[test]
    fn per_in_value_avg_no_proof_honors_explicit_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // `byColor` index: `summable: "amount"` + `countable:
        // "countable"`. No range flags — this is the no-range
        // PerInValue shape.
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        for (i, (color, amount)) in [("red", 5u64), ("green", 7), ("blue", 2), ("yellow", 4)]
            .iter()
            .enumerate()
        {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();

        // `In` over 4 color values, `limit = 2` — dispatcher must
        // truncate the per-In entry list to 2.
        let color_in = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("red".to_string()),
                Value::Text("green".to_string()),
                Value::Text("blue".to_string()),
                Value::Text("yellow".to_string()),
            ]),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByIn,
            limit: Some(2),
            prove: false,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed");
        let entries = match response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };
        assert_eq!(
            entries.len(),
            2,
            "PerInValue AVG no-proof must apply request.limit = 2 to the per-In \
             entry list (caller asked for 4 In values, dispatcher must truncate); \
             got {entries:?}"
        );
    }

    /// Empty-where `Aggregate` AVG MUST exercise the
    /// [`Drive::execute_document_count_and_sum_total_no_proof`]
    /// primary-key fast path when the doctype declares
    /// `documentsAverageable` (= `documentsCountable: true +
    /// documentsSummable: "<prop>"`). The fast path reads
    /// `[contract_doc, contract_id, [1], doctype, 0]` — the PCPS
    /// primary-key element — and decodes `(count, sum)` from it in one
    /// grovedb call without any index. Consensus-critical: a regression
    /// here would silently produce wrong `(count, sum)` for the
    /// most-trafficked AVG shape (unfiltered total).
    #[test]
    fn empty_where_total_executor_uses_primary_key_count_sum_tree_fast_path() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // `documentsAverageable: "amount"` desugars to BOTH
        // `documentsCountable: true` AND `documentsSummable:
        // "amount"`, which is exactly what the empty-where fast path
        // requires. No `indices` block — the fast path doesn't use
        // an index, it reads the doctype's primary-key
        // count-sum-bearing tree directly at `[..., doctype, 0]`.
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "amount": {"type": "integer", "position": 0, "minimum": 0, "maximum": 1000},
            },
            "required": ["amount"],
            "documentsAverageable": "amount",
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "score": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // Insert documents directly (no need for the widget helper —
        // this doctype has no color property).
        let document_type = data_contract
            .document_type_for_name("score")
            .expect("score type");
        for (i, amount) in [10u64, 20, 30, 40].iter().enumerate() {
            let mut properties = std::collections::BTreeMap::new();
            properties.insert("amount".to_string(), Value::U64(*amount));
            let document: Document = DocumentV0 {
                id: Identifier::from([(i + 1) as u8; 32]),
                owner_id: Identifier::from([0u8; 32]),
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
            }
            .into();
            let storage_flags = Some(std::borrow::Cow::Owned(StorageFlags::SingleEpoch(0)));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&document, storage_flags)),
                            owner_id: None,
                        },
                        contract: &data_contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("insert score");
        }

        let drive_config = DriveConfig::default();
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: Vec::new(),
            order_clauses: Vec::new(),
            mode: AverageMode::Aggregate,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("empty-where AVG no-proof must succeed via the primary-key fast path");
        match response {
            DocumentAverageResponse::Aggregate { count, sum } => {
                assert_eq!(
                    (count, sum),
                    (4, 100),
                    "primary-key count-sum tree fast path must return (4 docs, sum 10+20+30+40 = 100)"
                );
            }
            other => panic!("expected Aggregate, got {:?}", other),
        }
    }

    /// `PerInValue` no-proof AVG with `limit = None` must default to
    /// `drive_config.default_query_limit` per
    /// `DocumentAverageRequest::limit`'s documented contract.
    /// Regression test paired with the explicit-limit case above; pins
    /// the no-proof contract parity reviewers flagged after the
    /// initial PerInValue fix landed.
    #[test]
    fn per_in_value_avg_no_proof_defaults_limit_to_operator_default_query_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Same `summable + countable` `byColor` index as
        // `per_in_value_avg_no_proof_honors_explicit_limit`, but with
        // `default_query_limit = 2` and `limit = None` on the request
        // — the dispatcher must fall back to the operator's runtime
        // default and truncate the per-In entry list to 2.
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":  "amount",
                "countable": "countable",
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let data_contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        for (i, (color, amount)) in [("red", 5u64), ("green", 7), ("blue", 2), ("yellow", 4)]
            .iter()
            .enumerate()
        {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig {
            default_query_limit: 2,
            ..Default::default()
        };

        let color_in = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("red".to_string()),
                Value::Text("green".to_string()),
                Value::Text("blue".to_string()),
                Value::Text("yellow".to_string()),
            ]),
        };
        let request = DocumentAverageRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_in],
            order_clauses: Vec::new(),
            mode: AverageMode::GroupByIn,
            limit: None,
            prove: false,
            drive_config: &drive_config,
        };

        let response = drive
            .execute_document_average_request(request, None, platform_version)
            .expect("dispatcher should succeed");
        let entries = match response {
            DocumentAverageResponse::Entries(e) => e,
            other => panic!("expected Entries, got {:?}", other),
        };
        assert_eq!(
            entries.len(),
            2,
            "PerInValue AVG no-proof with `limit = None` must default to \
             `drive_config.default_query_limit` (= 2 here) and truncate the \
             per-In entry list; got {entries:?}"
        );
    }
}
