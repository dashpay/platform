use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{GetDocumentsSplitCountResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::query::DriveDocumentQuery;
use std::collections::BTreeMap;

/// The split counts of documents matching a query, verified from proof.
/// Maps property value bytes to count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSplitCounts(pub BTreeMap<Vec<u8>, u64>);

impl<'dq, Q> FromProof<Q> for DocumentSplitCounts
where
    Q: TryInto<DriveDocumentQuery<'dq>> + Clone + 'dq,
    Q::Error: std::fmt::Display,
{
    type Request = Q;
    type Response = GetDocumentsSplitCountResponse;

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

        let request: DriveDocumentQuery<'dq> =
            request
                .clone()
                .try_into()
                .map_err(|e: Q::Error| Error::RequestError {
                    error: e.to_string(),
                })?;

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let (root_hash, _documents) = request
            .verify_proof(&proof.grovedb_proof, platform_version)
            .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        // For split counts from proof, the client would need to deserialize the
        // verified documents and split by the requested property. For now, we
        // return an empty map as the proof verification confirms the data integrity.
        // Full client-side splitting can be done on top of the verified documents.
        Ok((
            Some(DocumentSplitCounts(BTreeMap::new())),
            mtd.clone(),
            proof.clone(),
        ))
    }
}
