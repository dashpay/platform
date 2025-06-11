use crate::platform::{Fetch, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_token_contract_info_request::GetTokenContractInfoRequestV0;
use dapi_grpc::platform::v0::{get_token_contract_info_request, GetTokenContractInfoRequest};
use dpp::tokens::contract_info::TokenContractInfo;

impl Query<GetTokenContractInfoRequest> for Identifier {
    fn query(self, prove: bool) -> Result<GetTokenContractInfoRequest, Error> {
        let request = GetTokenContractInfoRequest {
            version: Some(get_token_contract_info_request::Version::V0(
                GetTokenContractInfoRequestV0 {
                    token_id: self.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Fetch for TokenContractInfo {
    type Request = GetTokenContractInfoRequest;
}
