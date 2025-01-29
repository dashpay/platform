use crate::platform::{Fetch, FetchMany, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_identity_token_balances_request::GetIdentityTokenBalancesRequestV0;
use dapi_grpc::platform::v0::{
    get_identity_token_balances_request, GetIdentitiesTokenBalancesRequest,
    GetIdentityTokenBalancesRequest,
};
use dpp::balances::credits::TokenAmount;
use drive_proof_verifier::tokens::identity_token_balance::{
    IdentitiesTokenBalances, IdentitiesTokenBalancesQuery, IdentityTokenBalances,
    IdentityTokenBalancesQuery,
};

impl Query<GetIdentityTokenBalancesRequest> for IdentityTokenBalancesQuery {
    fn query(self, prove: bool) -> Result<GetIdentityTokenBalancesRequest, Error> {
        let request = GetIdentityTokenBalancesRequest {
            version: Some(get_identity_token_balances_request::Version::V0(
                GetIdentityTokenBalancesRequestV0 {
                    identity_id: vec![],
                    token_ids: vec![],
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

impl Query<GetIdentitiesTokenBalancesRequest> for IdentitiesTokenBalancesQuery {
    fn query(self, prove: bool) -> Result<GetIdentitiesTokenBalancesRequest, Error> {
        let request = GetIdentityTokenBalancesRequest {
            version: Some(get_identity_token_balances_request::Version::V0(
                GetIdentityTokenBalancesRequestV0 {
                    identity_id: vec![],
                    token_ids: vec![],
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Fetch for IdentitiesTokenBalances {
    type Request = GetIdentitiesTokenBalancesRequest;
}
