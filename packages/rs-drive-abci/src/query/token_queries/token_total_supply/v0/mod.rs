use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
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
use drive::util::grove_operations::GroveDBToUse;

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
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use crate::query::tests::setup_platform_with_token_state;
    use dapi_grpc::platform::v0::get_token_total_supply_response::get_token_total_supply_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenTotalSupplyRequestV0 {
            token_id: vec![0; 8],
            prove: false,
        };

        let result = platform
            .query_token_total_supply_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_token_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenTotalSupplyRequestV0 {
            token_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_token_total_supply_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::NotFound(msg)] if msg.contains("Token")
        ));
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetTokenTotalSupplyRequestV0 {
            token_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_token_total_supply_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenTotalSupplyResponseV0 {
                result: Some(get_token_total_supply_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_token_data() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 0: base_supply=100000, minted 100 to id1 + 100 to id2 = 200 additional
        let request = GetTokenTotalSupplyRequestV0 {
            token_id: token_ids[0].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_total_supply_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_total_supply_response_v0::Result::TokenTotalSupply(entry)) => {
                assert_eq!(entry.token_id, token_ids[0].to_vec());
                // Total supply = base_supply (100000) + minted (100 + 100) = 100200
                assert_eq!(entry.total_system_amount, 100200);
                // Aggregated identity balances = base_supply + minted = 100100 + 100 = 100200
                assert_eq!(entry.total_aggregated_amount_in_user_accounts, 100200);
            }
            _ => panic!("expected TokenTotalSupply result"),
        }
    }

    #[test]
    fn test_query_with_token_data_token_1() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        // Token 1: base_supply=100000, minted 200 to id1 + 100 to id3 = 300 additional
        let request = GetTokenTotalSupplyRequestV0 {
            token_id: token_ids[1].to_vec(),
            prove: false,
        };

        let result = platform
            .query_token_total_supply_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_token_total_supply_response_v0::Result::TokenTotalSupply(entry)) => {
                // Total supply = base_supply (100000) + minted (200 + 100) = 100300
                assert_eq!(entry.total_system_amount, 100300);
                // Aggregated identity balances = 100200 + 100 = 100300
                assert_eq!(entry.total_aggregated_amount_in_user_accounts, 100300);
            }
            _ => panic!("expected TokenTotalSupply result"),
        }
    }

    #[test]
    fn test_query_with_data_proof() {
        let (platform, state, version, _, token_ids, _) = setup_platform_with_token_state();

        let request = GetTokenTotalSupplyRequestV0 {
            token_id: token_ids[0].to_vec(),
            prove: true,
        };

        let result = platform
            .query_token_total_supply_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetTokenTotalSupplyResponseV0 {
                result: Some(get_token_total_supply_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
