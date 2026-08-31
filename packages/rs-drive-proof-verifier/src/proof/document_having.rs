//! Verified **having-range**
//! (`GROUP BY … HAVING <aggregate> <op> <value> LIMIT n`) document
//! results.
//!
//! A having-range query answers "which groups' aggregate falls inside a
//! value bound?" — `SELECT COUNT(*) GROUP BY hashtag HAVING $count > 100
//! LIMIT 100`. The answer is a value-bounded range read of the same
//! per-axis *secondary* Merk the ranked query walks, so it costs
//! `O(log n + k)` and comes with a proof that commits to exactly the
//! returned `(aggregate, group key)` pairs **and their completeness**:
//! the Merk range proof commits its boundaries, so an in-range group the
//! node omitted fails verification.
//!
//! This module holds the client-facing result type
//! ([`DocumentHavingEntries`]), the tenderdash-composition wrapper
//! around rs-drive's merk-level verifier
//! ([`verify_having_range_proof`]), and the decoder for the unproven
//! wire payload ([`DocumentHavingEntries::from_unproved_response`]) —
//! which rides the same `ResultData.ranked` variant the ranked surface
//! uses, since a having page is the same "group key + aggregate value"
//! entry list.
//!
//! Per-shape routing (which index covers the axis, which bounds the
//! clause translates to) lives in rs-sdk's `having_proof_helpers`,
//! exactly as the ranked equivalents live in `ranked_proof_helpers` —
//! it needs the data contract, which this crate does not carry.

use crate::error::MapGroveDbError;
use crate::proof::document_ranked::{ranked_entry_from_proto, result_variant_name};
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    result_data, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::{DriveDocumentHavingQuery, DriveDocumentQuery, RankedEntry};
use drive::verify::RootHash;

/// One page of a `GROUP BY … HAVING <aggregate> <op> <value> LIMIT n`
/// query: the groups whose aggregate falls inside the bound.
///
/// **Entry order is axis order in the walk direction** — ascending by
/// default, descending when the request ordered by the aggregate
/// descending. Callers must not re-sort; ties (groups with equal
/// aggregates) come back in group-key order in the direction of the
/// walk, same as on the ranked surface.
///
/// Fewer than `n` entries means fewer groups matched — not an error.
/// **Exactly `n` entries may mean the match set was cut at the limit**;
/// nothing in the page marks the cut. Tightening the bound past the
/// last aggregate value seen continues past *distinct* values only — a
/// cut inside a tie (several groups sharing the boundary aggregate)
/// cannot be continued, so size the limit above the widest expected
/// tie.
///
/// Entry semantics ([`RankedEntry`]) are identical to the ranked
/// surface's, including the fixed-point average scaling and the
/// exact-on-the-proved-path-only caveat.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentHavingEntries {
    /// The matching groups, in axis order in the walk direction.
    pub entries: Vec<RankedEntry>,
}

impl DocumentHavingEntries {
    /// Build a [`DocumentHavingEntries`] from the verifier-side entry
    /// list — the shape rs-drive's merk-level verifier returns.
    pub fn from_verified(entries: Vec<RankedEntry>) -> Self {
        DocumentHavingEntries { entries }
    }

    /// Decode the **unproven** having-range payload of a `getDocuments`
    /// response — the `ResultData.ranked` variant a node returns for a
    /// having-range request sent with `prove = false`. (The wire reuses
    /// the ranked entries message; a having page leaves its `skipped`
    /// field unset, and this decoder ignores it either way, because a
    /// value-bounded page has no rank base for it to describe.)
    ///
    /// Order is preserved verbatim. This is a plain wire decode with
    /// **no cryptographic guarantee whatsoever** — and unlike the ranked
    /// surface the missing guarantee here includes *completeness*: an
    /// unproven page is free to omit matching groups, which for a
    /// spam-resistance query is precisely the interesting attack. Prefer
    /// [`verify_having_range_proof`] (via rs-sdk's
    /// `DocumentHavingEntries::fetch`) unless you deliberately trust the
    /// node.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyVersion`] when the response carries no version.
    /// - [`Error::ResponseDecodeError`] when the response is a V0
    ///   response, carries a proof rather than data, carries a
    ///   non-ranked `ResultData` variant, or an entry's `value` oneof is
    ///   unset / out of domain.
    pub fn from_unproved_response(
        response: &GetDocumentsResponse,
    ) -> Result<(Self, ResponseMetadata), Error> {
        let version = response.version.as_ref().ok_or(Error::EmptyVersion)?;
        let ResponseVersion::V1(v1) = version else {
            return Err(Error::ResponseDecodeError {
                error: "having-range results are a V1-only response shape; got a V0 \
                        getDocuments response. Having-range queries require protocol \
                        version 14+."
                    .to_string(),
            });
        };
        let metadata = v1.metadata.clone().ok_or(Error::EmptyResponseMetadata)?;
        let entries = match v1.result.as_ref() {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Ranked(ranked)),
            })) => ranked
                .entries
                .iter()
                .map(ranked_entry_from_proto)
                .collect::<Result<Vec<_>, _>>()?,
            Some(get_documents_response_v1::Result::Proof(_)) => {
                return Err(Error::ResponseDecodeError {
                    error: "the response carries a proof, not unproven having-range entries; \
                            verify it with `verify_having_range_proof` instead of decoding it"
                        .to_string(),
                });
            }
            other => {
                return Err(Error::ResponseDecodeError {
                    error: format!(
                        "expected a `ResultData.ranked` payload for a having-range request, \
                         got {}. A response on another variant means the node routed \
                         the request to a different executor — check that the request \
                         carries a `group_by` and exactly one `having` clause bounding the \
                         single `select`'s aggregate.",
                        result_variant_name(other)
                    ),
                });
            }
        };
        Ok((DocumentHavingEntries { entries }, metadata))
    }
}

/// Verify a grovedb indexed-axis range proof **and the surrounding
/// tenderdash commit**, returning the reconstructed root hash and the
/// matching groups it commits to.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentHavingQuery::verify_having_range_proof`] in rs-drive
/// (which does the merk-level verification). Both sides derive the
/// proved subtree from the same
/// `DriveDocumentHavingQuery::indexed_property_name_tree_path` and the
/// bounded traversal from the same
/// `AxisRangeBounds::inclusive_bounds_i128`, so prover and verifier
/// cannot drift on *which bound over which tree* is being checked, and
/// grovedb re-executes the proof against that reconstruction — a proof
/// of one bound does not cover another (the limit binds as a cap: an
/// exhausted proof verifies under any admitting cap, a truncated one
/// fails a larger cap for missing coverage).
///
/// ## The root hash is the whole point
///
/// Same as on the ranked surface: the merk-level verifier returning
/// `Ok` is not by itself evidence of anything — the binding to the
/// quorum-signed app hash in [`verify_tenderdash_proof`] is what makes
/// the entries (and their completeness) attested facts. This function
/// exists so that composition can never be skipped by accident.
pub fn verify_having_range_proof(
    query: &DriveDocumentHavingQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, Vec<RankedEntry>), Error> {
    let (root_hash, entries) = query
        .verify_having_range_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((root_hash, entries))
}

/// Reject the generic [`FromProof`] entry point for
/// [`DocumentHavingEntries`] — same guard rail, same rationale as the
/// [`crate::DocumentRankedEntries`] blanket impl: the generic
/// `FromProof<Q: TryInto<DriveDocumentQuery>>` path carries neither the
/// bounds nor the covering index, so it errors out explicitly rather
/// than verifying the wrong thing.
impl<'dq, Q> FromProof<Q> for DocumentHavingEntries
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        _request: I,
        _response: O,
        _network: Network,
        _platform_version: &PlatformVersion,
        _provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        Err(Error::RequestError {
            error: "DocumentHavingEntries can't be verified via the generic FromProof path; \
                 call DocumentHavingEntries::fetch on a DocumentQuery carrying \
                 .with_select(<aggregate>), .with_group_by(<property>), \
                 .with_having(<one clause bounding the selected aggregate>) and \
                 .with_limit(n), which resolves the bounds and the covering index from \
                 the data contract"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the unproven decode and the response-shape
    //! rejections. Proof verification is exercised end-to-end by
    //! rs-drive's `drive_document_having_query::tests` (prover and
    //! merk-level verifier against a real Drive), rs-drive-abci's
    //! `having_range_tests` (wire encoding of the same values), and
    //! rs-drive-abci's `having_trust_boundary` suite, which runs this
    //! crate's [`verify_having_range_proof`] wrapper — including the
    //! tenderdash signature binding — against server-generated proofs.
    //! The tenderdash-composition tests live on the server side so this
    //! client crate keeps building drive with `verify` only.
    use super::*;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
        ranked_entry, Documents, RankedEntries, RankedEntry as ProtoRankedEntry,
    };
    use dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV1;
    use drive::query::RankedEntryValue;

    fn count_entry(key: &str, count: u64) -> ProtoRankedEntry {
        ProtoRankedEntry {
            in_key: None,
            key: key.as_bytes().to_vec(),
            value: Some(ranked_entry::Value::Count(count)),
        }
    }

    fn response_with(result: get_documents_response_v1::Result) -> GetDocumentsResponse {
        GetDocumentsResponse {
            version: Some(ResponseVersion::V1(GetDocumentsResponseV1 {
                result: Some(result),
                metadata: Some(ResponseMetadata {
                    height: 42,
                    ..Default::default()
                }),
                proven_join_values: Vec::new(),
            })),
        }
    }

    fn having_response(
        entries: Vec<ProtoRankedEntry>,
        skipped: Option<u64>,
    ) -> GetDocumentsResponse {
        response_with(get_documents_response_v1::Result::Data(ResultData {
            variant: Some(result_data::Variant::Ranked(RankedEntries {
                entries,
                skipped,
            })),
        }))
    }

    /// The headline decode: `HAVING $count > 100`-shaped entries come
    /// back in axis order, untouched, with `skipped` (unset on a
    /// having page) ignored.
    #[test]
    fn decodes_entries_preserving_axis_order() {
        let response = having_response(
            vec![count_entry("dash", 101), count_entry("evo", 250)],
            None,
        );
        let (decoded, metadata) = DocumentHavingEntries::from_unproved_response(&response)
            .expect("a well-formed having payload decodes");
        assert_eq!(metadata.height, 42);
        assert_eq!(
            decoded.entries.iter().map(|e| e.value).collect::<Vec<_>>(),
            vec![RankedEntryValue::Count(101), RankedEntryValue::Count(250)]
        );
    }

    /// A stray `skipped` from a non-conforming node is ignored, not a
    /// decode failure: the field cannot describe anything on a
    /// value-bounded page, and failing on it would break against a
    /// node that reused its ranked encoder wholesale.
    #[test]
    fn a_stray_skipped_field_is_ignored() {
        let response = having_response(vec![count_entry("dash", 101)], Some(7));
        let (decoded, _) = DocumentHavingEntries::from_unproved_response(&response)
            .expect("a stray skipped is not a decode failure");
        assert_eq!(decoded.entries.len(), 1);
    }

    /// No groups matching the bound is a legitimate answer.
    #[test]
    fn decodes_an_empty_match_set() {
        let (decoded, _) =
            DocumentHavingEntries::from_unproved_response(&having_response(vec![], None))
                .expect("an empty match set is well-formed");
        assert!(decoded.entries.is_empty());
    }

    /// Same caller-mistake guard as the ranked decoder: a proof must
    /// be verified, not decoded.
    #[test]
    fn rejects_a_proof_response() {
        let response = response_with(get_documents_response_v1::Result::Proof(Proof::default()));
        let err = DocumentHavingEntries::from_unproved_response(&response)
            .expect_err("a proof is not an unproven having payload");
        assert!(format!("{err}").contains("verify_having_range_proof"));
    }

    /// A response on another variant means the node routed the request
    /// somewhere else entirely.
    #[test]
    fn rejects_a_non_ranked_result_variant() {
        let response = response_with(get_documents_response_v1::Result::Data(ResultData {
            variant: Some(result_data::Variant::Documents(Documents {
                documents: Vec::new(),
            })),
        }));
        let err = DocumentHavingEntries::from_unproved_response(&response)
            .expect_err("a documents payload is not a having one");
        assert!(format!("{err}").contains("ResultData.ranked"));
    }

    /// V0 predates the SQL-shaped surface entirely.
    #[test]
    fn rejects_a_v0_response() {
        let response = GetDocumentsResponse {
            version: Some(ResponseVersion::V0(Default::default())),
        };
        let err = DocumentHavingEntries::from_unproved_response(&response)
            .expect_err("V0 has no having shape");
        assert!(format!("{err}").contains("V1-only"));
    }
}
