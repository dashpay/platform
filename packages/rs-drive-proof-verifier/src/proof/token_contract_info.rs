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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromProof;
    use dapi_grpc::platform::v0::{
        get_token_contract_info_request::{GetTokenContractInfoRequestV0, Version as ReqVersion},
        get_token_contract_info_response::{
            get_token_contract_info_response_v0::Result as RespResult,
            GetTokenContractInfoResponseV0, Version as RespVersion,
        },
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use std::sync::Arc;

    /// Context provider that panics if called — tests should stop before proof
    /// verification so the provider is unreachable.
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

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn response_with_proof_and_metadata() -> GetTokenContractInfoResponse {
        GetTokenContractInfoResponse {
            version: Some(RespVersion::V0(GetTokenContractInfoResponseV0 {
                result: Some(RespResult::Proof(Proof::default())),
                metadata: Some(ResponseMetadata::default()),
            })),
        }
    }

    #[test]
    fn token_contract_info_empty_version_on_request_missing_version() {
        let request = GetTokenContractInfoRequest { version: None };
        let response = response_with_proof_and_metadata();
        let err = <TokenContractInfo as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyVersion), "got: {err:?}");
    }

    #[test]
    fn token_contract_info_request_error_when_token_id_wrong_length() {
        // 16-byte token_id — not 32
        let request = GetTokenContractInfoRequest {
            version: Some(ReqVersion::V0(GetTokenContractInfoRequestV0 {
                token_id: vec![0u8; 16],
                prove: true,
            })),
        };
        let response = response_with_proof_and_metadata();
        let err = <TokenContractInfo as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(
                    error.contains("token_id"),
                    "error should mention token_id, got: {error}"
                );
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn token_contract_info_request_error_when_token_id_too_long() {
        let request = GetTokenContractInfoRequest {
            version: Some(ReqVersion::V0(GetTokenContractInfoRequestV0 {
                token_id: vec![1u8; 64], // too long
                prove: true,
            })),
        };
        let response = response_with_proof_and_metadata();
        let err = <TokenContractInfo as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::RequestError { .. }), "got: {err:?}");
    }

    #[test]
    fn token_contract_info_empty_response_metadata_when_metadata_missing() {
        let request = GetTokenContractInfoRequest {
            version: Some(ReqVersion::V0(GetTokenContractInfoRequestV0 {
                token_id: vec![0u8; 32],
                prove: true,
            })),
        };
        let response = GetTokenContractInfoResponse {
            version: Some(RespVersion::V0(GetTokenContractInfoResponseV0 {
                result: Some(RespResult::Proof(Proof::default())),
                metadata: None,
            })),
        };
        let err = <TokenContractInfo as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }

    #[test]
    fn token_contract_info_no_proof_when_response_has_no_result() {
        let request = GetTokenContractInfoRequest {
            version: Some(ReqVersion::V0(GetTokenContractInfoRequestV0 {
                token_id: vec![0u8; 32],
                prove: true,
            })),
        };
        // result=None — proof is missing
        let response = GetTokenContractInfoResponse {
            version: Some(RespVersion::V0(GetTokenContractInfoResponseV0 {
                result: None,
                metadata: Some(ResponseMetadata::default()),
            })),
        };
        let err = <TokenContractInfo as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::NoProofInResult), "got: {err:?}");
    }

    #[test]
    fn token_contract_info_empty_response_metadata_when_response_is_default() {
        // response.version = None. In `TokenContractInfo::maybe_from_proof`,
        // `response.metadata()` is called BEFORE `response.proof_owned()`, and
        // the derived `VersionedGrpcResponse` impl returns Err when version is
        // None, which maps to `Error::EmptyResponseMetadata`. Ordering is
        // deterministic, so this is the only acceptable outcome.
        let request = GetTokenContractInfoRequest {
            version: Some(ReqVersion::V0(GetTokenContractInfoRequestV0 {
                token_id: vec![0u8; 32],
                prove: true,
            })),
        };
        let response = GetTokenContractInfoResponse::default();
        let err = <TokenContractInfo as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(matches!(err, Error::EmptyResponseMetadata), "got: {err:?}");
    }
}
