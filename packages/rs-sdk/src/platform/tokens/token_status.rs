use crate::platform::{FetchMany, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_token_statuses_request::GetTokenStatusesRequestV0;
use dapi_grpc::platform::v0::{get_token_statuses_request, GetTokenStatusesRequest};
use dpp::tokens::status::TokenStatus;
use drive_proof_verifier::types::token_status::TokenStatuses;

impl Query<GetTokenStatusesRequest> for Vec<Identifier> {
    fn query(self, prove: bool) -> Result<GetTokenStatusesRequest, Error> {
        let request = GetTokenStatusesRequest {
            version: Some(get_token_statuses_request::Version::V0(
                GetTokenStatusesRequestV0 {
                    token_ids: self
                        .into_iter()
                        .map(|identifier| identifier.to_vec())
                        .collect(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl FetchMany<Identifier, TokenStatuses> for TokenStatus {
    type Request = GetTokenStatusesRequest;
}
