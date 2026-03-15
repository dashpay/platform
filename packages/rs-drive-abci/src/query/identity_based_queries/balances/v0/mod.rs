use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_response::{
    get_identities_balances_response_v0, GetIdentitiesBalancesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_identities_balances_v0(
        &self,
        GetIdentitiesBalancesRequestV0 { ids, prove }: GetIdentitiesBalancesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesBalancesResponseV0>, Error> {
        let identifiers = check_validation_result_with_data!(ids
            .into_iter()
            .map(|identity_id| {
                let identifier: Identifier = identity_id.try_into().map_err(|_| {
                    QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string(),
                    )
                })?;
                Ok(identifier.into_buffer())
            })
            .collect::<Result<Vec<_>, QueryError>>());
        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_many_identity_balances(
                    identifiers.as_slice(),
                    None,
                    &platform_version.drive
                ));

            GetIdentitiesBalancesResponseV0 {
                result: Some(get_identities_balances_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let map = |(key, value): ([u8; 32], Option<u64>)| {
                get_identities_balances_response_v0::IdentityBalance {
                    identity_id: key.to_vec(),
                    balance: value,
                }
            };

            let identities_balances = self
                .drive
                .fetch_optional_identities_balances(&identifiers, None, platform_version)?
                .into_iter()
                .map(map)
                .collect();

            GetIdentitiesBalancesResponseV0 {
                result: Some(
                    get_identities_balances_response_v0::Result::IdentitiesBalances(
                        get_identities_balances_response_v0::IdentitiesBalances {
                            entries: identities_balances,
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
    use dapi_grpc::platform::v0::get_identities_balances_response::get_identities_balances_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_identity_id_in_list() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![vec![0; 8]], // invalid: 8 bytes instead of 32
            prove: false,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("id must be a valid identifier (32 bytes long)")
        ));
    }

    #[test]
    fn test_empty_identifiers_list() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![],
            prove: false,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_identities_balances_response_v0::Result::IdentitiesBalances(balances)) => {
                assert!(balances.entries.is_empty());
            }
            other => panic!("expected IdentitiesBalances, got {:?}", other),
        }
    }

    #[test]
    fn test_identity_not_found_returns_none_balance() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id = vec![0; 32];
        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![id.clone()],
            prove: false,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_identities_balances_response_v0::Result::IdentitiesBalances(balances)) => {
                assert_eq!(balances.entries.len(), 1);
                assert_eq!(balances.entries[0].identity_id, id);
                assert_eq!(balances.entries[0].balance, None);
            }
            other => panic!("expected IdentitiesBalances, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_identities_not_found_returns_none_balances() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let id1 = vec![1; 32];
        let id2 = vec![2; 32];
        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![id1.clone(), id2.clone()],
            prove: false,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        let response = result.data.expect("expected response data");
        match response.result {
            Some(get_identities_balances_response_v0::Result::IdentitiesBalances(balances)) => {
                assert_eq!(balances.entries.len(), 2);
                for entry in &balances.entries {
                    assert!(entry.identity_id == id1 || entry.identity_id == id2);
                    assert_eq!(entry.balance, None);
                }
            }
            other => panic!("expected IdentitiesBalances, got {:?}", other),
        }
    }

    #[test]
    fn test_identities_balances_absence_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        assert!(matches!(
            result.data,
            Some(GetIdentitiesBalancesResponseV0 {
                result: Some(get_identities_balances_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_second_identifier_invalid_in_list() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![vec![0; 32], vec![1; 8]], // first valid, second invalid
            prove: false,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("id must be a valid identifier (32 bytes long)")
        ));
    }

    #[test]
    fn test_empty_identifiers_list_with_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesBalancesRequestV0 {
            ids: vec![],
            prove: true,
        };

        let result = platform
            .query_identities_balances_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.is_valid());

        assert!(matches!(
            result.data,
            Some(GetIdentitiesBalancesResponseV0 {
                result: Some(get_identities_balances_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
