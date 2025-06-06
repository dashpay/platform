use crate::error::MapGroveDbError;
use crate::types::token_contract_info::TokenContractInfoResult;
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_token_contract_info_request, get_token_contract_info_response, GetTokenContractInfoRequest,
    GetTokenContractInfoResponse, Proof, ResponseMetadata,
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

        // Parse response to read proof and metadata
        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let token_id = match request.version.ok_or(Error::EmptyVersion)? {
            get_token_contract_info_request::Version::V0(v0) => {
                v0.token_id.try_into().map_err(|_| Error::RequestError {
                    error: "token_id must be exactly 32 bytes".to_string(),
                })?
            }
        };

        // Extract content from proof and verify Drive/GroveDB proofs
        let (root_hash, maybe_token_contract_info) = Drive::verify_token_contract_info(
            &proof.grovedb_proof,
            token_id,
            false,
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider)?;

        Ok((maybe_token_contract_info, mtd.clone(), proof.clone()))
    }
}
