//! Identity related types and functions

use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
use dapi_grpc::platform::v0::{GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceRequest};
use dpp::prelude::Identity;

use crate::delegate_enum;
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
        Ok(IdentityRequest::GetIdentity(
            GetIdentityRequestV0 { id, prove: true }.into(),
        ))
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
        let request: proto::GetIdentityByPublicKeyHashRequest =
            GetIdentityByPublicKeyHashRequestV0 {
                prove,
                public_key_hash: self.0.to_vec(),
            }
            .into();

        Ok(request.into())
    }
}

impl Query<GetIdentityBalanceRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetIdentityBalanceRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(GetIdentityBalanceRequestV0 { id, prove }.into())
    }
}

impl Query<GetIdentityBalanceAndRevisionRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetIdentityBalanceAndRevisionRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(GetIdentityBalanceAndRevisionRequestV0 { id, prove }.into())
    }
}
