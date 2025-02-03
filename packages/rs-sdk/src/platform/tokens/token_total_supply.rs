use crate::platform::{Fetch, Identifier, Query};
use crate::Error;
use dapi_grpc::platform::v0::get_token_total_supply_request::GetTokenTotalSupplyRequestV0;
use dapi_grpc::platform::v0::{get_token_total_supply_request, GetTokenTotalSupplyRequest};
pub use dpp::balances::total_single_token_balance::TotalSingleTokenBalance;

impl Query<GetTokenTotalSupplyRequest> for Identifier {
    fn query(self, prove: bool) -> Result<GetTokenTotalSupplyRequest, Error> {
        let request = GetTokenTotalSupplyRequest {
            version: Some(get_token_total_supply_request::Version::V0(
                GetTokenTotalSupplyRequestV0 {
                    token_id: self.to_vec(),
                    prove,
                },
            )),
        };

        Ok(request)
    }
}

impl Fetch for TotalSingleTokenBalance {
    type Request = GetTokenTotalSupplyRequest;
}
