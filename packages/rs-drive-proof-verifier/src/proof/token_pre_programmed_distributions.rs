use crate::error::MapGroveDbError;
use crate::verify::verify_tenderdash_proof;
use crate::{types::TokenPreProgrammedDistributions, ContextProvider, Error};
use dapi_grpc::platform::v0::{
    get_token_pre_programmed_distributions_request, GetTokenPreProgrammedDistributionsRequest,
    GetTokenPreProgrammedDistributionsResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::Network;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use drive::drive::tokens::distribution::queries::QueryPreProgrammedDistributionStartAt;
use drive::drive::Drive;

use super::FromProof;

impl FromProof<GetTokenPreProgrammedDistributionsRequest> for TokenPreProgrammedDistributions {
    type Request = GetTokenPreProgrammedDistributionsRequest;
    type Response = GetTokenPreProgrammedDistributionsResponse;

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

        let get_token_pre_programmed_distributions_request::Version::V0(req_v0) =
            request.version.ok_or(Error::EmptyVersion)?;

        let token_id: [u8; 32] =
            req_v0
                .token_id
                .as_slice()
                .try_into()
                .map_err(|_| Error::RequestError {
                    error: "token_id must be 32 bytes".into(),
                })?;

        let start_at = match req_v0.start_at_info {
            Some(start_at_info) => {
                let start_at_recipient = match start_at_info.start_recipient {
                    Some(recipient_bytes) => {
                        let recipient_id =
                            Identifier::from_bytes(&recipient_bytes).map_err(|_| {
                                Error::RequestError {
                                    error: "start_recipient must be 32 bytes".into(),
                                }
                            })?;
                        // Default to inclusive: if omitted the start recipient is included.
                        let included = start_at_info.start_recipient_included.unwrap_or(true);
                        Some((recipient_id, included))
                    }
                    None => None,
                };

                Some(QueryPreProgrammedDistributionStartAt {
                    start_at_time: start_at_info.start_time_ms,
                    start_at_recipient,
                })
            }
            None => None,
        };

        let limit = req_v0
            .limit
            .map(|l| {
                u16::try_from(l).map_err(|_| Error::RequestError {
                    error: "limit exceeds u16::MAX".into(),
                })
            })
            .transpose()?;

        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();

        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;

        let (root_hash, result): ([u8; 32], TokenPreProgrammedDistributions) =
            Drive::verify_token_pre_programmed_distributions(
                &proof.grovedb_proof,
                token_id,
                start_at,
                limit,
                false,
                platform_version,
            )
            .map_drive_error(&proof, &metadata)?;

        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;

        if result.0.is_empty() {
            Ok((None, metadata, proof))
        } else {
            Ok((Some(result), metadata, proof))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FromProof;
    use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::{
        get_token_pre_programmed_distributions_request_v0::StartAtInfo,
        GetTokenPreProgrammedDistributionsRequestV0, Version as ReqVersion,
    };
    use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_response::{
        get_token_pre_programmed_distributions_response_v0::Result as RespResult,
        GetTokenPreProgrammedDistributionsResponseV0, Version as RespVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{CoreBlockHeight, DataContract};
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

    fn response_with_proof() -> GetTokenPreProgrammedDistributionsResponse {
        GetTokenPreProgrammedDistributionsResponse {
            version: Some(RespVersion::V0(
                GetTokenPreProgrammedDistributionsResponseV0 {
                    result: Some(RespResult::Proof(Proof::default())),
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        }
    }

    fn req_v0_defaults() -> GetTokenPreProgrammedDistributionsRequestV0 {
        GetTokenPreProgrammedDistributionsRequestV0 {
            token_id: vec![0u8; 32],
            start_at_info: None,
            limit: None,
            prove: true,
        }
    }

    #[test]
    fn empty_version_when_request_has_no_version() {
        let request = GetTokenPreProgrammedDistributionsRequest { version: None };
        let response = response_with_proof();
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
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
        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(ReqVersion::V0(
                GetTokenPreProgrammedDistributionsRequestV0 {
                    token_id: vec![0u8; 5], // invalid
                    ..req_v0_defaults()
                },
            )),
        };
        let response = response_with_proof();
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
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

    #[test]
    fn request_error_when_start_recipient_wrong_length() {
        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(ReqVersion::V0(
                GetTokenPreProgrammedDistributionsRequestV0 {
                    token_id: vec![1u8; 32],
                    start_at_info: Some(StartAtInfo {
                        start_time_ms: 10_000,
                        start_recipient: Some(vec![1u8; 16]), // not 32 bytes
                        start_recipient_included: Some(true),
                    }),
                    limit: None,
                    prove: true,
                },
            )),
        };
        let response = response_with_proof();
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("start_recipient"), "got: {error}")
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn request_error_when_limit_exceeds_u16_max() {
        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(ReqVersion::V0(
                GetTokenPreProgrammedDistributionsRequestV0 {
                    token_id: vec![2u8; 32],
                    start_at_info: None,
                    limit: Some(u32::MAX), // exceeds u16::MAX
                    prove: true,
                },
            )),
        };
        let response = response_with_proof();
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        match err {
            Error::RequestError { error } => {
                assert!(error.contains("limit"), "got: {error}");
                assert!(error.contains("u16"), "got: {error}");
            }
            other => panic!("expected RequestError, got: {other:?}"),
        }
    }

    #[test]
    fn empty_response_metadata_when_metadata_missing() {
        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(ReqVersion::V0(req_v0_defaults())),
        };
        // No version ⇒ metadata() errors ⇒ EmptyResponseMetadata.
        let response = GetTokenPreProgrammedDistributionsResponse { version: None };
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
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
        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(ReqVersion::V0(req_v0_defaults())),
        };
        let response = GetTokenPreProgrammedDistributionsResponse {
            version: Some(RespVersion::V0(
                GetTokenPreProgrammedDistributionsResponseV0 {
                    result: None,
                    metadata: Some(ResponseMetadata::default()),
                },
            )),
        };
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
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
    fn limit_at_u16_max_does_not_error_on_conversion() {
        // Using limit = u16::MAX as u32 — conversion must succeed.
        //
        // To stop deterministically BEFORE Drive verification and the provider,
        // we use a response whose `version = None`. Control flow in
        // `maybe_from_proof_with_metadata` is:
        //   version -> token_id -> start_at -> limit conversion -> metadata -> proof -> drive
        // So the limit conversion runs first; if it wrongly rejected u16::MAX,
        // we'd see `Error::RequestError { "limit exceeds u16::MAX" }`.
        // Instead, the conversion should succeed and we should fall through to
        // the deterministic `EmptyResponseMetadata` branch.
        let request = GetTokenPreProgrammedDistributionsRequest {
            version: Some(ReqVersion::V0(
                GetTokenPreProgrammedDistributionsRequestV0 {
                    token_id: vec![3u8; 32],
                    start_at_info: None,
                    limit: Some(u16::MAX as u32),
                    prove: true,
                },
            )),
        };
        // response.version = None ⇒ metadata() errors ⇒ EmptyResponseMetadata,
        // which fires BEFORE proof_owned() or Drive verification, and well
        // before the provider could ever be consulted.
        let response = GetTokenPreProgrammedDistributionsResponse { version: None };
        let err = <TokenPreProgrammedDistributions as FromProof<_>>::maybe_from_proof(
            request,
            response,
            Network::Testnet,
            pv(),
            &UnreachableProvider,
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::EmptyResponseMetadata),
            "expected EmptyResponseMetadata (proves limit conversion succeeded), got: {err:?}"
        );
    }
}
