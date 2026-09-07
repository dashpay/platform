//! Verified **ranked** (`GROUP BY … ORDER BY <aggregate> LIMIT n`)
//! document results.
//!
//! A ranked query answers "which `n` groups score highest (or lowest)
//! on an aggregate?" — `SELECT AVG(grade) GROUP BY restaurantId
//! ORDER BY grade DESC LIMIT 5`. The answer is read straight out of
//! the per-axis *secondary* Merk of an indexed tree (grovedb PR #657),
//! so it costs `O(log n + k)` and comes with a proof that commits to
//! exactly the `k` returned `(aggregate, group key)` pairs — plus the
//! `OFFSET`, which grovedb counts from the subtree aggregates rather
//! than by walking the skipped region — and additionally attests, on
//! this proved path — so a deep page costs `O(log n + k)` like any
//! other rather than growing with the offset.
//!
//! This module holds the client-facing result type
//! ([`DocumentRankedEntries`]), the tenderdash-composition wrapper
//! around rs-drive's merk-level verifier
//! ([`verify_ranked_top_k_proof`]), and the decoder for the unproven
//! `ResultData.ranked` wire payload
//! ([`DocumentRankedEntries::from_unproved_response`]).
//!
//! Per-shape routing (which index covers the axis, which
//! `(axis, descending, k, offset)` tuple the request resolves to) lives
//! in rs-sdk's `ranked_proof_helpers`, exactly as count's four-way
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
use drive::query::{
    DriveDocumentQuery, DriveDocumentRankedQuery, RankedEntry, RankedEntryValue, RankedPage,
    RANKED_AVG_SCALE,
};
use drive::verify::RootHash;

/// One page of a `GROUP BY … ORDER BY <aggregate> LIMIT n [OFFSET m]`
/// query: the ranked groups, plus the rank the page starts at.
///
/// **Entry order is the ranking order** — best-first for `DESC`,
/// worst-first for `ASC`. Callers must not re-sort; ties (groups with
/// equal aggregates) come back in group-key order *in the direction of
/// the walk*, which is descending group-key order for `DESC`.
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
///
/// The fixed point is **exact on the proved path only**. A page built
/// by [`Self::from_verified`] carries the very integer the proof
/// commits to; one built by [`Self::from_unproved_response`] carries a
/// best-effort reconstruction from the wire's `double` — see that
/// method for what that costs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentRankedEntries {
    /// The 0-based rank of `entries[0]` — the query's `OFFSET`, as
    /// actually honoured.
    ///
    /// This is what turns a page back into a *ranking*: entry `i` is
    /// the group at rank `starting_rank + i`. Without it a caller who
    /// asked for `ORDER BY avg(grade) DESC LIMIT 1 OFFSET 4` receives
    /// one entry and has no way to tell it really is the 5th-best
    /// group rather than the best.
    ///
    /// On the **proved** path this is grovedb's cryptographically
    /// attested count, re-derived by the verifier from the counted
    /// subtree commitments in the proof bytes rather than trusted from
    /// the response. It equals the requested offset unless the walk ran
    /// out of groups first, in which case `entries` is empty and this
    /// is a *proof* that the ranking holds exactly this many groups in
    /// total — an offset past the end is a positive answer, not an
    /// error.
    ///
    /// On the **unproven** decode it is whatever the node put on the
    /// wire (`0` when the field is absent), and carries no more weight
    /// than the entries beside it.
    pub starting_rank: u64,
    /// The groups on this page, **in ranking order**.
    pub entries: Vec<RankedEntry>,
}

impl DocumentRankedEntries {
    /// Build a [`DocumentRankedEntries`] from a verifier-side
    /// [`RankedPage`] — the shape rs-drive's merk-level verifier
    /// returns, carrying the attested skip alongside the entries.
    ///
    /// Mirrors
    /// [`DocumentSplitCounts::from_verified`](crate::DocumentSplitCounts::from_verified),
    /// except that it is not the identity: `RankedPage` is rs-drive's
    /// internal type and this is the client-facing one, so the rename
    /// of `skipped` → `starting_rank` happens here, where the value
    /// stops being "how far the walk skipped" and starts being "which
    /// rank you are looking at".
    pub fn from_verified(page: RankedPage) -> Self {
        DocumentRankedEntries {
            starting_rank: page.skipped,
            entries: page.entries,
        }
    }

    /// Decode the **unproven** ranked payload of a `getDocuments`
    /// response — the `ResultData.ranked` variant a node returns for a
    /// ranked request sent with `prove = false`.
    ///
    /// Order is preserved verbatim: the server emits entries in
    /// ranking order and this decoder never re-sorts.
    ///
    /// This is a plain wire decode with **no cryptographic guarantee
    /// whatsoever** — it is the "trust the node" path, and that applies
    /// to [`Self::starting_rank`] every bit as much as to the entries:
    /// an unproven page claiming to start at rank 4 is a claim, not a
    /// fact. Prefer [`verify_ranked_top_k_proof`] (via rs-sdk's
    /// `DocumentRankedEntries::fetch`) unless you are deliberately
    /// reading from a node you already trust.
    ///
    /// A node that predates the wire `skipped` field leaves it unset;
    /// that decodes to `starting_rank == 0`, which is the right answer
    /// for the offset-less queries such a node could serve at all.
    ///
    /// ## Averages come back approximate here
    ///
    /// The wire's `avg` is a `double`, deliberately: these entries only
    /// exist on this path, and a proof-verifying client reconstructs
    /// the exact fixed point from the proof instead. To keep one
    /// [`RankedEntryValue`] type across both paths this decoder
    /// multiplies the double back up by [`crate::RANKED_AVG_SCALE`] and
    /// rounds, so the [`RankedEntryValue::AvgFixedPoint`] it yields is a
    /// **best-effort reconstruction, not the committed integer** — its
    /// low digits are noise beyond `f64`'s ~15–16 significant decimal
    /// digits. Render it, compare it loosely, but do not treat it as the
    /// value grovedb ranked on; ask for the proof if you need that.
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyVersion`] when the response carries no version.
    /// - [`Error::ResponseDecodeError`] when the response is a V0
    ///   response (which has no ranked shape), carries a proof rather
    ///   than data, or carries a non-ranked `ResultData` variant.
    /// - [`Error::ResponseDecodeError`] when an entry's `value` oneof
    ///   is unset, or its `avg` is not a finite double that scales into
    ///   `i128` range.
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
        let (starting_rank, entries) = match v1.result.as_ref() {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Ranked(ranked)),
            })) => (
                ranked.skipped.unwrap_or(0),
                ranked
                    .entries
                    .iter()
                    .map(ranked_entry_from_proto)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
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
                         {}. A response on another variant means the node routed the \
                         request to a different executor — check that the request carries a \
                         `group_by` and a single `order_by` naming the single `select`'s \
                         aggregate (`$count` for `COUNT(*)`).",
                        result_variant_name(other)
                    ),
                });
            }
        };
        Ok((
            DocumentRankedEntries {
                starting_rank,
                entries,
            },
            metadata,
        ))
    }
}

/// The received-but-unexpected shape of a `getDocuments` V1 result, by
/// **name only** — never the payload. Interpolating the payload into an
/// error would make the message (and any log line carrying it) grow
/// with an untrusted response, and could copy returned document bytes
/// into logs. Shared by the ranked and having-range decoders.
pub(crate) fn result_variant_name(
    result: Option<&get_documents_response_v1::Result>,
) -> &'static str {
    match result {
        None => "an absent result",
        Some(get_documents_response_v1::Result::Proof(_)) => "a proof",
        Some(get_documents_response_v1::Result::Data(ResultData { variant })) => match variant {
            None => "a ResultData with no variant",
            Some(result_data::Variant::Documents(_)) => "a ResultData.documents payload",
            Some(result_data::Variant::Counts(_)) => "a ResultData.counts payload",
            Some(result_data::Variant::Sums(_)) => "a ResultData.sums payload",
            Some(result_data::Variant::Averages(_)) => "a ResultData.averages payload",
            Some(result_data::Variant::Ranked(_)) => "a ResultData.ranked payload",
            Some(result_data::Variant::Chained(_)) => "a ResultData.chained payload",
        },
    }
}

/// Decode one wire [`ProtoRankedEntry`] into rs-drive's
/// [`RankedEntry`].
///
/// The `avg` arm re-scales the wire's `double` into the fixed-point
/// `i128` [`RankedEntryValue`] carries, so callers see one type
/// regardless of which path produced the page. The round trip is lossy
/// in one direction only — the server divided an exact integer by
/// [`RANKED_AVG_SCALE`], we multiply back and round — so the result is
/// the closest fixed point to what the node reported, not necessarily
/// the one it committed to. `from_unproved_response` documents that;
/// exactness lives on the proof path.
///
/// A non-finite `avg`, or one that scales past `i128`, is rejected
/// rather than saturated: `as` casts would silently turn `NaN` into
/// `0` (an average of zero — a plausible-looking lie) and an
/// out-of-range double into `i128::MIN`/`MAX`. Every legitimate value
/// fits comfortably, since `|sum| ≤ i64::MAX` bounds the true fixed
/// point at `i64::MAX * 10^19 ≈ 9.2e37 < i128::MAX`.
pub(crate) fn ranked_entry_from_proto(entry: &ProtoRankedEntry) -> Result<RankedEntry, Error> {
    let value = match entry.value.as_ref() {
        Some(ranked_entry::Value::Count(count)) => RankedEntryValue::Count(*count),
        Some(ranked_entry::Value::Sum(sum)) => RankedEntryValue::Sum(*sum),
        Some(ranked_entry::Value::Avg(avg)) => {
            let scaled = avg * (RANKED_AVG_SCALE as f64);
            // `i128::MIN as f64` is exactly −2^127; `i128::MAX as f64`
            // rounds *up* to 2^127, hence the asymmetric comparisons.
            if !scaled.is_finite() || scaled < (i128::MIN as f64) || scaled >= -(i128::MIN as f64) {
                return Err(Error::ResponseDecodeError {
                    error: format!(
                        "`avg` must be a finite double that scales into i128 range when \
                         multiplied by {RANKED_AVG_SCALE}, got {avg}"
                    ),
                });
            }
            RankedEntryValue::AvgFixedPoint(scaled.round() as i128)
        }
        None => {
            return Err(Error::ResponseDecodeError {
                error: "ranked entry carries no `value`; the server always sets exactly one \
                        of `count` / `sum` / `avg`"
                    .to_string(),
            });
        }
    };
    Ok(RankedEntry {
        // Present exactly on `IN`-pinned responses; the wire's absent
        // state maps to the drive type's `None` untouched.
        in_key: entry.in_key.clone(),
        key: entry.key.clone(),
        value,
    })
}

/// Verify a grovedb indexed-axis top-k proof **and the surrounding
/// tenderdash commit**, returning the reconstructed root hash and the
/// [`RankedPage`] it commits to.
///
/// The page is returned whole rather than as a bare entry list because
/// [`RankedPage::skipped`] is verified evidence in its own right: it is
/// re-derived from the counted subtree commitments in the proof bytes,
/// so it pins each entry to an absolute rank, and on a page past the
/// end of the ranking it is the *only* payload — an attested total
/// population under an empty entry list.
///
/// Thin tenderdash-composition wrapper over
/// [`DriveDocumentRankedQuery::verify_ranked_top_k_proof`] in rs-drive
/// (which does the merk-level verification). Both sides derive the
/// proved subtree from the same
/// `DriveDocumentRankedQuery::indexed_property_name_tree_path`, so
/// prover and verifier cannot drift on *which* ranking is being
/// checked, and grovedb re-executes the proof against the
/// `(axis, k, offset, descending)` traversal rebuilt from the request —
/// a proof of one ranking does not cover another.
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
) -> Result<(RootHash, RankedPage), Error> {
    let (root_hash, page) = query
        .verify_ranked_top_k_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((root_hash, page))
}

/// Reject the generic [`FromProof`] entry point for
/// [`DocumentRankedEntries`].
///
/// `DocumentRankedEntries` is reached from rs-sdk via the
/// `FromProof<DocumentQuery>` impl defined alongside the SDK's
/// `DocumentQuery` type (see
/// `rs-sdk/src/platform/documents/document_ranked_entries.rs`), which
/// resolves the `(axis, descending, k, offset)` tuple and the covering
/// index from the request's `(select, group_by, order_by, limit,
/// offset)` shape plus the data contract. The generic
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
                 .with_select(<aggregate>), .with_group_by(<property>), \
                 .order_by_selected_aggregate(<direction>) and .with_limit(n), which \
                 resolves the ranking axis and the covering index from the data contract"
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
            in_key: None,
            key: key.as_bytes().to_vec(),
            value: Some(ranked_entry::Value::Count(count)),
        }
    }

    /// An entry as the *server* would emit it for a group whose exact
    /// fixed-point average is `fixed_point`: the wire carries
    /// `fixed_point as f64 / RANKED_AVG_SCALE as f64`, the same
    /// conversion `RankedEntryValue::as_f64` performs in rs-drive-abci.
    fn avg_entry(key: &str, fixed_point: i128) -> ProtoRankedEntry {
        avg_entry_raw(key, (fixed_point as f64) / (RANKED_AVG_SCALE as f64))
    }

    /// An entry carrying an arbitrary double, for the malformed cases
    /// that no fixed point maps to.
    fn avg_entry_raw(key: &str, avg: f64) -> ProtoRankedEntry {
        ProtoRankedEntry {
            in_key: None,
            key: key.as_bytes().to_vec(),
            value: Some(ranked_entry::Value::Avg(avg)),
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
        paged_ranked_response(entries, None)
    }

    fn paged_ranked_response(
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

    /// The headline decode: `SELECT AVG(grade) … GROUP BY restaurantId
    /// ORDER BY grade DESC LIMIT 3`. Entry order is the ranking order
    /// and must survive the decode verbatim, and each double must
    /// re-scale into the fixed point it was rendered from.
    ///
    /// These particular averages survive the double round trip *bit for
    /// bit* — `95`, `85` and `10.5` times `10^19` all fit in `f64`'s 53
    /// significand bits — so the assertion can be exact. That is a
    /// property of the fixtures, not a guarantee of the wire format:
    /// `from_unproved_response` reconstructs a best-effort fixed point,
    /// and an average with more significant digits than `f64` holds
    /// would come back slightly off. Exactness lives on the proof path.
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
        let keys: Vec<&[u8]> = decoded.entries.iter().map(|e| e.key.as_slice()).collect();
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
            decoded.entries[2].value,
            RankedEntryValue::AvgFixedPoint((21 * RANKED_AVG_SCALE).div_euclid(2))
        );
        assert_eq!(
            decoded.entries[2].value.as_f64(),
            10.5,
            "dividing by RANKED_AVG_SCALE recovers the average a caller renders"
        );
    }

    /// Averages are signed: a group whose summable property is
    /// negative ranks below zero, and the sign must survive the double
    /// round trip. A decoder that mishandled it would turn `-0.5` into
    /// a positive number and silently invert the ranking's meaning.
    ///
    /// The last entry is the largest magnitude the axis can actually
    /// produce — `sum = i64::MIN` over a single document — which pins
    /// that the `i128`-range guard rejects only genuinely impossible
    /// doubles, not legitimate extremes.
    #[test]
    fn decodes_negative_averages() {
        let negative_half = (-RANKED_AVG_SCALE).div_euclid(2);
        let extreme = (i64::MIN as i128) * RANKED_AVG_SCALE;
        let response = ranked_response(vec![
            avg_entry("above", RANKED_AVG_SCALE),
            avg_entry("below", negative_half),
            avg_entry("floor", extreme),
        ]);

        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&response)
            .expect("negative averages are well-formed");

        assert_eq!(
            decoded.entries[1].value,
            RankedEntryValue::AvgFixedPoint(negative_half)
        );
        assert_eq!(decoded.entries[1].value.as_f64(), -0.5);
        assert_eq!(
            decoded.entries[2].value,
            RankedEntryValue::AvgFixedPoint(extreme),
            "the most negative average the axis can hold is decoded, not rejected"
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
            decoded.entries.iter().map(|e| e.value).collect::<Vec<_>>(),
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
            in_key: None,
            key: b"refunds".to_vec(),
            value: Some(ranked_entry::Value::Sum(-1_000)),
        }]);

        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&response)
            .expect("signed sums are well-formed");

        assert_eq!(decoded.entries[0].value, RankedEntryValue::Sum(-1_000));
    }

    /// An empty ranking is a legitimate answer, not an error: the
    /// index simply has no groups yet.
    #[test]
    fn decodes_an_empty_ranking() {
        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&ranked_response(vec![]))
            .expect("an empty ranking is well-formed");
        assert!(decoded.entries.is_empty());
        assert_eq!(decoded.starting_rank, 0);
    }

    /// `skipped` is what makes a page a ranking rather than a list.
    /// A single entry at rank 4 is the *5th* best group, and the decode
    /// must carry that through — dropping it would leave the caller
    /// unable to distinguish it from the winner.
    #[test]
    fn decodes_the_starting_rank_of_a_page() {
        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&paged_ranked_response(
            vec![avg_entry("epsilon", 10 * RANKED_AVG_SCALE)],
            Some(4),
        ))
        .expect("a paged ranked payload decodes");
        assert_eq!(decoded.starting_rank, 4);
        assert_eq!(decoded.entries.len(), 1);
    }

    /// An offset past the end: no entries, but `skipped` still carries
    /// the population. Empty entries plus a positive rank is a
    /// meaningful answer ("there are only 12 groups"), not a
    /// contradiction to be normalized away.
    #[test]
    fn decodes_a_page_past_the_end_of_the_ranking() {
        let (decoded, _) =
            DocumentRankedEntries::from_unproved_response(&paged_ranked_response(vec![], Some(12)))
                .expect("a past-the-end page decodes");
        assert!(decoded.entries.is_empty());
        assert_eq!(decoded.starting_rank, 12);
    }

    /// A node that predates the wire field leaves `skipped` unset.
    /// That must read as rank 0 — the right answer for the offset-less
    /// queries such a node could serve at all — rather than as a decode
    /// failure.
    #[test]
    fn an_absent_skipped_field_decodes_as_rank_zero() {
        let (decoded, _) = DocumentRankedEntries::from_unproved_response(&paged_ranked_response(
            vec![count_entry("delta", 4)],
            None,
        ))
        .expect("an absent `skipped` is not a decode failure");
        assert_eq!(decoded.starting_rank, 0);
    }

    /// An `avg` that cannot be a scaled fixed point is rejected rather
    /// than cast. `as` casts on `f64 -> i128` saturate and map `NaN` to
    /// `0`, so a malformed wire value would otherwise decode as a
    /// confident average of zero (or of `i128::MIN`) — a
    /// plausible-looking lie is far worse than a loud failure.
    ///
    /// This is the double-shaped replacement for the old
    /// exactly-16-bytes length check: a `double` field has no length to
    /// validate, but it does have values no legitimate average maps to.
    #[test]
    fn rejects_an_avg_that_is_not_a_scaled_fixed_point() {
        for avg in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            // Past i128 range once multiplied by 10^19.
            1e30,
            -1e30,
        ] {
            let response = ranked_response(vec![avg_entry_raw("alpha", avg)]);
            let err = match DocumentRankedEntries::from_unproved_response(&response) {
                Err(err) => err,
                Ok(decoded) => panic!("an `avg` of {avg} must be rejected, decoded {decoded:?}"),
            };
            let message = format!("{err}");
            assert!(
                message.contains("finite double") && message.contains("i128 range"),
                "the rejection must name the contract it violates; got {message}"
            );
        }
    }

    /// A `RankedEntry` with no `value` set means the server (or a
    /// middlebox) produced a message this client cannot interpret.
    /// Defaulting it to zero would fabricate a ranking position.
    #[test]
    fn rejects_an_entry_with_no_value() {
        let response = ranked_response(vec![ProtoRankedEntry {
            in_key: None,
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

    /// `from_verified` is what the SDK's proof path wraps a verified
    /// [`RankedPage`] with. Both halves must survive: dropping
    /// `skipped` here would silently turn every proved page into a
    /// claim about rank 0.
    #[test]
    fn from_verified_carries_both_halves_of_the_page() {
        let entries = vec![
            RankedEntry {
                in_key: None,
                key: b"gamma".to_vec(),
                value: RankedEntryValue::Count(9),
            },
            RankedEntry {
                in_key: None,
                key: b"alpha".to_vec(),
                value: RankedEntryValue::Count(2),
            },
        ];
        let wrapped = DocumentRankedEntries::from_verified(RankedPage {
            skipped: 7,
            entries: entries.clone(),
        });
        assert_eq!(wrapped.entries, entries);
        assert_eq!(
            wrapped.starting_rank, 7,
            "the attested skip is the page's rank base and must not be dropped"
        );
    }

    // The generic `FromProof<Q>` rejection is covered by the SDK's
    // tests, which can construct a valid `DriveDocumentQuery` via
    // dpp's `fixtures-and-mocks` feature. drive-proof-verifier itself
    // doesn't depend on `dpp/fixtures-and-mocks` outside dev-deps for
    // the contract fixtures that path needs.
}
