use crate::error::MapGroveDbError;
use crate::types::data_contracts_latest_versions::{
    DataContractLatestVersion, DataContractsLatestVersions,
};
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_data_contracts_latest_versions_request, GetDataContractsLatestVersionsRequest,
    GetDataContractsLatestVersionsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetDataContractsLatestVersionsRequest> for DataContractsLatestVersions {
    type Request = GetDataContractsLatestVersionsRequest;
    type Response = GetDataContractsLatestVersionsResponse;

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

        let proof = response.proof().or(Err(Error::NoProofInResult))?;
        let mtd = response.metadata().or(Err(Error::EmptyResponseMetadata))?;

        let get_data_contracts_latest_versions_request::Version::V0(v0) =
            request.version.ok_or(Error::EmptyVersion)?;
        let include_contracts = v0.include_contracts;
        let ids = v0
            .ids
            .into_iter()
            .map(|id| {
                let len = id.len();
                id.try_into().map_err(|_| Error::RequestError {
                    error: format!("wrong id size: expected: 32, got: {len}"),
                })
            })
            .collect::<Result<Vec<[u8; 32]>, Error>>()?;

        // The proof is the multi-contract proof, which carries the contracts: each version is
        // read off the verified contract, and the contract is kept only when it was asked for.
        let (root_hash, contracts) = Drive::verify_contracts(
            supported_grovedb_proof_bytes(proof, platform_version)?,
            false,
            ids.as_slice(),
            platform_version,
        )
        .map_drive_error(proof, mtd)?;

        verify_tenderdash_proof(proof, mtd, &root_hash, provider, platform_version)?;

        let versions = contracts
            .into_iter()
            .map(|(id, maybe_contract)| {
                let id = Identifier::from_bytes(&id).map_err(|e| Error::ResultEncodingError {
                    error: e.to_string(),
                })?;
                let entry = maybe_contract.map(|contract| DataContractLatestVersion {
                    version: contract.version(),
                    data_contract: include_contracts.then_some(contract),
                });
                Ok((id, entry))
            })
            .collect::<Result<DataContractsLatestVersions, Error>>()?;

        // Every requested id has an entry, so the result is never absent.
        Ok((Some(versions), mtd.clone(), proof.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_data_contracts_latest_versions_request::{
        GetDataContractsLatestVersionsRequestV0, Version as ReqVersion,
    };
    use dapi_grpc::platform::v0::get_data_contracts_latest_versions_response::{
        get_data_contracts_latest_versions_response_v0::Result as RespResult,
        GetDataContractsLatestVersionsResponseV0, Version as RespVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use std::sync::Arc;

    /// Context provider that panics if called: every test here fails on the request before
    /// proof verification, so the provider must stay unreachable.
    struct UnreachableProvider;

    impl ContextProvider for UnreachableProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _pv: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            panic!("context provider should not be called")
        }

        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            panic!("context provider should not be called")
        }

        fn get_quorum_public_key(
            &self,
            _qt: u32,
            _qh: [u8; 32],
            _h: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            panic!("context provider should not be called")
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            panic!("context provider should not be called")
        }
    }

    fn response_with_proof_and_metadata() -> GetDataContractsLatestVersionsResponse {
        GetDataContractsLatestVersionsResponse {
            version: Some(RespVersion::V0(GetDataContractsLatestVersionsResponseV0 {
                result: Some(RespResult::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    fn request(ids: Vec<Vec<u8>>) -> GetDataContractsLatestVersionsRequest {
        GetDataContractsLatestVersionsRequest {
            version: Some(ReqVersion::V0(GetDataContractsLatestVersionsRequestV0 {
                ids,
                include_contracts: false,
                prove: true,
            })),
        }
    }

    fn from_proof_error(
        request: GetDataContractsLatestVersionsRequest,
        response: GetDataContractsLatestVersionsResponse,
    ) -> Error {
        <DataContractsLatestVersions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            PlatformVersion::latest(),
            &UnreachableProvider,
        )
        .unwrap_err()
    }

    #[test]
    fn should_fail_with_empty_version_when_request_has_no_version() {
        let err = from_proof_error(
            GetDataContractsLatestVersionsRequest { version: None },
            response_with_proof_and_metadata(),
        );
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn should_reject_id_that_is_not_32_bytes() {
        let err = from_proof_error(
            request(vec![vec![1u8; 32], vec![0u8; 5]]),
            response_with_proof_and_metadata(),
        );
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn should_fail_without_proof_when_response_is_empty() {
        let err = from_proof_error(
            request(vec![vec![0u8; 32]]),
            GetDataContractsLatestVersionsResponse::default(),
        );
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }
}
