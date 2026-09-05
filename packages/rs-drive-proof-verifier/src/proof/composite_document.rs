//! Verified **composite document** results: a page plus the
//! sub-queries derived from it, answered as ONE merged grovedb proof.
//!
//! The server proves the limited page and every sub-query (by-id
//! joins, indexed lookups, grouped counts, siblings) as one merged
//! path query. The verifier
//! ([`DriveCompositeDocumentQuery::verify_composite_documents_proof`])
//! bootstraps the page from the proof with a subset pass, re-derives
//! every sub-query's `IN` clause from the PROVEN page (or the proven
//! earlier sub-query it binds), rebuilds the same merged query, verifies
//! it in one authoritative pass, and routes the proved entries back to
//! their components — refusing unclaimed entries, dangling joins and
//! derivation divergence. This module's [`FromProof`] impl composes
//! that with the tenderdash signature binding of the single root.
//!
//! There is deliberately **no unproven decoder with verification
//! semantics** here: an unproven composite response is free to
//! fabricate any sub-result, which is precisely what the surface exists
//! to prevent. [`CompositeDocuments`] can still be built from a trusted
//! node's unproven wire by the SDK if it chooses, but the canonical
//! path proves.

use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use drive::query::drive_composite_document_query::{DriveCompositeDocumentQuery, SubQueryResult};
use drive::verify::RootHash;

/// The verified result of a composite document query.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompositeDocuments {
    /// The page, exactly as the page query alone would return it.
    pub page_documents: Vec<Document>,
    /// One result per sub-query, in request order: a by-id join's
    /// documents in first-appearance order of their ids among the
    /// source documents, a lookup's or sibling's in query order, or one
    /// count per derived value that has a count tree (a value without
    /// an entry counts zero).
    pub sub_results: Vec<SubQueryResult>,
}

/// Verify a composite query's single merged proof and bind its root
/// hash to the quorum signature.
///
/// The merk-level composition (bootstrap subset pass on the page,
/// re-derivation of every sub-query, authoritative full verification,
/// routing with the completeness checks) lives in rs-drive's
/// [`DriveCompositeDocumentQuery::verify_composite_documents_proof`];
/// this wrapper adds the [`verify_tenderdash_proof`] binding — the root
/// hash the proof commits to is only an attested fact once it is tied
/// to the quorum-signed app hash, and this function exists so the
/// composition can never be skipped by accident.
pub fn verify_composite_documents_proof(
    query: &DriveCompositeDocumentQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, CompositeDocuments), Error> {
    let (root_hash, result) = query
        .verify_composite_documents_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((
        root_hash,
        CompositeDocuments {
            page_documents: result.page_documents,
            sub_results: result.sub_results,
        },
    ))
}

impl<'dq, Q> FromProof<Q> for CompositeDocuments
where
    Q: TryInto<DriveCompositeDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let query: DriveCompositeDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        // The standard envelope carries the single MERGED proof, and
        // the proof alone is enough: the verifier bootstraps the page
        // from it via a subset pass and re-derives the rest.
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (_root_hash, composite) =
            verify_composite_documents_proof(&query, proof, mtd, platform_version, provider)?;

        // An empty page is a valid, proven "nothing here" — surface it
        // as Some(empty) rather than None so callers can tell it apart
        // from a missing object.
        Ok((Some(composite), mtd.clone(), proof.clone()))
    }
}
