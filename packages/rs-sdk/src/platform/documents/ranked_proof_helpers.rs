//! Ranked-proof dispatch used by [`DocumentRankedEntries`].
//!
//! Ranked-side analog of [`super::count_proof_helpers`]: it turns a
//! caller-built [`DocumentQuery`] plus the node's response into a
//! verified `Vec<RankedEntry>`. The routing decisions — which ranking
//! axis, which direction, how many groups, which index covers them —
//! are **not** re-derived here. They come from rs-drive's own
//! [`detect_ranked_mode`] and [`find_ranked_index_for_axis`], the same
//! two functions the server calls, so client and server land on the
//! same grove path and the same `(axis, k, descending)` triple by
//! construction rather than by two copies of a grammar agreeing.
//!
//! Unlike the count helper there is no per-shape dispatch: the ranked
//! surface has exactly one proof primitive
//! (`prove_indexed_axis_top_k`), and all of a request's variation is
//! carried *inside* the query struct.
//!
//! [`DocumentRankedEntries`]: drive_proof_verifier::DocumentRankedEntries

use crate::platform::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::DocumentTypeV0Getters,
};
use drive::query::drive_document_ranked_query::index_picker::find_ranked_index_for_axis;
use drive::query::drive_document_ranked_query::mode_detection::detect_ranked_mode;
use drive::query::{
    DocumentRankedMode, DriveDocumentRankedQuery, RankedEntry, RankedPaginationInputs,
};
use drive_proof_verifier::verify_ranked_top_k_proof;

/// Validate that the caller-built [`DocumentQuery`] really describes a
/// ranked query, and resolve it into the
/// `(axis, descending, k, group property, aggregate field)` tuple the
/// index picker and the prover both work from.
///
/// This is the ranked counterpart of
/// [`assert_select_is_count`](super::count_proof_helpers::assert_select_is_count),
/// with one deliberate difference: it *returns* the resolved mode
/// rather than returning `()`. The count assertion guards a shape the
/// verifier cannot reproduce and is therefore a separate pre-check;
/// here the very same call is the first step of resolution, and
/// running rs-drive's grammar twice (once to assert, once to resolve)
/// would be pure duplication with a chance of the two calls being
/// given different inputs.
///
/// The grammar itself lives in rs-drive
/// ([`detect_ranked_mode`]) and is versioned through
/// `platform_version.drive.methods.document.query.detect_ranked_mode`,
/// so an SDK built against one protocol version cannot quietly accept
/// a request shape the network of that version rejects — and, more
/// importantly, cannot resolve a request to a *different*
/// `(axis, descending, k)` triple than the prover used.
///
/// `order_by` is checked here rather than in rs-drive because
/// `DocumentRankedRequest` has no field to carry it: the server
/// rejects it in its routing layer for exactly that reason, and this
/// is the client-side mirror of that rejection.
pub(super) fn assert_ranked_shape(
    request: &DocumentQuery,
    platform_version: &PlatformVersion,
) -> Result<DocumentRankedMode, drive_proof_verifier::Error> {
    if !request.order_by_clauses.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: "ORDER BY is not valid for a ranked query: the entry order of a \
                    `HAVING … TOP(n)` / `BOTTOM(n)` result already is the ranking order \
                    (best-first for TOP, worst-first for BOTTOM). Drop `order_by`, or \
                    flip TOP ↔ BOTTOM to reverse the ranking."
                .to_string(),
        });
    }

    // `DocumentQuery` uses `0` as the "unset" sentinel for `limit`
    // (it becomes `None` on the wire) and has no `offset` field at
    // all — the encoder always emits `offset: None`. Both are
    // reported to rs-drive exactly as the server sees them so the
    // client rejects the same requests the server would, with the
    // same message.
    let pagination = RankedPaginationInputs {
        limit: (request.limit != 0).then_some(request.limit),
        offset: None,
        has_start_at: request.start.is_some(),
    };

    detect_ranked_mode(
        &request.select,
        &request.group_by,
        &request.having,
        &request.where_clauses,
        pagination,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!(
            "this DocumentQuery is not a well-formed ranked query: {e}. A ranked query is \
             `.with_select(<COUNT(*)|SUM(f)|AVG(f)>)`, `.with_group_by(<property>)` and \
             `.with_having(vec![<one clause whose right operand is a TOP(n) / BOTTOM(n) \
             ranking on the same aggregate>])`, with no where clauses, no limit, no \
             start_at and no order_by."
        ),
    })
}

/// Verify a ranked-shape proof and return the ranked entries, **in
/// ranking order**.
///
/// Single source of truth for the ranked proof path. The steps are
/// deliberately the same ones the drive-side suite runs in its
/// `client_side_query` helper — re-run rs-drive's versioned request
/// validation, resolve the covering index off the contract, rebuild
/// the query — because prover and verifier agreeing on the grove path
/// is what makes the proof mean anything at all.
///
/// ## Root-hash binding
///
/// The merk-level verifier returning `Ok` proves nothing on its own: a
/// bit-flip sweep over a real ranked envelope shows ~9% of mutations
/// verifying cleanly with the correct entries under a *different*
/// reconstructed root hash. The binding that rejects those is inside
/// [`verify_ranked_top_k_proof`], which checks the reconstructed root
/// against the quorum-signed app hash carried by the response's
/// metadata before returning. There is no path through this helper
/// that yields entries without that check having run.
pub(super) fn verify_ranked_query(
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

    let mode = assert_ranked_shape(&request, platform_version)?;

    // Pick the index the prover picked. Availability is decided from
    // the index's `ranked_*` flags, not from the stored element
    // variant, and this is rs-drive's own picker — a second
    // implementation here could choose a different index on a
    // contract with several candidates and then verify a proof of the
    // wrong subtree's ranking (or, more likely, fail to verify at all
    // with a confusing merk error).
    let index = find_ranked_index_for_axis(
        document_type.indexes(),
        &mode.group_by_property,
        mode.axis,
        &mode.aggregate_field,
    )
    .ok_or_else(|| drive_proof_verifier::Error::RequestError {
        error: format!(
            "no index on document type `{}` can rank by `{:?}` grouped on `{}`: a ranked \
             query needs a single-property index over `{}` declaring `{}` (and, for SUM / \
             AVG, `summable: \"{}\"`). Ranked indexes are opt-in contract grammar \
             (meta-schema v3, protocol version 14+).",
            request.document_type_name,
            mode.axis,
            mode.group_by_property,
            mode.group_by_property,
            mode.axis.required_index_keyword(),
            mode.aggregate_field,
        ),
    })?;

    let ranked_query = DriveDocumentRankedQuery {
        document_type,
        contract_id: request.data_contract.id().to_buffer(),
        document_type_name: request.document_type_name.clone(),
        index,
        axis: mode.axis,
        descending: mode.descending,
        k: mode.k,
    };

    // Binds the reconstructed grovedb root hash to the quorum-signed
    // app hash before returning — see the module docs.
    let (root_hash, entries) =
        verify_ranked_top_k_proof(&ranked_query, proof, mtd, platform_version, provider)?;

    tracing::trace!(
        target: "dash_sdk::ranked_query",
        root_hash = hex::encode(root_hash),
        height = mtd.height,
        entries = entries.len(),
        "verified ranked top-k proof"
    );

    Ok((Some(entries), mtd.clone(), proof.clone()))
}
