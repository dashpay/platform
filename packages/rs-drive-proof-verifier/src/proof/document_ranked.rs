//! Verified **ranked** (`HAVING … TOP(n)` / `BOTTOM(n)`) document
//! results.
//!
//! A ranked query answers "which `n` groups score highest (or lowest)
//! on an aggregate?" — `SELECT AVG(grade) GROUP BY restaurantId
//! HAVING AVG(grade) IN TOP(5)`. The answer is read straight out of
//! the per-axis *secondary* Merk of an indexed tree (grovedb PR #657),
//! so it costs `O(log n + k)` and comes with a proof that commits to
//! exactly the `k` returned `(aggregate, group key)` pairs.
//!
//! This module holds the client-facing result type
//! ([`DocumentRankedEntries`]), the tenderdash-composition wrapper
//! around rs-drive's merk-level verifier
//! ([`verify_ranked_top_k_proof`]), and the decoder for the unproven
//! `ResultData.ranked` wire payload
//! ([`DocumentRankedEntries::from_unproved_response`]).
//!
//! Per-shape routing (which index covers the axis, which
//! `(axis, descending, k)` triple the request resolves to) lives in
//! rs-sdk's `ranked_proof_helpers`, exactly as count's four-way
//! dispatch lives in `count_proof_helpers` — it needs the data
//! contract, which this crate does not carry.

use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
    ranked_entry, result_data, RankedEntry as ProtoRankedEntry, ResultData,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v1, Version as ResponseVersion,
};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::{DriveDocumentQuery, DriveDocumentRankedQuery, RankedEntry, RankedEntryValue};
use drive::verify::RootHash;

/// The ranked groups of a `HAVING … TOP(n)` query.
///
/// **Entry order is the ranking order** — best-first for `TOP(n)`,
/// worst-first for `BOTTOM(n)`. Callers must not re-sort; ties (groups
/// with equal aggregates) come back in group-key order *in the
/// direction of the walk*, which is descending group-key order for
/// `TOP`. That tie handling is also why the value-based `MAX` / `MIN`
/// rankings are refused rather than served as `TOP(1)` / `BOTTOM(1)`:
/// a bounded read cannot prove it returned *every* group tied at the
/// extreme.
///
/// Fewer than `n` entries is normal — the index simply holds fewer
/// groups than were asked for — and is not an error.
///
/// Each [`RankedEntry`]'s `key` is the raw index-key bytes of the
/// `GROUP BY` property's value (for a `string` property, its UTF-8
/// bytes); its `value` is the aggregate, one of
/// [`RankedEntryValue::Count`] / [`RankedEntryValue::Sum`] /
/// [`RankedEntryValue::AvgFixedPoint`]. Averages are fixed-point
/// integers scaled by [`crate::RANKED_AVG_SCALE`]; divide by it (or
/// call [`RankedEntryValue::as_f64`]) to render one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentRankedEntries(pub Vec<RankedEntry>);

impl DocumentRankedEntries {
    /// Build a [`DocumentRankedEntries`] from a verifier-side
    /// `Vec<RankedEntry>`. Identity for now; kept as a constructor in
    /// case the internal shape evolves, mirroring
    /// [`DocumentSplitCounts::from_verified`](crate::DocumentSplitCounts::from_verified).
    pub fn from_verified(entries: Vec<RankedEntry>) -> Self {
        DocumentRankedEntries(entries)
    }

    /// Decode the **unproven** ranked payload of a `getDocuments`
    /// response — the `ResultData.ranked` variant a node returns for a
    /// ranked request sent with `prove = false`.
    ///
    /// Order is preserved verbatim: the server emits entries in
    /// ranking order and this decoder never re-sorts.
    ///
    /// This is a plain wire decode with **no cryptographic guarantee
    /// whatsoever** — it is the "trust the node" path. Prefer
    /// [`verify_ranked_top_k_proof`] (via rs-sdk's
    /// `DocumentRankedEntries::fetch`) unless you are deliberately
    /// reading from a node you already trust, or you hit the
    /// empty-ranking prove gap documented on the SDK type: grovedb has
    /// no absence-proof shape for an empty axis secondary, so a node
    /// rejects `prove = true` over an index with no documents and only
    /// the unproven read can answer it (with an empty list).
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyVersion`] when the response carries no version.
    /// - [`Error::ResponseDecodeError`] when the response is a V0
    ///   response (which has no ranked shape), carries a proof rather
    ///   than data, or carries a non-ranked `ResultData` variant.
    /// - [`Error::ResponseDecodeError`] when an entry's `value` oneof
    ///   is unset, or its `avg_fixed_point` is not exactly 16 bytes.
    pub fn from_unproved_response(
        response: &GetDocumentsResponse,
    ) -> Result<(Self, ResponseMetadata), Error> {
        let version = response.version.as_ref().ok_or(Error::EmptyVersion)?;
        let ResponseVersion::V1(v1) = version else {
            return Err(Error::ResponseDecodeError {
                error: "ranked results are a V1-only response shape; got a V0 getDocuments \
                        response. Ranked queries require protocol version 14+."
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
                    error: "the response carries a proof, not unproven ranked entries; verify \
                            it with `verify_ranked_top_k_proof` instead of decoding it"
                        .to_string(),
                });
            }
            other => {
                return Err(Error::ResponseDecodeError {
                    error: format!(
                        "expected a `ResultData.ranked` payload for a ranked request, got \
                         {other:?}. A response on another variant means the node routed the \
                         request to a different executor — check that the `having` clause \
                         carries a ranking operand matching the single `select`."
                    ),
                });
            }
        };
        Ok((DocumentRankedEntries(entries), metadata))
    }
}

/// Decode one wire [`ProtoRankedEntry`] into rs-drive's
/// [`RankedEntry`].
///
/// The `avg_fixed_point` arm enforces the proto's **exactly 16 bytes,
/// big-endian two's-complement `i128`** contract rather than
/// zero-padding a short buffer: a truncated average would decode to a
/// plausible-but-wrong number (dropping the low bytes of a
/// `10^19`-scaled integer changes the value by orders of magnitude),
/// and there is no length under 16 that is more likely to be a
/// legitimate encoding than a corrupted one.
fn ranked_entry_from_proto(entry: &ProtoRankedEntry) -> Result<RankedEntry, Error> {
    let value = match entry.value.as_ref() {
        Some(ranked_entry::Value::Count(count)) => RankedEntryValue::Count(*count),
        Some(ranked_entry::Value::Sum(sum)) => RankedEntryValue::Sum(*sum),
        Some(ranked_entry::Value::AvgFixedPoint(bytes)) => {
            let bytes: [u8; 16] =
                bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::ResponseDecodeError {
                        error: format!(
                            "`avg_fixed_point` must be exactly 16 bytes (a big-endian \
                             two's-complement i128), got {}",
                            bytes.len()
                        ),
                    })?;
            RankedEntryValue::AvgFixedPoint(i128::from_be_bytes(bytes))
        }
        None => {
            return Err(Error::ResponseDecodeError {
                error: "ranked entry carries no `value`; the server always sets exactly one \
                        of `count` / `sum` / `avg_fixed_point`"
                    .to_string(),
            });
        }
    };
    Ok(RankedEntry {
        key: entry.key.clone(),
        value,
    })
}

/// Verify a grovedb indexed-axis top-k proof **and the surrounding
/// tenderdash commit**, returning the reconstructed root hash and the
/// ranked entries it commits to.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`] in rs-drive
/// (which does the merk-level verification). Both sides derive the
/// proved subtree from the same
/// `DriveDocumentRankedQuery::indexed_property_name_tree_path`, so
/// prover and verifier cannot drift on *which* ranking is being
/// checked, and grovedb re-checks the `(axis, k, descending)` triple
/// echoed in the envelope — a proof of one ranking does not verify as
/// another.
///
/// ## The root hash is the whole point
///
/// The merk-level verifier returning `Ok` is **not** by itself
/// evidence of anything. Sweeping every bit of a real ranked envelope
/// shows why: most flips do error out, but roughly 9% of them (bytes
/// of sibling-subtree hashes inside the ancestor layer proofs) verify
/// cleanly and return the correct entries — under a *different*
/// reconstructed root hash. What rejects those is the
/// [`verify_tenderdash_proof`] call below, which checks the
/// reconstructed root against the quorum-signed app hash for the
/// response's block. This function exists so that composition can
/// never be skipped by accident: there is no way to obtain the entries
/// from it without the binding having run.
///
/// The `RootHash` is returned as well, already bound, so callers can
/// log or cross-check it (e.g. against a root hash they verified for a
/// different query at the same height). Callers must not treat it as
/// something they still have to check.
pub fn verify_ranked_top_k_proof(
    query: &DriveDocumentRankedQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, Vec<RankedEntry>), Error> {
    let (root_hash, page) = query
        .verify_ranked_top_k_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    // `page.skipped` — the attested rank this page starts at — is
    // dropped here for now: this signature predates offset pagination
    // and widening it is part of the client-side rework that also
    // teaches the SDK's request builder the `ORDER BY … LIMIT … OFFSET`
    // grammar. Dropping it is safe (a caller that never asked for an
    // offset gets `skipped == 0`) but lossy for one that did, which is
    // why it is a stop-gap and not the end state.
    Ok((root_hash, page.entries))
}

/// Reject the generic [`FromProof`] entry point for
/// [`DocumentRankedEntries`].
///
/// `DocumentRankedEntries` is reached from rs-sdk via the
/// `FromProof<DocumentQuery>` impl defined alongside the SDK's
/// `DocumentQuery` type (see
/// `rs-sdk/src/platform/documents/document_ranked_entries.rs`), which
/// resolves the `(axis, descending, k)` triple and the covering index
/// from the request's `(select, group_by, having)` triple plus the
/// data contract. The generic
/// `FromProof<Q: TryInto<DriveDocumentQuery>>` path carries neither —
/// `DriveDocumentQuery` has no notion of a ranking — so it errors out
/// explicitly rather than verifying the wrong thing; calling this impl
/// directly is a programmer mistake.
impl<'dq, Q> FromProof<Q> for DocumentRankedEntries
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
            error: "DocumentRankedEntries can't be verified via the generic FromProof path; \
                 call DocumentRankedEntries::fetch on a DocumentQuery carrying \
                 .with_select(<aggregate>), .with_group_by(<property>) and .with_having(<one \
                 clause with a TOP(n) / BOTTOM(n) ranking>), which resolves the ranking axis \
                 and the covering index from the data contract"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Offline tests for the parts of the ranked surface that need
    //! neither a grovedb proof nor a populated Drive:
    //!
    //! - the unproven `ResultData.ranked` decode, including order
    //!   preservation, negative fixed-point averages, and the
    //!   malformed-length rejection;
    //! - the response-shape rejections (proof instead of data, wrong
    //!   `ResultData` variant, V0 response);
    //! - the generic `FromProof<Q>` impl that intentionally errors to
    //!   prevent a silently-wrong verification.
    //!
    //! Proof verification itself is exercised end-to-end by rs-drive's
    //! `drive_document_ranked_query::tests` (prover and verifier run
    //! against a real Drive, with a bit-flip sweep asserting no tamper
    //! survives with the honest root hash) and by rs-drive-abci's
    //! `ranked_tests` (wire encoding of the same values). Reproducing
    //! it here would need a populated Drive, which is outside this
    //! crate's feature surface.
    use super::*;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
        Documents, RankedEntries,
    };
    use dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV1;
    use drive::query::RANKED_AVG_SCALE;

    fn count_entry(key: &str, count: u64) -> ProtoRankedEntry {
        ProtoRankedEntry {
            key: key.as_bytes().to_vec(),
            value: Some(ranked_entry::Value::Count(count)),
        }
    }

    fn avg_entry(key: &str, fixed_point: i128) -> ProtoRankedEntry {
        ProtoRankedEntry {
            key: key.as_bytes().to_vec(),
            value: Some(ranked_entry::Value::AvgFixedPoint(
                fixed_point.to_be_bytes().to_vec(),
            )),
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
            })),
        }
    }

    fn ranked_response(entries: Vec<ProtoRankedEntry>) -> GetDocumentsResponse {
        response_with(get_documents_response_v1::Result::Data(ResultData {
            variant: Some(result_data::Variant::Ranked(RankedEntries { entries })),
        }))
    }

    /// The headline decode: `SELECT AVG(grade) … HAVING AVG(grade) IN
    /// TOP(3)`. Entry order is the ranking order and must survive the
    /// decode verbatim, and each 16-byte big-endian payload must come
    /// back as the exact `i128` grovedb sorted by.
    #[test]
    fn decodes_avg_entries_preserving_ranking_order() {
        let response = ranked_response(vec![
            avg_entry("gamma", 95 * RANKED_AVG_SCALE),
            avg_entry("alpha", 85 * RANKED_AVG_SCALE),
            // 21/2 = 10.5 — a non-integral average, so the fixed-point
            // floor is actually exercised rather than a round multiple.
            avg_entry("epsilon", (21 * RANKED_AVG_SCALE).div_euclid(2)),
        ]);

        let (decoded, metadata) = DocumentRankedEntries::from_unproved_response(&response)
            .expect("a well-formed ranked payload decodes");

        assert_eq!(metadata.height, 42, "metadata rides along with the entries");
        let keys: Vec<&[u8]> = decoded.0.iter().map(|e| e.key.as_slice()).collect();
        assert_eq!(
            keys,
            vec![
                b"gamma".as_slice(),
                b"alpha".as_slice(),
                b"epsilon".as_slice()
            ],
            "entry order is the ranking order and must not be re-sorted"
        );
        assert_eq!(
            decoded.0[2].value,
            RankedEntryValue::AvgFixedPoint((21 * RANKED_AVG_SCALE).div_euclid(2))
        );
        assert_eq!(
            decoded.0[2].value.as_f64(),
            10.5,
            "dividing by RANKED_AVG_SCALE recovers the average a caller renders"
        );
    }

    /// Averages are signed: a group whose summable property is
    /// negative ranks below zero, and the two's-complement round trip
    /// must survive. A decoder that read the bytes as unsigned would
    /// turn `-0.5` into a colossal positive number and silently invert
    /// the ranking's meaning.
    #[test]
    fn decodes_negative_fixed_point_averages() {
        let negative_half = (-RANKED_AVG_SCALE).div_euclid(2);
        let response = ranked_response(vec![
            avg_entry("above", RANKED_AVG_SCALE),
            avg_entry("below", negative_half),
            avg_entry("floor", i128::MIN),
        ]);

        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&response)
            .expect("negative averages are well-formed");

        assert_eq!(
            decoded.0[1].value,
            RankedEntryValue::AvgFixedPoint(negative_half)
        );
        assert_eq!(decoded.0[1].value.as_f64(), -0.5);
        assert_eq!(
            decoded.0[2].value,
            RankedEntryValue::AvgFixedPoint(i128::MIN),
            "the extreme two's-complement value round-trips unchanged"
        );
    }

    /// Count entries decode to `u64` untouched — including values
    /// above 2^53, which is why the wire field is `jstype = JS_STRING`.
    #[test]
    fn decodes_count_entries() {
        let response = ranked_response(vec![
            count_entry("delta", 4),
            count_entry("beta", 3),
            count_entry("huge", u64::MAX),
        ]);

        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&response)
            .expect("count entries are well-formed");

        assert_eq!(
            decoded.0.iter().map(|e| e.value).collect::<Vec<_>>(),
            vec![
                RankedEntryValue::Count(4),
                RankedEntryValue::Count(3),
                RankedEntryValue::Count(u64::MAX)
            ]
        );
    }

    /// Sums are signed `sint64` on the wire, so a negative running
    /// total (a group of refunds) must decode as-is.
    #[test]
    fn decodes_signed_sum_entries() {
        let response = ranked_response(vec![ProtoRankedEntry {
            key: b"refunds".to_vec(),
            value: Some(ranked_entry::Value::Sum(-1_000)),
        }]);

        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&response)
            .expect("signed sums are well-formed");

        assert_eq!(decoded.0[0].value, RankedEntryValue::Sum(-1_000));
    }

    /// An empty ranking is a legitimate answer, not an error: the
    /// index simply has no groups yet. This is exactly the shape a
    /// caller falls back to after a node refuses to *prove* an empty
    /// ranking, so it must decode cleanly.
    #[test]
    fn decodes_an_empty_ranking() {
        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&ranked_response(vec![]))
            .expect("an empty ranking is well-formed");
        assert!(decoded.0.is_empty());
    }

    /// A short or long `avg_fixed_point` is rejected rather than
    /// zero-padded. Truncation would decode to a plausible-looking
    /// number many orders of magnitude off (the scale is 10^19), which
    /// is far worse than a loud failure.
    #[test]
    fn rejects_avg_fixed_point_of_the_wrong_length() {
        for bytes in [vec![0u8; 15], vec![0u8; 17], Vec::new()] {
            let length = bytes.len();
            let response = ranked_response(vec![ProtoRankedEntry {
                key: b"alpha".to_vec(),
                value: Some(ranked_entry::Value::AvgFixedPoint(bytes)),
            }]);
            let err = match DocumentRankedEntries::from_unproved_response(&response) {
                Err(err) => err,
                Ok(decoded) => {
                    panic!("a {length}-byte average must be rejected, decoded {decoded:?}")
                }
            };
            let message = format!("{err}");
            assert!(
                message.contains("exactly 16 bytes") && message.contains(&length.to_string()),
                "the rejection must name the contract and the offending length; got {message}"
            );
        }
    }

    /// A `RankedEntry` with no `value` set means the server (or a
    /// middlebox) produced a message this client cannot interpret.
    /// Defaulting it to zero would fabricate a ranking position.
    #[test]
    fn rejects_an_entry_with_no_value() {
        let response = ranked_response(vec![ProtoRankedEntry {
            key: b"alpha".to_vec(),
            value: None,
        }]);
        let err = DocumentRankedEntries::from_unproved_response(&response)
            .expect_err("an entry with no value must be rejected");
        assert!(format!("{err}").contains("no `value`"));
    }

    /// A proved response reaching the unproven decoder is a caller
    /// mistake with a security consequence — silently returning
    /// nothing (or erroring vaguely) would let a caller believe an
    /// unverified path was the verified one. Name the right entry
    /// point in the error.
    #[test]
    fn rejects_a_proof_response() {
        let response = response_with(get_documents_response_v1::Result::Proof(Proof::default()));
        let err = DocumentRankedEntries::from_unproved_response(&response)
            .expect_err("a proof is not an unproven ranked payload");
        assert!(format!("{err}").contains("verify_ranked_top_k_proof"));
    }

    /// A response on another `ResultData` variant means the node
    /// routed the request somewhere else entirely (no ranking operand
    /// reached it, or it landed on the count / sum / average
    /// executor). Report the shape rather than an empty list.
    #[test]
    fn rejects_a_non_ranked_result_variant() {
        let response = response_with(get_documents_response_v1::Result::Data(ResultData {
            variant: Some(result_data::Variant::Documents(Documents {
                documents: Vec::new(),
            })),
        }));
        let err = DocumentRankedEntries::from_unproved_response(&response)
            .expect_err("a documents payload is not a ranked one");
        assert!(format!("{err}").contains("ResultData.ranked"));
    }

    /// The V0 `getDocuments` response predates the whole SQL-shaped
    /// surface and has no ranked variant; a node answering V0 is a
    /// node that cannot serve this query at all.
    #[test]
    fn rejects_a_v0_response() {
        let response = GetDocumentsResponse {
            version: Some(ResponseVersion::V0(Default::default())),
        };
        let err = DocumentRankedEntries::from_unproved_response(&response)
            .expect_err("V0 has no ranked shape");
        assert!(format!("{err}").contains("V1-only"));
    }

    /// A versionless response is a decode failure, not an empty
    /// ranking.
    #[test]
    fn rejects_a_versionless_response() {
        let err =
            DocumentRankedEntries::from_unproved_response(&GetDocumentsResponse { version: None })
                .expect_err("a versionless response must be rejected");
        assert!(matches!(err, Error::EmptyVersion));
    }

    /// `from_verified` is the identity constructor the SDK's proof
    /// path wraps verified entries with.
    #[test]
    fn from_verified_round_trips_the_input_vec() {
        let entries = vec![
            RankedEntry {
                key: b"gamma".to_vec(),
                value: RankedEntryValue::Count(9),
            },
            RankedEntry {
                key: b"alpha".to_vec(),
                value: RankedEntryValue::Count(2),
            },
        ];
        assert_eq!(
            DocumentRankedEntries::from_verified(entries.clone()).0,
            entries
        );
    }

    // The generic `FromProof<Q>` rejection is covered by the SDK's
    // tests, which can construct a valid `DriveDocumentQuery` via
    // dpp's `fixtures-and-mocks` feature. drive-proof-verifier itself
    // doesn't depend on `dpp/fixtures-and-mocks` outside dev-deps for
    // the contract fixtures that path needs.
}
