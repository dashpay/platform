//! `GetIdentity` requests.

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use tonic::IntoRequest;

use crate::{settings::AppliedSettings, DapiRequest, GrpcRequestError, Settings};

use super::PlatformGrpcClient;

/// Request Identity bytes.
pub struct GetIdentity {
    /// Identity ID to search.
    pub id: Vec<u8>,
}

/// DAPI response for [GetIdentity].
pub struct GetIdentityResponse {
    /// Serialized Identity
    pub identity_bytes: Vec<u8>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetIdentity {
    type DapiResponse = Option<GetIdentityResponse>;

    const SETTINGS_OVERRIDES: Settings = Settings::empty();

    type Transport = PlatformGrpcClient;

    type Error = GrpcRequestError;

    fn prepare<'c>(
        &self,
        client: &'c mut Self::Transport,
        settings: AppliedSettings,
    ) -> BoxFuture<'c, Result<Self::DapiResponse, Self::Error>> {
        let mut grpc_request = platform_proto::GetIdentityRequest {
            id: self.id.clone(),
            prove: false,
        }
        .into_request();
        grpc_request.set_timeout(settings.timeout);

        async {
            let fetch_response = client
                .get_identity(grpc_request)
                .map_ok(tonic::Response::<platform_proto::GetIdentityResponse>::into_inner)
                .map_err(Into::<Self::Error>::into)
                .await?;

            use platform_proto::get_identity_response::Result as GrpcResponseBody;
            use platform_proto::GetIdentityResponse as GrpcResponse;

            match fetch_response {
                GrpcResponse { result: None, .. } => Ok(None),
                GrpcResponse {
                    result: Some(GrpcResponseBody::Identity(identity_bytes)),
                    metadata: Some(metadata),
                } => Ok(Some(GetIdentityResponse {
                    identity_bytes,
                    metadata,
                })),
                _ => Err(Self::Error::IncompleteResponse),
            }
        }
        .boxed()
    }
}

/// Request Identity bytes wrapped into proof.
pub struct GetIdentityProof {
    /// Identity ID to search.
    pub id: Vec<u8>,
}

/// DAPI response for [GetIdentity].
pub struct GetIdentityProofResponse {
    /// Proof data that wraps Identity
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

impl DapiRequest for GetIdentityProof {
    type DapiResponse = GetIdentityProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::empty();

    type Transport = PlatformGrpcClient;

    type Error = GrpcRequestError;

    fn prepare<'c>(
        &self,
        client: &'c mut Self::Transport,
        settings: AppliedSettings,
    ) -> BoxFuture<'c, Result<Self::DapiResponse, Self::Error>> {
        let mut grpc_request = platform_proto::GetIdentityRequest {
            id: self.id.clone(),
            prove: true,
        }
        .into_request();
        grpc_request.set_timeout(settings.timeout);

        async {
            let fetch_response = client
                .get_identity(grpc_request)
                .map_ok(tonic::Response::<platform_proto::GetIdentityResponse>::into_inner)
                .map_err(Into::<Self::Error>::into)
                .await?;

            use platform_proto::get_identity_response::Result as GrpcResponseBody;
            use platform_proto::GetIdentityResponse as GrpcResponse;

            match fetch_response {
                GrpcResponse {
                    result: Some(GrpcResponseBody::Proof(proof)),
                    metadata: Some(metadata),
                } => Ok(GetIdentityProofResponse { proof, metadata }),
                _ => Err(Self::Error::IncompleteResponse),
            }
        }
        .boxed()
    }
}
