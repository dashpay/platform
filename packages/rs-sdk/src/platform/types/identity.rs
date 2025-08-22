//! Identity related types and functions

use crate::delegate_enum;
use crate::{
    platform::{proto, Query},
    Error,
};

use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_request::GetIdentityBalanceAndRevisionRequestV0;
use dapi_grpc::platform::v0::get_identity_balance_request::GetIdentityBalanceRequestV0;
use dapi_grpc::platform::v0::get_identity_by_non_unique_public_key_hash_request::GetIdentityByNonUniquePublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0;
use dapi_grpc::platform::v0::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0;
use dapi_grpc::platform::v0::get_identity_nonce_request::GetIdentityNonceRequestV0;
use dapi_grpc::platform::v0::get_identity_request::GetIdentityRequestV0;
use dapi_grpc::platform::v0::{
    get_identities_balances_request, get_identity_balance_and_revision_request,
    get_identity_balance_request, get_identity_by_public_key_hash_request,
    get_identity_contract_nonce_request, get_identity_nonce_request, get_identity_request,
    GetIdentitiesBalancesRequest, GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceRequest,
    GetIdentityByPublicKeyHashRequest, GetIdentityContractNonceRequest, GetIdentityNonceRequest,
    GetIdentityRequest, ResponseMetadata,
};
use dpp::prelude::Identity;
use rs_dapi_client::transport::TransportError;

// Create enum [IdentityRequest] and [IdentityResponse] that will wrap all possible
// request/response types for [Identity] object.
delegate_enum! {
    IdentityRequest,
    IdentityResponse,
    Identity,
    (GetIdentity,proto::GetIdentityRequest,proto::GetIdentityResponse),
    (GetIdentityByPublicKeyHash, proto::GetIdentityByPublicKeyHashRequest, proto::GetIdentityByPublicKeyHashResponse),
    (GetIdentityByNonUniquePublicKeyHash, proto::GetIdentityByNonUniquePublicKeyHashRequest, proto::GetIdentityByNonUniquePublicKeyHashResponse)
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
                prove,
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

/// Non-unique public key hash that can be used as a [Query] to find an identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonUniquePublicKeyHashQuery {
    pub key_hash: [u8; 20],
    pub after: Option<[u8; 32]>,
}

impl Query<IdentityRequest> for NonUniquePublicKeyHashQuery {
    fn query(self, prove: bool) -> Result<IdentityRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let request = proto::GetIdentityByNonUniquePublicKeyHashRequest {
            version: Some(
                proto::get_identity_by_non_unique_public_key_hash_request::Version::V0(
                    GetIdentityByNonUniquePublicKeyHashRequestV0 {
                        public_key_hash: self.key_hash.to_vec(),
                        start_after: self.after.map(|a| a.to_vec()),
                        prove,
                    },
                ),
            ),
        }
        .into();

        Ok(request)
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

impl Query<GetIdentityNonceRequest> for dpp::prelude::Identifier {
    fn query(self, prove: bool) -> Result<GetIdentityNonceRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }

        let request: GetIdentityNonceRequest = GetIdentityNonceRequest {
            version: Some(get_identity_nonce_request::Version::V0(
                GetIdentityNonceRequestV0 {
                    identity_id: self.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Query<GetIdentityContractNonceRequest>
    for (dpp::prelude::Identifier, dpp::prelude::Identifier)
{
    fn query(self, prove: bool) -> Result<GetIdentityContractNonceRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let (identity_id, contract_id) = self;

        let request: GetIdentityContractNonceRequest = GetIdentityContractNonceRequest {
            version: Some(get_identity_contract_nonce_request::Version::V0(
                GetIdentityContractNonceRequestV0 {
                    identity_id: identity_id.to_vec(),
                    contract_id: contract_id.to_vec(),
                    prove,
                },
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

impl Query<GetIdentitiesBalancesRequest> for Vec<dpp::prelude::Identifier> {
    fn query(self, prove: bool) -> Result<GetIdentitiesBalancesRequest, Error> {
        if !prove {
            unimplemented!("queries without proofs are not supported yet");
        }
        let ids = self.into_iter().map(|a| a.to_vec()).collect();

        let request: GetIdentitiesBalancesRequest = GetIdentitiesBalancesRequest {
            version: Some(get_identities_balances_request::Version::V0(
                GetIdentitiesBalancesRequestV0 { ids, prove },
            )),
        };

        Ok(request)
    }
}
