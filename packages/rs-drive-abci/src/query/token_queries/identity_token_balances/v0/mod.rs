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
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

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
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
    use dapi_grpc::platform::v0::get_identity_token_balances_response::get_identity_token_balances_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenBalancesRequestV0 {
            identity_id: vec![0; 8],
            token_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identity_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenBalancesRequestV0 {
            identity_id: vec![0; 32],
            token_ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_identity_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenBalancesRequestV0 {
            identity_id: vec![0; 32],
            token_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identity_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identity_token_balances_response_v0::Result::TokenBalances(balances)) => {
                assert_eq!(balances.token_balances.len(), 1);
                assert!(balances.token_balances[0].balance.is_none());
            }
            _ => panic!("expected TokenBalances result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityTokenBalancesRequestV0 {
            identity_id: vec![0; 32],
            token_ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_identity_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentityTokenBalancesResponseV0 {
                result: Some(get_identity_token_balances_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_with_token_data() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        // Identity 1 has: token_0=100, token_1=200, token_2=300
        let request = GetIdentityTokenBalancesRequestV0 {
            identity_id: identity_ids[0].to_vec(),
            token_ids: vec![
                token_ids[0].to_vec(),
                token_ids[1].to_vec(),
                token_ids[2].to_vec(),
            ],
            prove: false,
        };

        let result = platform
            .query_identity_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identity_token_balances_response_v0::Result::TokenBalances(balances)) => {
                assert_eq!(balances.token_balances.len(), 3);
                let mut total = 0u64;
                for entry in &balances.token_balances {
                    total += entry.balance.unwrap_or(0);
                }
                // Identity 1 balances include base_supply (100000) per token:
                // token_0: 100000 + 100 = 100100
                // token_1: 100000 + 200 = 100200
                // token_2: 100000 + 300 = 100300
                // Total = 300600
                assert_eq!(total, 300600);
            }
            _ => panic!("expected TokenBalances result"),
        }
    }

    #[test]
    fn test_query_with_token_data_proof() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentityTokenBalancesRequestV0 {
            identity_id: identity_ids[0].to_vec(),
            token_ids: vec![token_ids[0].to_vec()],
            prove: true,
        };

        let result = platform
            .query_identity_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentityTokenBalancesResponseV0 {
                result: Some(get_identity_token_balances_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
