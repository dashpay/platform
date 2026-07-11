use crate::error::MapGroveDbError;
use crate::types::identity_token_balance::{IdentitiesTokenBalances, IdentityTokenBalances};
use crate::verify::verify_tenderdash_proof;
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_identities_token_balances_request, get_identity_token_balances_request,
    GetIdentitiesTokenBalancesRequest, GetIdentitiesTokenBalancesResponse,
    GetIdentityTokenBalancesRequest, GetIdentityTokenBalancesResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetIdentityTokenBalancesRequest> for IdentityTokenBalances {
    type Request = GetIdentityTokenBalancesRequest;
    type Response = GetIdentityTokenBalancesResponse;

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
            get_identity_token_balances_request::Version::V0(v0) => {
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

        let (root_hash, result) = Drive::verify_token_balances_for_identity_id(
            &proof.grovedb_proof,
            &token_ids,
            identity_id,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}

impl FromProof<GetIdentitiesTokenBalancesRequest> for IdentitiesTokenBalances {
    type Request = GetIdentitiesTokenBalancesRequest;
    type Response = GetIdentitiesTokenBalancesResponse;

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
            get_identities_token_balances_request::Version::V0(v0) => {
                let token_id = <[u8; 32]>::try_from(v0.token_id.as_slice()).map_err(|error| {
                    Error::RequestError {
                        error: error.to_string(),
                    }
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

        let (root_hash, result) = Drive::verify_token_balances_for_identity_ids(
            &proof.grovedb_proof,
            token_id,
            &identity_ids,
            false,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        Ok((Some(result), metadata, proof))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::get_identities_token_balances_request::{
        GetIdentitiesTokenBalancesRequestV0, Version as IdsReqVersion,
    };
    use dapi_grpc::platform::v0::get_identity_token_balances_request::{
        GetIdentityTokenBalancesRequestV0, Version as IdReqVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use std::sync::Arc;

    struct UnreachableProvider;

    impl ContextProvider for UnreachableProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _pv: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            panic!("should not be called")
        }
        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            panic!("should not be called")
        }
        fn get_quorum_public_key(
            &self,
            _qt: u32,
            _qh: [u8; 32],
            _h: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            panic!("should not be called")
        }
        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            panic!("should not be called")
        }
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    // -------- IdentityTokenBalances --------

    #[test]
    fn identity_token_balances_empty_version_on_request_missing() {
        let request = GetIdentityTokenBalancesRequest { version: None };
        let response = GetIdentityTokenBalancesResponse::default();
        let err = <IdentityTokenBalances as FromProof<_>>::maybe_from_proof(
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
    fn identity_token_balances_request_error_on_bad_identity_id() {
        let request = GetIdentityTokenBalancesRequest {
            version: Some(IdReqVersion::V0(GetIdentityTokenBalancesRequestV0 {
                identity_id: vec![0u8; 10], // bad
                token_ids: vec![vec![0u8; 32]],
                prove: true,
            })),
        };
        let response = GetIdentityTokenBalancesResponse::default();
        let err = <IdentityTokenBalances as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("identity_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn identity_token_balances_request_error_on_bad_token_id() {
        let request = GetIdentityTokenBalancesRequest {
            version: Some(IdReqVersion::V0(GetIdentityTokenBalancesRequestV0 {
                identity_id: vec![0u8; 32],
                token_ids: vec![vec![0u8; 32], vec![1u8; 5]], // second token_id bad
                prove: true,
            })),
        };
        let response = GetIdentityTokenBalancesResponse::default();
        let err = <IdentityTokenBalances as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("token_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    // -------- IdentitiesTokenBalances --------

    #[test]
    fn identities_token_balances_empty_version_on_request_missing() {
        let request = GetIdentitiesTokenBalancesRequest { version: None };
        let response = GetIdentitiesTokenBalancesResponse::default();
        let err = <IdentitiesTokenBalances as FromProof<_>>::maybe_from_proof(
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
    fn identities_token_balances_request_error_on_bad_token_id() {
        let request = GetIdentitiesTokenBalancesRequest {
            version: Some(IdsReqVersion::V0(GetIdentitiesTokenBalancesRequestV0 {
                token_id: vec![0u8; 7], // bad
                identity_ids: vec![vec![1u8; 32]],
                prove: true,
            })),
        };
        let response = GetIdentitiesTokenBalancesResponse::default();
        let err = <IdentitiesTokenBalances as FromProof<_>>::maybe_from_proof(
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
    fn identities_token_balances_request_error_on_bad_identity_id() {
        let request = GetIdentitiesTokenBalancesRequest {
            version: Some(IdsReqVersion::V0(GetIdentitiesTokenBalancesRequestV0 {
                token_id: vec![0u8; 32],
                identity_ids: vec![vec![1u8; 32], vec![2u8; 33]], // too long
                prove: true,
            })),
        };
        let response = GetIdentitiesTokenBalancesResponse::default();
        let err = <IdentitiesTokenBalances as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(error.contains("identity_id"), "got: {error}"),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }
}
