use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_token_total_supply_request::GetTokenTotalSupplyRequestV0;
use dapi_grpc::platform::v0::get_token_total_supply_response::get_token_total_supply_response_v0::TokenTotalSupplyEntry;
use dapi_grpc::platform::v0::get_token_total_supply_response::{
    get_token_total_supply_response_v0, GetTokenTotalSupplyResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_token_total_supply_v0(
        &self,
        GetTokenTotalSupplyRequestV0 { token_id, prove }: GetTokenTotalSupplyRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetTokenTotalSupplyResponseV0>, Error> {
        let token_id: [u8; 32] =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_token_total_supply_and_aggregated_identity_balances(
                    token_id,
                    None,
                    platform_version,
                ));

            GetTokenTotalSupplyResponseV0 {
                result: Some(get_token_total_supply_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let Some(token_total_aggregated_identity_balances) = self
                .drive
                .fetch_token_total_aggregated_identity_balances(token_id, None, platform_version)?
            else {
                return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                    format!("Token {} not found", Identifier::new(token_id)),
                )));
            };

            let Some(token_total_supply) =
                self.drive
                    .fetch_token_total_supply(token_id, None, platform_version)?
            else {
                return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                    format!("Token {} total supply not found", Identifier::new(token_id)),
                )));
            };

            GetTokenTotalSupplyResponseV0 {
                result: Some(
                    get_token_total_supply_response_v0::Result::TokenTotalSupply(
                        TokenTotalSupplyEntry {
                            token_id: token_id.to_vec(),
                            total_aggregated_amount_in_user_accounts:
                                token_total_aggregated_identity_balances,
                            total_system_amount: token_total_supply,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
