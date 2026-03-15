use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_token_balances_request::GetIdentitiesTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_token_balances_response::{get_identities_token_balances_response_v0, GetIdentitiesTokenBalancesResponseV0};
use dapi_grpc::platform::v0::get_identities_token_balances_response::get_identities_token_balances_response_v0::{IdentityTokenBalanceEntry, IdentityTokenBalances};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_identities_token_balances_v0(
        &self,
        GetIdentitiesTokenBalancesRequestV0 {
            token_id,
            identity_ids,
            prove,
        }: GetIdentitiesTokenBalancesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesTokenBalancesResponseV0>, Error> {
        if identity_ids.len() > platform_version.drive_abci.query.max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get {} identities token balances, maximum is {}",
                    identity_ids.len(),
                    platform_version.drive_abci.query.max_returned_elements
                )),
            )));
        }
        let token_id: Identifier =
            check_validation_result_with_data!(token_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "token_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let identity_ids: Vec<[u8; 32]> = check_validation_result_with_data!(identity_ids
            .into_iter()
            .map(|identity_id| {
                identity_id.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "identity_id must be a valid identifier (32 bytes long)".to_string(),
                    )
                })
            })
            .collect::<Result<Vec<[u8; 32]>, QueryError>>());

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_identities_token_balances(
                    token_id.into_buffer(),
                    identity_ids.as_slice(),
                    None,
                    platform_version,
                ));

            GetIdentitiesTokenBalancesResponseV0 {
                result: Some(get_identities_token_balances_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let identity_token_balances = self
                .drive
                .fetch_identities_token_balances(
                    token_id.into_buffer(),
                    identity_ids.as_slice(),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(identity_id, amount)| IdentityTokenBalanceEntry {
                    identity_id: identity_id.to_vec(),
                    balance: amount,
                })
                .collect();

            GetIdentitiesTokenBalancesResponseV0 {
                result: Some(
                    get_identities_token_balances_response_v0::Result::IdentityTokenBalances(
                        IdentityTokenBalances {
                            identity_token_balances,
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
    use dapi_grpc::platform::v0::get_identities_token_balances_response::get_identities_token_balances_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: vec![0; 8], // invalid: not 32 bytes
            identity_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![0; 8]], // invalid
            prove: false,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(result.data.is_some());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_balances_response_v0::Result::IdentityTokenBalances(
                balances,
            )) => {
                assert_eq!(balances.identity_token_balances.len(), 1);
                // Balance should be None (no token exists)
                assert!(balances.identity_token_balances[0].balance.is_none());
            }
            _ => panic!("expected IdentityTokenBalances result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesTokenBalancesResponseV0 {
                result: Some(get_identities_token_balances_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_identity_ids_exceeding_max_limit_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as usize;

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: vec![0; 32],
            identity_ids: (0..=max).map(|i| vec![i as u8; 32]).collect(),
            prove: false,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                drive::error::query::QuerySyntaxError::InvalidLimit(_)
            )]
        ));
    }

    #[test]
    fn test_identity_ids_at_max_limit_is_accepted() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as usize;

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: vec![0; 32],
            identity_ids: (0..max).map(|i| vec![i as u8; 32]).collect(),
            prove: false,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            !result.errors.iter().any(|e| matches!(
                e,
                QueryError::Query(drive::error::query::QuerySyntaxError::InvalidLimit(_))
            )),
            "should not be rejected at exactly the max limit"
        );
    }

    #[test]
    fn test_query_with_token_data() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![identity_ids[0].to_vec(), identity_ids[1].to_vec()],
            prove: false,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_balances_response_v0::Result::IdentityTokenBalances(
                balances,
            )) => {
                assert_eq!(balances.identity_token_balances.len(), 2);
                // Identity 1 has base_supply (100000) + minted (100) = 100100
                // Identity 2 has minted (100)
                let mut found_balances: Vec<u64> = balances
                    .identity_token_balances
                    .iter()
                    .filter_map(|e| e.balance)
                    .collect();
                found_balances.sort();
                assert_eq!(found_balances, vec![100, 100100]);
            }
            _ => panic!("expected IdentityTokenBalances result"),
        }
    }

    #[test]
    fn test_query_with_token_data_proof() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentitiesTokenBalancesRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![identity_ids[0].to_vec()],
            prove: true,
        };

        let result = platform
            .query_identities_token_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesTokenBalancesResponseV0 {
                result: Some(get_identities_token_balances_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
