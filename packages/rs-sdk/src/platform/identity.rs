//! Identity related types and functions

use dpp::prelude::Identity;

use crate::delegate_enum;
use crate::{
    platform::{proto, Query},
    Error,
};

delegate_enum! {
    IdentityRequest,
    IdentityResponse,
    Identity,
    (GetIdentity,proto::GetIdentityRequest,proto::GetIdentityResponse),
    (GetIdentityByPublicKeyHash, proto::GetIdentityByPublicKeyHashesRequest, proto::GetIdentityByPublicKeyHashesResponse)
}

impl Query<IdentityRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<IdentityRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let id = self.to_vec();
        Ok(IdentityRequest::GetIdentity(proto::GetIdentityRequest {
            id,
            prove: true,
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
        if prove != true {
            unimplemented!("queries without proofs are not supported yet");
        }
        let request = proto::GetIdentityByPublicKeyHashesRequest {
            prove,
            public_key_hash: self.0.to_vec(),
        };

        Ok(request.into())
    }
}
