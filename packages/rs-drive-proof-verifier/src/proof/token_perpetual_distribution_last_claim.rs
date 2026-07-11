use dapi_grpc::platform::v0::{
    get_token_perpetual_distribution_last_claim_request::Version as RequestVersion,
    get_token_perpetual_distribution_last_claim_response::{
        get_token_perpetual_distribution_last_claim_response_v0, Version as ResponseVersion,
    },
    GetTokenPerpetualDistributionLastClaimResponse, Proof, ResponseMetadata,
};
use dpp::{
    dashcore::Network,
    data_contract::associated_token::{
        token_configuration::accessors::v0::TokenConfigurationV0Getters,
        token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters,
        token_perpetual_distribution::{
            methods::v0::TokenPerpetualDistributionV0Accessors,
            reward_distribution_moment::RewardDistributionMoment,
        },
    },
    prelude::Identifier,
    version::PlatformVersion,
};
use drive::drive::Drive;
use get_token_perpetual_distribution_last_claim_response_v0::Result as RespResult;

use crate::{verify::verify_tenderdash_proof, ContextProvider, Error};

use super::FromProof;
use dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest;

impl FromProof<GetTokenPerpetualDistributionLastClaimRequest> for RewardDistributionMoment {
    type Request = GetTokenPerpetualDistributionLastClaimRequest;
    type Response = GetTokenPerpetualDistributionLastClaimResponse;

    /// Parse & verify the last‑claim proof returned by Platform.
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
        let request = request.into();
        let response = response.into();

        let RequestVersion::V0(req_v0) = request.version.ok_or(Error::EmptyVersion)?;

        let token_id: [u8; 32] =
            req_v0
                .token_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::RequestError {
                    error: "token_id must be 32 bytes".into(),
                })?;

        let identity_id: [u8; 32] =
            req_v0
                .identity_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::RequestError {
                    error: "identity_id must be 32 bytes".into(),
                })?;

        let ResponseVersion::V0(resp_v0) = response.version.ok_or(Error::EmptyVersion)?;

        let metadata = resp_v0
            .metadata
            .clone()
            .ok_or(Error::EmptyResponseMetadata)?;

        let result = resp_v0.result.clone().ok_or(Error::NoProofInResult)?;

        match result {
            RespResult::Proof(proof_msg) => {
                let maybe_distribution_type = {
                    let token_id_identifier = Identifier::from_vec(req_v0.token_id.clone())
                        .map_err(|_| Error::RequestError {
                            error: "token_id must be 32 bytes".into(),
                        })?;

                    let maybe_token_config =
                        provider.get_token_configuration(&token_id_identifier)?;
                    let maybe_dist_type = maybe_token_config
                        .as_ref()
                        .and_then(|cfg| cfg.distribution_rules().perpetual_distribution())
                        .map(|perp| perp.distribution_type().clone());

                    maybe_dist_type
                };

                match maybe_distribution_type {
                    Some(distribution_type) => {
                        let (root_hash, moment_opt) =
                            Drive::verify_token_perpetual_distribution_last_paid_time(
                                &proof_msg.grovedb_proof,
                                token_id,
                                identity_id,
                                &distribution_type,
                                false,
                                platform_version,
                            )?;

                        verify_tenderdash_proof(&proof_msg, &metadata, &root_hash, provider)?;

                        // May be None if identity has not yet claimed
                        Ok((moment_opt, metadata, proof_msg))
                    }
                    None => Err(Error::RequestError {
                        error: "Token distribution type not found with get_token_distribution()"
                            .into(),
                    }),
                }
            }

            RespResult::LastClaim(_) => Err(Error::RequestError {
                error: "Non-proof LastClaim response is not supported in rs-sdk".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromProof;
    use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_request::{
        GetTokenPerpetualDistributionLastClaimRequestV0, Version as ReqVersion,
    };
    use dapi_grpc::platform::v0::get_token_perpetual_distribution_last_claim_response::{
        get_token_perpetual_distribution_last_claim_response_v0::LastClaimInfo,
        GetTokenPerpetualDistributionLastClaimResponseV0, Version as RespVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
    use std::sync::Arc;

    /// Provider that panics — tests must error out before reaching it.
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

    /// Provider that reports no token configuration — used to exercise the
    /// "Token distribution type not found" error branch.
    struct NoTokenConfigProvider;

    impl ContextProvider for NoTokenConfigProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _pv: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }
        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None) // no config ⇒ distribution type is None
        }
        fn get_quorum_public_key(
            &self,
            _qt: u32,
            _qh: [u8; 32],
            _h: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            // Unreachable in these tests: we fail before tenderdash proof verification.
            Ok([0u8; 48])
        }
        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    fn pv() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    fn make_response_with_proof() -> GetTokenPerpetualDistributionLastClaimResponse {
        GetTokenPerpetualDistributionLastClaimResponse {
            version: Some(RespVersion::V0(
                GetTokenPerpetualDistributionLastClaimResponseV0 {
                    result: Some(RespResult::Proof(Proof::default())),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn make_request(
        token_id: Vec<u8>,
        identity_id: Vec<u8>,
    ) -> GetTokenPerpetualDistributionLastClaimRequest {
        GetTokenPerpetualDistributionLastClaimRequest {
            version: Some(ReqVersion::V0(
                GetTokenPerpetualDistributionLastClaimRequestV0 {
                    token_id,
                    contract_info: None,
                    identity_id,
                    prove: true,
                },
            )),
        }
    }

    #[test]
    fn empty_version_when_request_has_no_version() {
        let request = GetTokenPerpetualDistributionLastClaimRequest { version: None };
        let response = make_response_with_proof();
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
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
    fn request_error_when_token_id_wrong_length() {
        let request = make_request(vec![0u8; 16], vec![1u8; 32]);
        let response = make_response_with_proof();
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(
                error.contains("token_id"),
                "error should mention token_id, got: {error}"
            ),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn request_error_when_identity_id_wrong_length() {
        let request = make_request(vec![0u8; 32], vec![1u8; 10]);
        let response = make_response_with_proof();
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(
                error.contains("identity_id"),
                "error should mention identity_id, got: {error}"
            ),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn empty_version_when_response_has_no_version() {
        let request = make_request(vec![0u8; 32], vec![1u8; 32]);
        let response = GetTokenPerpetualDistributionLastClaimResponse { version: None };
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
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
    fn empty_response_metadata_when_metadata_missing() {
        let request = make_request(vec![0u8; 32], vec![1u8; 32]);
        let response = GetTokenPerpetualDistributionLastClaimResponse {
            version: Some(RespVersion::V0(
                GetTokenPerpetualDistributionLastClaimResponseV0 {
                    result: Some(RespResult::Proof(Proof::default())),
                    metadata: None,
                },
            )),
        };
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
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
    fn no_proof_in_result_when_result_missing() {
        let request = make_request(vec![0u8; 32], vec![1u8; 32]);
        let response = GetTokenPerpetualDistributionLastClaimResponse {
            version: Some(RespVersion::V0(
                GetTokenPerpetualDistributionLastClaimResponseV0 {
                    result: None,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        };
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
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
    fn request_error_when_last_claim_variant_returned_instead_of_proof() {
        // This branch explicitly rejects direct LastClaim responses.
        let request = make_request(vec![0u8; 32], vec![1u8; 32]);
        let response = GetTokenPerpetualDistributionLastClaimResponse {
            version: Some(RespVersion::V0(
                GetTokenPerpetualDistributionLastClaimResponseV0 {
                    result: Some(RespResult::LastClaim(LastClaimInfo { paid_at: None })),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        };
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(
                error.contains("Non-proof LastClaim"),
                "error should mention Non-proof LastClaim, got: {error}"
            ),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn request_error_when_token_config_returns_none() {
        // token_id is valid 32 bytes; provider returns None for the token config.
        // The code builds `maybe_distribution_type = None` and errors with
        // "Token distribution type not found".
        let request = make_request(vec![42u8; 32], vec![7u8; 32]);
        let response = make_response_with_proof();
        let err = <RewardDistributionMoment as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &NoTokenConfigProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => assert!(
                error.contains("Token distribution type not found"),
                "error should mention 'Token distribution type not found', got: {error}"
            ),
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }
}
