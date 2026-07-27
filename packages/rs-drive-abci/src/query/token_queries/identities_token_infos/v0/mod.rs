use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_token_infos_request::GetIdentitiesTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_identities_token_infos_response::{get_identities_token_infos_response_v0, GetIdentitiesTokenInfosResponseV0};
use dapi_grpc::platform::v0::get_identities_token_infos_response::get_identities_token_infos_response_v0::{IdentityTokenInfos, TokenIdentityInfoEntry, TokenInfoEntry};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_identities_token_infos_v0(
        &self,
        GetIdentitiesTokenInfosRequestV0 {
            token_id,
            identity_ids,
            prove,
        }: GetIdentitiesTokenInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesTokenInfosResponseV0>, Error> {
        if identity_ids.len() > platform_version.drive_abci.query.max_returned_elements as usize {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidLimit(format!(
                    "trying to get {} identities token infos, maximum is {}",
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
                check_validation_result_with_data!(self.drive.prove_identities_token_infos(
                    token_id.into_buffer(),
                    identity_ids.as_slice(),
                    None,
                    platform_version,
                ));

            GetIdentitiesTokenInfosResponseV0 {
                result: Some(get_identities_token_infos_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let identity_token_infos = self
                .drive
                .fetch_identities_token_infos(
                    token_id.into_buffer(),
                    identity_ids.as_slice(),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(identity_id, info)| {
                    let info = info.map(|identity_token_info| TokenIdentityInfoEntry {
                        frozen: identity_token_info.frozen(),
                    });
                    TokenInfoEntry {
                        identity_id: identity_id.to_vec(),
                        info,
                    }
                })
                .collect();

            GetIdentitiesTokenInfosResponseV0 {
                result: Some(
                    get_identities_token_infos_response_v0::Result::IdentityTokenInfos(
                        IdentityTokenInfos {
                            token_infos: identity_token_infos,
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
    use dapi_grpc::platform::v0::get_identities_token_infos_response::get_identities_token_infos_response_v0;
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_token_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 8],
            identity_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_query_with_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(result.data.is_some());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_infos_response_v0::Result::IdentityTokenInfos(infos)) => {
                assert_eq!(infos.token_infos.len(), 1);
                // Info should be None (no token exists)
                assert!(infos.token_infos[0].info.is_none());
            }
            _ => panic!("expected IdentityTokenInfos result"),
        }
    }

    #[test]
    fn test_query_with_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![0; 32]],
            prove: true,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesTokenInfosResponseV0 {
                result: Some(get_identities_token_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_identity_ids_exceeding_max_limit_is_rejected() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as usize;

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: (0..=max).map(|i| vec![i as u8; 32]).collect(),
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
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

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: (0..max).map(|i| vec![i as u8; 32]).collect(),
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
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
    fn test_query_with_token_data_not_frozen() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        // Identity 1 is NOT frozen on token 0 - no explicit info record is stored
        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![identity_ids[0].to_vec()],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_infos_response_v0::Result::IdentityTokenInfos(infos)) => {
                assert_eq!(infos.token_infos.len(), 1);
                // Unfrozen identities have no info record, so info is None
                assert!(infos.token_infos[0].info.is_none());
            }
            _ => panic!("expected IdentityTokenInfos result"),
        }
    }

    #[test]
    fn test_query_with_token_data_frozen() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        // Identity 2 IS frozen on token 0
        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![identity_ids[1].to_vec()],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_infos_response_v0::Result::IdentityTokenInfos(infos)) => {
                assert_eq!(infos.token_infos.len(), 1);
                let info = infos.token_infos[0].info.as_ref().expect("expected info");
                assert!(info.frozen);
            }
            _ => panic!("expected IdentityTokenInfos result"),
        }
    }

    #[test]
    fn test_query_with_token_data_proof() {
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![identity_ids[0].to_vec()],
            prove: true,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesTokenInfosResponseV0 {
                result: Some(get_identities_token_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_invalid_token_id_with_multiple_valid_identities() {
        // Invalid token_id fires first even when identity_ids are well-formed.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 31],
            identity_ids: vec![vec![1; 32], vec![2; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_one_invalid_identity_in_list_rejected() {
        // A mixed list with one malformed identity fails the entire call.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![vec![1; 32], vec![2; 5], vec![3; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("identity_id")
        ));
    }

    #[test]
    fn test_empty_identity_ids_list() {
        // Empty list is accepted (len 0 does not exceed max) and yields empty
        // token_infos.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: vec![],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_infos_response_v0::Result::IdentityTokenInfos(infos)) => {
                assert!(infos.token_infos.is_empty());
            }
            _ => panic!("expected IdentityTokenInfos result"),
        }
    }

    #[test]
    fn test_query_mixed_frozen_and_unfrozen_identities() {
        // Query multiple identities, one frozen (id 2), one unfrozen (id 1).
        // Both entries should appear; the frozen one has Some(info) with
        // frozen=true, the unfrozen one has info=None.
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![identity_ids[0].to_vec(), identity_ids[1].to_vec()],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let data = result.data.unwrap();
        match data.result {
            Some(get_identities_token_infos_response_v0::Result::IdentityTokenInfos(infos)) => {
                assert_eq!(infos.token_infos.len(), 2);
                let mut frozen_found = false;
                let mut none_found = false;
                for entry in &infos.token_infos {
                    if entry.identity_id == identity_ids[0].to_vec() {
                        assert!(
                            entry.info.is_none(),
                            "unfrozen identity should have None info"
                        );
                        none_found = true;
                    } else if entry.identity_id == identity_ids[1].to_vec() {
                        let info = entry
                            .info
                            .as_ref()
                            .expect("frozen identity should have info");
                        assert!(info.frozen);
                        frozen_found = true;
                    }
                }
                assert!(frozen_found && none_found);
            }
            _ => panic!("expected IdentityTokenInfos result"),
        }
    }

    #[test]
    fn test_query_multiple_identities_proof() {
        // Prove path with multiple identities should return Proof variant.
        let (platform, state, version, _, token_ids, identity_ids) =
            setup_platform_with_token_state();

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: token_ids[0].to_vec(),
            identity_ids: vec![
                identity_ids[0].to_vec(),
                identity_ids[1].to_vec(),
                identity_ids[2].to_vec(),
            ],
            prove: true,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetIdentitiesTokenInfosResponseV0 {
                result: Some(get_identities_token_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_invalid_token_id_zero_length() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![],
            identity_ids: vec![vec![0; 32]],
            prove: false,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("token_id")
        ));
    }

    #[test]
    fn test_identity_ids_at_max_limit_with_proof_still_proceeds() {
        // A full-size batch with prove=true should not hit the size guard.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let max = version.drive_abci.query.max_returned_elements as usize;

        let request = GetIdentitiesTokenInfosRequestV0 {
            token_id: vec![0; 32],
            identity_ids: (0..max).map(|i| vec![i as u8; 32]).collect(),
            prove: true,
        };

        let result = platform
            .query_identities_token_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(
            !result.errors.iter().any(|e| matches!(
                e,
                QueryError::Query(drive::error::query::QuerySyntaxError::InvalidLimit(_))
            )),
            "should not reject at exactly max: {:?}",
            result.errors
        );
    }
}
