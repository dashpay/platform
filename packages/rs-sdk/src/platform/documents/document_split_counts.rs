//! `FromProof` + `Fetch` for [`DocumentSplitCounts`] — the
//! per-group-entry view of the unified `getDocuments` endpoint.
//!
//! Backed by the same [`DocumentQuery`] as
//! [`drive_proof_verifier::DocumentCount`]; the only difference is
//! response shape — `DocumentSplitCounts` returns the full
//! `entries` list keyed by the splitting property's serialized
//! value, while `DocumentCount` returns the sum.
//!
//! Splitting is signalled by:
//! - An `In` where-clause on the request: the field of that clause
//!   becomes the split property and each value in the array
//!   becomes one entry in the result. On the **proof path**,
//!   grovedb's `verify_query` enumerates every queried key and
//!   emits `Some(element)` for present branches and `None` for
//!   absent ones — the drive-level verifier propagates this
//!   directly onto `SplitCountEntry::count` (no SDK-side
//!   synthesis). The **no-proof path** queries each branch and
//!   emits `count: Some(0)` for ones the executor confirmed are
//!   empty.
//! - A range where-clause plus `with_group_by(range_field)`: each
//!   distinct value in the range becomes one entry. Zero-count
//!   ranges are simply absent on both paths — the range itself is
//!   unbounded, so there's no enumerable key set to ever-emit.
//!
//! Without any grouping the response is a single entry with empty
//! `key` (i.e., the total count expressed as one-element entries
//! for shape uniformity). That branch shares dispatch with
//! [`drive_proof_verifier::DocumentCount`] via the shared
//! [`super::count_proof_helpers::verify_aggregate_count`].

use crate::platform::documents::count_proof_helpers::{
    assert_select_is_count, limit_to_u16_or_default, verify_aggregate_count,
};
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::DriveDocumentCountQuery;
use drive_proof_verifier::{
    verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, DocumentSplitCounts, FromProof, SplitCountEntry,
};

impl FromProof<DocumentQuery> for DocumentSplitCounts {
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
        assert_select_is_count(&request)?;
        let response: Self::Response = response.into();

        // Aggregate mode (`select=COUNT, group_by=[]`): a single
        // empty-key entry carrying the verified total. Share the
        // per-shape dispatch with `DocumentCount` via
        // `verify_aggregate_count` — both impls need exactly this
        // verified `u64`, only the wrapping differs.
        if request.group_by.is_empty() {
            let (count, mtd, proof) =
                verify_aggregate_count(request, response, platform_version, provider)?;
            let entries = count.map(|c| {
                vec![SplitCountEntry {
                    in_key: None,
                    key: Vec::new(),
                    count: Some(c),
                }]
            });
            return Ok((entries.map(DocumentSplitCounts::from_verified), mtd, proof));
        }

        // Non-empty `group_by`: per-group entries. Split on
        // whether the request carries a range clause — the proof
        // shape differs (`RangeDistinctProof` for range,
        // `PointLookupProof` for In-on-last).
        let has_range = request
            .where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator));

        if has_range {
            // Range + non-empty group_by (GroupByRange or
            // GroupByCompound): per-distinct-value counts via a
            // regular merk range proof. The proof's `KVCount` ops
            // carry per-`(in_key, key)` counts that the merk root
            // commits to via `node_hash_with_count`.
            let document_type = request
                .data_contract
                .document_type_for_name(&request.document_type_name)
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "document type {} not found in contract: {}",
                        request.document_type_name, e
                    ),
                })?;
            let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                document_type.indexes(),
                &request.where_clauses,
            )
            .ok_or_else(|| drive_proof_verifier::Error::RequestError {
                error: "distinct range count requires a `range_countable: true` index whose \
                        last property matches the range field"
                    .to_string(),
            })?;
            let count_query = DriveDocumentCountQuery {
                document_type,
                contract_id: request.data_contract.id().to_buffer(),
                document_type_name: request.document_type_name.clone(),
                index,
                where_clauses: request.where_clauses.clone(),
            };
            // Limit + direction anchor to the same compile-time
            // constants the server's `RangeDistinctProof` arm uses,
            // so path-query bytes match byte-for-byte across
            // operators.
            let limit_u16 = limit_to_u16_or_default(request.limit)?;
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let proof = response
                .proof()
                .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
            let mtd = response
                .metadata()
                .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;
            let entries = verify_distinct_count_proof(
                &count_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            return Ok((
                Some(DocumentSplitCounts::from_verified(entries)),
                mtd.clone(),
                proof.clone(),
            ));
        }

        // No range, non-empty group_by (GroupByIn): route through
        // the count-tree proof primitives. Two sub-cases:
        //
        // 1. `documents_countable + empty where`: primary-key
        //    CountTree fast path. Single empty-key entry.
        // 2. Else: covering `countable: true` index. Verifier
        //    walks grovedb's `(path, key, Option<Element>)`
        //    triples and emits one `SplitCountEntry` per queried
        //    key — `Some` for present branches, `None` for the
        //    absent ones.
        let document_type = request
            .data_contract
            .document_type_for_name(&request.document_type_name)
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: format!(
                    "document type {} not found in contract: {}",
                    request.document_type_name, e
                ),
            })?;
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

        if request.where_clauses.is_empty() && document_type.documents_countable() {
            let contract_id = request.data_contract.id().to_buffer();
            let count = verify_primary_key_count_tree_proof(
                contract_id,
                &request.document_type_name,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            // `documents_countable` fast path: the proof verified
            // the primary-key CountTree element directly. The
            // returned count IS the verified value (possibly 0
            // for an empty doctype), so emit `Some(_)` rather
            // than `None`.
            let entries = vec![SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                count: Some(count),
            }];
            return Ok((
                Some(DocumentSplitCounts::from_verified(entries)),
                mtd.clone(),
                proof.clone(),
            ));
        }

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove count requires a `countable: true` index whose properties \
                    exactly match the where clause fields, or `documentsCountable: \
                    true` on the document type for unfiltered total counts"
                .to_string(),
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id: request.data_contract.id().to_buffer(),
            document_type_name: request.document_type_name.clone(),
            index,
            where_clauses: request.where_clauses.clone(),
        };
        // The verifier emits one entry per queried key — grovedb's
        // `verify_query` returns `(path, key, Option<Element>)`
        // triples for every key the path query enumerates. `Some`
        // → `count: Some(n)`; `None` → `count: None`. The SDK
        // doesn't synthesize anything beyond what grovedb already
        // provides.
        let entries =
            verify_point_lookup_count_proof(&count_query, proof, mtd, platform_version, provider)?;
        Ok((
            Some(DocumentSplitCounts::from_verified(entries)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl Fetch for DocumentSplitCounts {
    type Request = DocumentQuery;
}
