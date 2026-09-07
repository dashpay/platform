//! Verified **chained document** (provable semi-join) results.
//!
//! A chained query is `SELECT * FROM <outer> WHERE $id IN (SELECT
//! <join_property> FROM <inner> WHERE …)` answered as ONE merged
//! grovedb proof: the limited inner indexOnly page and the outer
//! by-ids fetch derived from its values, merged by the server (grovedb
//! lifts the inner limit into a per-instance branch limit). The
//! verifier ([`DriveChainedDocumentQuery::verify_chained_documents_proof`])
//! reconstructs the merged query from the response's UNTRUSTED
//! join-value hint, verifies in one pass, and requires the proven
//! outer documents to match the PROVEN inner join values exactly — a
//! missing referenced document is an invalid proof (`refersTo:
//! permanentDocument` targets cannot dangle) — and this module's
//! [`FromProof`] impl composes that with the tenderdash signature
//! binding of the single root.
//!
//! There is deliberately **no unproven decoder with verification
//! semantics** here: an unproven chained response is free to fabricate
//! the join entirely (substitute, omit, inject), which is precisely
//! what the surface exists to prevent. [`ChainedDocuments`] can still
//! be built from a trusted node's unproven wire by the SDK if it
//! chooses, but the canonical path proves.

use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use drive::query::drive_chained_document_query::DriveChainedDocumentQuery;
use drive::verify::RootHash;

/// The verified result of a chained document query, both halves in
/// inner-proof order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChainedDocuments {
    /// The inner projections (synthesized indexOnly documents), exactly
    /// as the inner query alone would return them. The last one's join
    /// property carries the pagination cursor.
    pub inner_documents: Vec<Document>,
    /// The joined outer documents, ordered by first appearance of their
    /// id among the inner projections (deduplicated).
    pub outer_documents: Vec<Document>,
}

/// Verify a chained query's single merged proof and bind its root hash
/// to the quorum signature.
///
/// The merk-level composition (bootstrap subset pass on the inner
/// query, merged-query re-derivation, authoritative full verification,
/// exact set equality against the PROVEN join values) lives in rs-drive's
/// [`DriveChainedDocumentQuery::verify_chained_documents_proof`]; this
/// wrapper adds the [`verify_tenderdash_proof`] binding — the root hash
/// the proof commits to is only an attested fact once it is tied to the
/// quorum-signed app hash, and this function exists so the composition
/// can never be skipped by accident.
///
pub fn verify_chained_documents_proof(
    query: &DriveChainedDocumentQuery,
    proof: &Proof,
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, ChainedDocuments), Error> {
    let (root_hash, result) = query
        .verify_chained_documents_proof(&proof.grovedb_proof, platform_version)
        .map_drive_error(proof, mtd)?;

    verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

    Ok((
        root_hash,
        ChainedDocuments {
            inner_documents: result.inner_documents,
            outer_documents: result.outer_documents,
        },
    ))
}

impl<'dq, Q> FromProof<Q> for ChainedDocuments
where
    Q: TryInto<DriveChainedDocumentQuery<'dq>> + Clone + 'dq,
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

        let query: DriveChainedDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        // The standard envelope carries the single MERGED proof, and
        // the proof alone is enough: the verifier bootstraps the join
        // values from it via a subset pass.
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (_root_hash, chained) =
            verify_chained_documents_proof(&query, proof, mtd, platform_version, provider)?;

        // An empty inner page is a valid, proven "you have nothing
        // here" — surface it as Some(empty) rather than None so callers
        // can tell it apart from a missing object.
        Ok((Some(chained), mtd.clone(), proof.clone()))
    }
}
