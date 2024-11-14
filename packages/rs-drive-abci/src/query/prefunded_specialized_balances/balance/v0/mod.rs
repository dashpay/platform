use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_prefunded_specialized_balance_request::GetPrefundedSpecializedBalanceRequestV0;
use dapi_grpc::platform::v0::get_prefunded_specialized_balance_response::get_prefunded_specialized_balance_response_v0;
use dapi_grpc::platform::v0::get_prefunded_specialized_balance_response::GetPrefundedSpecializedBalanceResponseV0;
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_prefunded_specialized_balance_v0(
        &self,
        GetPrefundedSpecializedBalanceRequestV0 { id, prove }: GetPrefundedSpecializedBalanceRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetPrefundedSpecializedBalanceResponseV0>, Error> {
        let balance_id: Identifier =
            check_validation_result_with_data!(id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_prefunded_specialized_balance(
                    balance_id.into_buffer(),
                    None,
                    platform_version,
                ));

            GetPrefundedSpecializedBalanceResponseV0 {
                result: Some(
                    get_prefunded_specialized_balance_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let maybe_balance = self.drive.fetch_prefunded_specialized_balance(
                balance_id.into_buffer(),
                None,
                platform_version,
            )?;

            let Some(balance) = maybe_balance else {
                return Ok(ValidationResult::new_with_error(QueryError::NotFound(
                    "No Identity found".to_string(),
                )));
            };

            GetPrefundedSpecializedBalanceResponseV0 {
                result: Some(
                    get_prefunded_specialized_balance_response_v0::Result::Balance(balance),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPrefundedSpecializedBalanceRequestV0 {
            id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_prefunded_specialized_balance_v0(request, &state, version)
            .expect("should query balance");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_identity_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];

        let request = GetPrefundedSpecializedBalanceRequestV0 {
            id: id.clone(),
            prove: false,
        };

        let result = platform
            .query_prefunded_specialized_balance_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(_)]
        ));
    }

    #[test]
    fn test_identity_balance_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];

        let request = GetPrefundedSpecializedBalanceRequestV0 {
            id: id.clone(),
            prove: true,
        };

        let result = platform
            .query_prefunded_specialized_balance_v0(request, &state, version)
            .expect("should query balance");

        assert!(matches!(
            result.data,
            Some(GetPrefundedSpecializedBalanceResponseV0 {
                result: Some(get_prefunded_specialized_balance_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ));
    }
}
