//! Identity related types and functions

use async_trait::async_trait;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
use dapi_grpc::platform::v0::{
    get_identity_balance_and_revision_request, get_identity_balance_request,
    get_identity_by_public_key_hash_request, get_identity_request,
    GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceRequest,
    GetIdentityByPublicKeyHashRequest, GetIdentityRequest,
};
use dpp::prelude::Identity;

use crate::{delegate_enum, Sdk};
use crate::{
    platform::{proto, Query},
    Error,
};

// Create enum [IdentityRequest] and [IdentityResponse] that will wrap all possible
// request/response types for [Identity] object.
delegate_enum! {
    IdentityRequest,
    IdentityResponse,
    Identity,
    (GetIdentity,proto::GetIdentityRequest,proto::GetIdentityResponse),
    (GetIdentityByPublicKeyHash, proto::GetIdentityByPublicKeyHashRequest, proto::GetIdentityByPublicKeyHashResponse)
}

impl Query<IdentityRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<IdentityRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(IdentityRequest::GetIdentity(GetIdentityRequest {
            version: Some(get_identity_request::Version::V0(GetIdentityRequestV0 {
                id,
                prove: true,
            })),
        }))
    }
}

/// Public key hash that can be used as a [Query] to find an identity.
///
/// You can use [`Fetch::fetch(PublicKeyHash)`](crate::platform::Fetch::fetch()) to fetch an identity
/// by its public key hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKeyHash(pub [u8; 20]);

impl Query<IdentityRequest> for PublicKeyHash {
    fn query(self, prove: bool) -> Result<IdentityRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let request: GetIdentityByPublicKeyHashRequest = GetIdentityByPublicKeyHashRequest {
            version: Some(get_identity_by_public_key_hash_request::Version::V0(
                GetIdentityByPublicKeyHashRequestV0 {
                    prove,
                    public_key_hash: self.0.to_vec(),
                },
            )),
        };

        Ok(request.into())
    }
}

impl Query<GetIdentityBalanceRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetIdentityBalanceRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();

        let request: GetIdentityBalanceRequest = GetIdentityBalanceRequest {
            version: Some(get_identity_balance_request::Version::V0(
                GetIdentityBalanceRequestV0 { id, prove },
            )),
        };

        Ok(request)
    }
}

impl Query<GetIdentityBalanceAndRevisionRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetIdentityBalanceAndRevisionRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        let request: GetIdentityBalanceAndRevisionRequest = GetIdentityBalanceAndRevisionRequest {
            version: Some(get_identity_balance_and_revision_request::Version::V0(
                GetIdentityBalanceAndRevisionRequestV0 { id, prove },
            )),
        };

        Ok(request)
    }
}

/// A trait for identity related types
#[async_trait]
pub trait IdentityEx {
    /// Register a new identity with provided amount of Dash.
    ///
    ///
    async fn register(sdk: &Sdk, amount: u64) -> Result<Identity, Error>;
}

#[async_trait]
impl IdentityEx for Identity {
    async fn register(sdk: &Sdk, amount: u64) -> Result<Identity, Error> {
        let mut wg = sdk.wallet().await;
        let wallet = wg.as_mut().ok_or(Error::WalletNotInitialized)?;

        let asset_lock = wallet.lock_assets(1).await?;

        todo!("implement me")
        // let mut identity = Identity::new();
        // identity.set_balance(amount);
        // identity.put_to_platform_and_wait_for_response(sdk).await?;
        // Ok(identity)
    }
}
