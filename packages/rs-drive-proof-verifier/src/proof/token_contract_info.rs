use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_token_contract_info_request, GetTokenContractInfoRequest, GetTokenContractInfoResponse,
    Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetTokenContractInfoRequest> for TokenContractInfo {
    type Request = GetTokenContractInfoRequest;
    type Response = GetTokenContractInfoResponse;

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

        let token_id = match request.version.ok_or(Error::EmptyVersion)? {
            get_token_contract_info_request::Version::V0(v0) => {
                <[u8; 32]>::try_from(v0.token_id).map_err(|_| Error::RequestError {
                    error: "can't convert token_id to [u8; 32]".to_string(),
                })?
            }
        };

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result) = Drive::verify_token_contract_info(
            &proof.grovedb_proof,
            token_id,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((result, metadata, proof))
    }
}