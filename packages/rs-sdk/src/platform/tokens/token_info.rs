use crate::platform::{FetchMany, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_identities_token_infos_request::GetIdentitiesTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_identity_token_infos_request::GetIdentityTokenInfosRequestV0;
use dapi_grpc::platform::v0::{
    get_identities_token_infos_request, get_identity_token_infos_request,
    GetIdentitiesTokenInfosRequest, GetIdentityTokenInfosRequest,
};
use dpp::tokens::info::IdentityTokenInfo;
use drive_proof_verifier::types::token_info::{IdentitiesTokenInfos, IdentityTokenInfos};

#[derive(Debug, Clone)]
/// Query to fetch multiple token information of one specific identity
pub struct IdentityTokenInfosQuery {
    /// Identity ID
    pub identity_id: Identifier,
    /// Token IDs
    pub token_ids: Vec<Identifier>,
}

impl Query<GetIdentityTokenInfosRequest> for IdentityTokenInfosQuery {
    fn query(self, prove: bool) -> Result<GetIdentityTokenInfosRequest, Error> {
        let request = GetIdentityTokenInfosRequest {
            version: Some(get_identity_token_infos_request::Version::V0(
                GetIdentityTokenInfosRequestV0 {
                    identity_id: self.identity_id.to_vec(),
                    token_ids: self.token_ids.into_iter().map(|id| id.to_vec()).collect(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<Identifier, IdentityTokenInfos> for IdentityTokenInfo {
    type Request = GetIdentityTokenInfosRequest;
}

#[derive(Debug, Clone)]
/// Query to fetch multiple identity token information of one specific token
pub struct IdentitiesTokenInfosQuery {
    /// Identity IDs
    pub identity_ids: Vec<Identifier>,
    /// Token ID
    pub token_id: Identifier,
}

impl Query<GetIdentitiesTokenInfosRequest> for IdentitiesTokenInfosQuery {
    fn query(self, prove: bool) -> Result<GetIdentitiesTokenInfosRequest, Error> {
        let request = GetIdentitiesTokenInfosRequest {
            version: Some(get_identities_token_infos_request::Version::V0(
                GetIdentitiesTokenInfosRequestV0 {
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

// TODO: Implement Fetch (and for others)

impl FetchMany<Identifier, IdentitiesTokenInfos> for IdentityTokenInfo {
    type Request = GetIdentitiesTokenInfosRequest;
}
