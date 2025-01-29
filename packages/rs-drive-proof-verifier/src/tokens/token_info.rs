use crate::types::RetrievedOptionalObjects;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_identities_token_infos_request, get_identity_token_infos_request,
    GetIdentitiesTokenInfosRequest, GetIdentitiesTokenInfosResponse, GetIdentityTokenInfosRequest,
    GetIdentityTokenInfosResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use derive_more::From;
use dpp::dashcore::Network;
use dpp::identifier::Identifier;
use dpp::tokens::info::IdentityTokenInfo;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// Identity tokens information
#[derive(Clone, Debug, Default)]
pub struct IdentityTokenInfos(
    /// Token ID to token info
    pub RetrievedOptionalObjects<Identifier, IdentityTokenInfo>,
);

impl FromIterator<(Identifier, Option<IdentityTokenInfo>)> for IdentityTokenInfos {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<IdentityTokenInfo>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedOptionalObjects<Identifier, IdentityTokenInfo>>()
            .into()
    }
}

impl From<RetrievedOptionalObjects<Identifier, IdentityTokenInfo>> for IdentityTokenInfos {
    fn from(retrieved_objects: RetrievedOptionalObjects<Identifier, IdentityTokenInfo>) -> Self {
        Self(retrieved_objects)
    }
}

impl FromProof<GetIdentityTokenInfosRequest> for IdentityTokenInfos {
    type Request = GetIdentityTokenInfosRequest;
    type Response = GetIdentityTokenInfosResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let (token_ids, identity_id) = match request.version.ok_or(Error::EmptyVersion)? {
            get_identity_token_infos_request::Version::V0(v0) => {
                let identity_id =
                    <[u8; 32]>::try_from(v0.identity_id).map_err(|_| Error::RequestError {
                        error: "can't convert identity_id to [u8; 32]".to_string(),
                    })?;

                let token_ids = v0
                    .token_ids
                    .into_iter()
                    .map(<[u8; 32]>::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| Error::RequestError {
                        error: "can't convert token_id to [u8; 32]".to_string(),
                    })?;

                (token_ids, identity_id)
            }
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_token_infos_for_identity_id(
            &proof.grovedb_proof,
            &token_ids,
            identity_id,
            false,
            platform_version,
        )
        .map_err(|e| match e {
            drive::error::Error::GroveDB(e) => Error::GroveDBError {
                proof_bytes: proof.grovedb_proof.clone(),
                height: metadata.height,
                time_ms: metadata.time_ms,
                error: e.to_string(),
            },
            _ => e.into(),
        })?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}

/// Identity tokens information
#[derive(Debug, Default)]
pub struct IdentitiesTokenInfos(
    /// Identity ID to token info
    pub RetrievedOptionalObjects<Identifier, IdentityTokenInfo>,
);

impl FromIterator<(Identifier, Option<IdentityTokenInfo>)> for IdentitiesTokenInfos {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<IdentityTokenInfo>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedOptionalObjects<Identifier, IdentityTokenInfo>>()
            .into()
    }
}

impl From<RetrievedOptionalObjects<Identifier, IdentityTokenInfo>> for IdentitiesTokenInfos {
    fn from(retrieved_objects: RetrievedOptionalObjects<Identifier, IdentityTokenInfo>) -> Self {
        Self(retrieved_objects)
    }
}

impl FromProof<GetIdentitiesTokenInfosRequest> for IdentitiesTokenInfos {
    type Request = GetIdentitiesTokenInfosRequest;
    type Response = GetIdentitiesTokenInfosResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let response: Self::Response = response.into();

        let (token_id, identity_ids) = match request.version.ok_or(Error::EmptyVersion)? {
            get_identities_token_infos_request::Version::V0(v0) => {
                let token_id =
                    <[u8; 32]>::try_from(v0.token_id.clone()).map_err(|_| Error::RequestError {
                        error: "can't convert token_id to [u8; 32]".to_string(),
                    })?;

                let identity_ids = v0
                    .identity_ids
                    .into_iter()
                    .map(<[u8; 32]>::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| Error::RequestError {
                        error: "can't convert identity_id to [u8; 32]".to_string(),
                    })?;

                (token_id, identity_ids)
            }
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_token_infos_for_identity_ids(
            &proof.grovedb_proof,
            token_id,
            &identity_ids,
            false,
            platform_version,
        )
        .map_err(|e| match e {
            drive::error::Error::GroveDB(e) => Error::GroveDBError {
                proof_bytes: proof.grovedb_proof.clone(),
                height: metadata.height,
                time_ms: metadata.time_ms,
                error: e.to_string(),
            },
            _ => e.into(),
        })?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}
