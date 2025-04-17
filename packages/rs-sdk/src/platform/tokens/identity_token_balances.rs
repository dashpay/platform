use crate::platform::{FetchMany, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_identities_token_balances_request::GetIdentitiesTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identity_token_balances_request::GetIdentityTokenBalancesRequestV0;
use dapi_grpc::platform::v0::{
    get_identities_token_balances_request, get_identity_token_balances_request,
    GetIdentitiesTokenBalancesRequest, GetIdentityTokenBalancesRequest,
};
use dpp::balances::credits::TokenAmount;
pub use drive_proof_verifier::types::identity_token_balance::{
    IdentitiesTokenBalances, IdentityTokenBalances,
};

#[derive(Debug, Clone)]
/// Query to fetch multiple token balances of one specific identity
pub struct IdentityTokenBalancesQuery {
    /// Identity ID
    pub identity_id: Identifier,
    /// Token IDs
    pub token_ids: Vec<Identifier>,
}

impl Query<GetIdentityTokenBalancesRequest> for IdentityTokenBalancesQuery {
    fn query(self, prove: bool) -> Result<GetIdentityTokenBalancesRequest, Error> {
        let request = GetIdentityTokenBalancesRequest {
            version: Some(get_identity_token_balances_request::Version::V0(
                GetIdentityTokenBalancesRequestV0 {
                    identity_id: self.identity_id.to_vec(),
                    token_ids: self.token_ids.into_iter().map(|id| id.to_vec()).collect(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<Identifier, IdentityTokenBalances> for TokenAmount {
    type Request = GetIdentityTokenBalancesRequest;
}

#[derive(Debug, Clone)]
/// Query to fetch multiple identity balances of one specific token
pub struct IdentitiesTokenBalancesQuery {
    /// Identity IDs
    pub identity_ids: Vec<Identifier>,
    /// Token ID
    pub token_id: Identifier,
}

impl Query<GetIdentitiesTokenBalancesRequest> for IdentitiesTokenBalancesQuery {
    fn query(self, prove: bool) -> Result<GetIdentitiesTokenBalancesRequest, Error> {
        let request = GetIdentitiesTokenBalancesRequest {
            version: Some(get_identities_token_balances_request::Version::V0(
                GetIdentitiesTokenBalancesRequestV0 {
                    identity_ids: self
                        .identity_ids
                        .into_iter()
                        .map(|id| id.to_vec())
                        .collect(),
                    token_id: self.token_id.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<Identifier, IdentitiesTokenBalances> for TokenAmount {
    type Request = GetIdentitiesTokenBalancesRequest;
}
