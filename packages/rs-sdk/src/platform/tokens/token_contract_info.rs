use crate::platform::{Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_token_contract_info_request::GetTokenContractInfoRequestV0;
use dapi_grpc::platform::v0::{get_token_contract_info_request, GetTokenContractInfoRequest};

#[derive(Debug, Clone)]
/// Query to fetch contract info for a specific token
pub struct TokenContractInfoQuery {
    /// Token ID
    pub token_id: Identifier,
}

impl Query<GetTokenContractInfoRequest> for TokenContractInfoQuery {
    fn query(self, prove: bool) -> Result<GetTokenContractInfoRequest, Error> {
        let request = GetTokenContractInfoRequest {
            version: Some(get_token_contract_info_request::Version::V0(
                GetTokenContractInfoRequestV0 {
                    token_id: self.token_id.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Query<GetTokenContractInfoRequest> for Identifier {
    fn query(self, prove: bool) -> Result<GetTokenContractInfoRequest, Error> {
        TokenContractInfoQuery { token_id: self }.query(prove)
    }
}