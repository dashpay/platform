//! Having-range proof dispatch used by [`DocumentHavingEntries`].
//!
//! Having-side analog of [`super::ranked_proof_helpers`]: it turns a
//! caller-built [`DocumentQuery`] plus the node's response into a
//! verified entry list. The routing decisions — which axis, which
//! inclusive bounds the operator translates to, which direction, which
//! index covers them, which prefix-value segments a pinned compound
//! request descends through — are **not** re-derived here. They come
//! from rs-drive's own [`detect_having_mode`] and
//! [`resolve_having_query_for_mode`], the same two functions the server
//! calls, so client and server land on the same grove path and the same
//! bounds by construction rather than by two copies of a grammar
//! agreeing. The bounds matter doubly here: the verifier rebuilds the
//! proof's Merk query from them, so a divergence is a failed
//! verification, not a subtly different answer.
//!
//! [`DocumentHavingEntries`]: drive_proof_verifier::DocumentHavingEntries

use crate::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::DocumentTypeV0Getters,
};
use drive::query::drive_document_having_query::mode_detection::detect_having_mode;
use drive::query::drive_document_having_query::resolve_having_query_for_mode;
use drive::query::{DocumentHavingMode, RankedEntry, RankedPaginationInputs};
use drive_proof_verifier::verify_having_range_proof;

/// Validate that the caller-built [`DocumentQuery`] really describes a
/// having-range query, and resolve it into the `(bounds, descending,
/// limit, group property, aggregate field)` tuple the index picker and
/// the prover both work from.
///
/// Same return-the-mode design as
/// [`assert_ranked_shape`](super::ranked_proof_helpers::assert_ranked_shape),
/// for the same reason: the grammar check *is* the first step of
/// resolution. The grammar lives in rs-drive ([`detect_having_mode`])
/// and is versioned through
/// `platform_version.drive.methods.document.query.detect_having_mode`,
/// so the SDK cannot resolve a clause to different bounds than the
/// prover used.
pub(super) fn assert_having_shape(
    request: &DocumentQuery,
    platform_version: &PlatformVersion,
) -> Result<DocumentHavingMode, drive_proof_verifier::Error> {
    // Same sentinel handling as the ranked helper: `limit == 0` is
    // `DocumentQuery`'s "unset", reported to rs-drive as `None`.
    let pagination = RankedPaginationInputs {
        limit: (request.limit != 0).then_some(request.limit),
        offset: request.offset,
        has_start_at: request.start.is_some(),
    };

    detect_having_mode(
        &request.select,
        &request.group_by,
        &request.having,
        &request.order_by_clauses,
        &request.where_clauses,
        pagination,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!(
            "this DocumentQuery is not a well-formed having-range query: {e}. A having-range \
             query is `.with_select(<COUNT(*)|SUM(f)|AVG(f)>)`, `.with_group_by(<property>)`, \
             `.with_having(<one clause bounding the selected aggregate with a range \
             operator>)` and `.with_limit(n)`, optionally \
             `.order_by_selected_aggregate(<direction>)`, with no offset and no start_at; \
             where clauses, when present, pin the covering compound index's leading \
             properties — one equality pin per property, of which at most one may instead \
             be an `IN` of 2..=10 elements (merged entries then carry `in_key`; a null pin \
             on another property is rejected with `IN`)."
        ),
    })
}

/// Verify a having-range proof and return the verified entries — the
/// matching groups **in axis order in the walk direction**.
///
/// Single source of truth for the having proof path, mirroring
/// [`verify_ranked_query`](super::ranked_proof_helpers::verify_ranked_query)
/// step for step: re-run rs-drive's versioned request validation
/// (which resolves the bounds), resolve the covering index off the
/// contract, rebuild the query, verify. The root-hash binding to the
/// quorum-signed app hash happens inside [`verify_having_range_proof`]
/// and cannot be skipped through this helper.
pub(super) fn verify_having_query(
    request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<RankedEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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

    let mode = assert_having_shape(&request, platform_version)?;

    // Resolve the query exactly as the prover did — rs-drive's own
    // resolution (covering-index pick, shared with the ranked surface
    // because both read the same indexed tree, plus the equality-pin
    // encoding into prefix path segments for pinned compound requests).
    let having_query = resolve_having_query_for_mode(
        request.data_contract.id().to_buffer(),
        document_type,
        request.document_type_name.clone(),
        document_type.indexes(),
        &mode,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!(
            "document type `{}` cannot serve this having-range query: {e}. Ranked indexes \
             are opt-in contract grammar (meta-schema v3, protocol version 14+); a pinned \
             (compound-index) bound additionally needs every leading index property pinned \
             by a where clause — equality pins, of which at most one may be an `IN` of \
             2..=10 elements.",
            request.document_type_name,
        ),
    })?;

    // Binds the reconstructed grovedb root hash to the quorum-signed
    // app hash before returning — see the module docs.
    let (root_hash, entries) =
        verify_having_range_proof(&having_query, proof, mtd, platform_version, provider)?;

    tracing::trace!(
        target: "dash_sdk::having_query",
        root_hash = hex::encode(root_hash),
        height = mtd.height,
        entries = entries.len(),
        "verified having range proof"
    );

    Ok((Some(entries), mtd.clone(), proof.clone()))
}
