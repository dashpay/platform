use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_token_balances_request::GetIdentityTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identity_token_balances_response::{get_identity_token_balances_response_v0, GetIdentityTokenBalancesResponseV0};
use dapi_grpc::platform::v0::get_identity_token_balances_response::get_identity_token_balances_response_v0::{TokenBalanceEntry, TokenBalances};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_identity_token_balances_v0(
        &self,
        GetIdentityTokenBalancesRequestV0 {
            identity_id,
            token_ids,
            prove,
        }: GetIdentityTokenBalancesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityTokenBalancesResponseV0>, Error> {
        let identity_id: Identifier =
            check_validation_result_with_data!(identity_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "identity_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let token_ids: Vec<[u8; 32]> = check_validation_result_with_data!(token_ids
            .into_iter()
            .map(|token_id| {
                token_id.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "token_id must be a valid identifier (32 bytes long)".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_identity_token_balances(
                    token_ids.as_slice(),
                    identity_id.into_buffer(),
                    None,
                    platform_version,
                ));

            GetIdentityTokenBalancesResponseV0 {
                result: Some(get_identity_token_balances_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let token_balances = self
                .drive
                .fetch_identity_token_balances(
                    token_ids.as_slice(),
                    identity_id.into_buffer(),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(token_id, amount)| TokenBalanceEntry {
                    token_id: token_id.to_vec(),
                    balance: amount,
                })
                .collect();

            GetIdentityTokenBalancesResponseV0 {
                result: Some(
                    get_identity_token_balances_response_v0::Result::TokenBalances(TokenBalances {
                        token_balances,
                    }),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
