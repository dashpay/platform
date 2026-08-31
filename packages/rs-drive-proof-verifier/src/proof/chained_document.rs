//! Verified **chained document** (provable semi-join) results.
//!
//! A chained query is `SELECT * FROM <outer> WHERE $id IN (SELECT
//! <join_property> FROM <inner> WHERE …)` answered as TWO grovedb
//! proofs bound to ONE state root: the inner indexOnly page, and the
//! outer by-ids fetch DERIVED from the proven inner values. The
//! verifier ([`DriveChainedDocumentQuery::verify_chained_documents_proof`])
//! re-derives the outer query itself, requires equal root hashes and
//! exact id↔document set equality — a missing referenced document is an
//! invalid proof (`refersTo: permanentDocument` targets cannot dangle)
//! — and this module's [`FromProof`] impl composes that with the
//! tenderdash signature binding of the shared root.
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
use dapi_grpc::platform::v0::get_chained_documents_response::Version as ResponseVersion;
use dapi_grpc::platform::v0::{GetChainedDocumentsResponse, Proof, ResponseMetadata};
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

/// Verify a chained proof pair and bind the shared root hash to the
/// quorum signature.
///
/// The merk-level composition (outer-query re-derivation, root
/// equality, exact set equality) lives in rs-drive's
/// [`DriveChainedDocumentQuery::verify_chained_documents_proof`]; this
/// wrapper adds the [`verify_tenderdash_proof`] binding — the root
/// hash both proofs commit to is only an attested fact once it is tied
/// to the quorum-signed app hash, and this function exists so the
/// composition can never be skipped by accident.
///
/// `outer_grovedb_proof` is the response's rider field: empty means
/// "no outer proof" (required for an empty inner page, refused
/// otherwise — the verifier enforces presence-iff-nonempty).
pub fn verify_chained_documents_proof(
    query: &DriveChainedDocumentQuery,
    proof: &Proof,
    outer_grovedb_proof: &[u8],
    mtd: &ResponseMetadata,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(RootHash, ChainedDocuments), Error> {
    let outer_proof = (!outer_grovedb_proof.is_empty()).then_some(outer_grovedb_proof);
    let (root_hash, result) = query
        .verify_chained_documents_proof(&proof.grovedb_proof, outer_proof, platform_version)
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
    type Response = GetChainedDocumentsResponse;

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

        // The standard envelope carries the INNER proof (and the
        // signature fields); the outer grovedb proof rides beside the
        // result oneof.
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;
        let outer_grovedb_proof = match &response.version {
            Some(ResponseVersion::V0(v0)) => v0.outer_grovedb_proof.as_slice(),
            None => return Err(Error::EmptyVersion),
        };

        let (_root_hash, chained) = verify_chained_documents_proof(
            &query,
            proof,
            outer_grovedb_proof,
            mtd,
            platform_version,
            provider,
        )?;

        // An empty inner page is a valid, proven "you have nothing
        // here" — surface it as Some(empty) rather than None so callers
        // can tell it apart from a missing object.
        Ok((Some(chained), mtd.clone(), proof.clone()))
    }
}
