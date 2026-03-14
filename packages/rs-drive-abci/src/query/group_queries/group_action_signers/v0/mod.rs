use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_group_action_signers_request::GetGroupActionSignersRequestV0;
use dapi_grpc::platform::v0::get_group_action_signers_response::{
    get_group_action_signers_response_v0, GetGroupActionSignersResponseV0,
};
use dapi_grpc::platform::v0::get_group_action_signers_response::get_group_action_signers_response_v0::{GroupActionSigner, GroupActionSigners};
use dpp::check_validation_result_with_data;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

impl<C> Platform<C> {
    pub(super) fn query_group_action_signers_v0(
        &self,
        GetGroupActionSignersRequestV0 {
            contract_id,
            group_contract_position,
            status,
            action_id,
            prove,
        }: GetGroupActionSignersRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetGroupActionSignersResponseV0>, Error> {
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let action_id: Identifier =
            check_validation_result_with_data!(action_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "action id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        if group_contract_position > u16::MAX as u32 {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidParameter(format!(
                    "group contract position {} can not be over u16::MAX",
                    group_contract_position
                )),
            )));
        }

        let group_status: GroupActionStatus =
            check_validation_result_with_data!(status.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "group action status must be Active or Closed".to_string(),
                )
            }));

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_action_signers(
                contract_id,
                group_contract_position as GroupContractPosition,
                group_status,
                action_id,
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetGroupActionSignersResponseV0 {
                result: Some(get_group_action_signers_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let group_action_signers = self
                .drive
                .fetch_action_signers(
                    contract_id,
                    group_contract_position as GroupContractPosition,
                    group_status,
                    action_id,
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(signer_id, power)| GroupActionSigner {
                    signer_id: signer_id.to_vec(),
                    power,
                })
                .collect();
            GetGroupActionSignersResponseV0 {
                result: Some(
                    get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners {
                            signers: group_action_signers,
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
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::group::action_event::GroupActionEvent as DppGroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::identifier::Identifier;
    use dpp::tokens::token_event::TokenEvent;
    use std::collections::BTreeMap;

    #[test]
    fn test_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 8], // invalid: must be 32 bytes
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_when_prove_is_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 8],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_action_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 8], // invalid: must be 32 bytes
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("action id must be a valid identifier")
        ));
    }

    #[test]
    fn test_empty_action_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            action_id: vec![],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("action id must be a valid identifier")
        ));
    }

    #[test]
    fn test_group_contract_position_exceeds_u16_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32 + 1,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }

    #[test]
    fn test_invalid_status() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 99, // invalid status
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg.contains("group action status must be Active or Closed")
        ));
    }

    #[test]
    fn test_query_group_action_signers_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0, // Active
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(
                    get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners { signers }
                    )
                ),
                metadata: Some(_),
            }) if signers.is_empty()
        ));
    }

    #[test]
    fn test_query_group_action_signers_with_prove_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 32],
            prove: true,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(get_group_action_signers_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_action_signers_with_closed_status() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            status: 1, // Closed
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(
                    get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners { signers }
                    )
                ),
                metadata: Some(_),
            }) if signers.is_empty()
        ));
    }

    #[test]
    fn test_empty_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![],
            group_contract_position: 0,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_group_contract_position_at_u16_max_boundary() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Exactly u16::MAX should be valid (no error about position)
        let request = GetGroupActionSignersRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32,
            status: 0,
            action_id: vec![0; 32],
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        // Should not have a validation error about position
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_query_group_action_signers_with_populated_action() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let contract_id = Identifier::from([1u8; 32]);
        let member_id = Identifier::from([2u8; 32]);
        let action_id = Identifier::from([3u8; 32]);
        let recipient_id = Identifier::from([4u8; 32]);

        // Create the group structure
        let mut members = BTreeMap::new();
        members.insert(member_id, 5u32);

        let group = Group::V0(GroupV0 {
            members,
            required_power: 10,
        });

        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        platform
            .drive
            .add_new_groups(
                contract_id,
                &groups,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to add groups");

        // Create a Mint action with a signer
        let mint_action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: member_id,
            token_contract_position: 0,
            event: DppGroupActionEvent::TokenEvent(TokenEvent::Mint(
                1000,
                recipient_id,
                Some("test mint".to_string()),
            )),
        });

        platform
            .drive
            .add_group_action(
                contract_id,
                0,
                Some(mint_action),
                false, // not closing
                action_id,
                member_id,
                5,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to add group action");

        let request = GetGroupActionSignersRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0, // Active
            action_id: action_id.to_vec(),
            prove: false,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        match result.data {
            Some(GetGroupActionSignersResponseV0 {
                result:
                    Some(get_group_action_signers_response_v0::Result::GroupActionSigners(
                        GroupActionSigners { signers },
                    )),
                metadata: Some(_),
            }) => {
                assert_eq!(signers.len(), 1);
                assert_eq!(signers[0].signer_id, member_id.to_vec());
                assert_eq!(signers[0].power, 5);
            }
            other => panic!("unexpected result: {:?}", other),
        }
    }

    #[test]
    fn test_query_group_action_signers_with_populated_action_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let contract_id = Identifier::from([1u8; 32]);
        let member_id = Identifier::from([2u8; 32]);
        let action_id = Identifier::from([3u8; 32]);
        let recipient_id = Identifier::from([4u8; 32]);

        let mut members = BTreeMap::new();
        members.insert(member_id, 5u32);

        let group = Group::V0(GroupV0 {
            members,
            required_power: 10,
        });

        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        platform
            .drive
            .add_new_groups(
                contract_id,
                &groups,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to add groups");

        let mint_action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: member_id,
            token_contract_position: 0,
            event: DppGroupActionEvent::TokenEvent(TokenEvent::Mint(
                1000,
                recipient_id,
                Some("test mint".to_string()),
            )),
        });

        platform
            .drive
            .add_group_action(
                contract_id,
                0,
                Some(mint_action),
                false,
                action_id,
                member_id,
                5,
                &BlockInfo::genesis(),
                true,
                None,
                version,
            )
            .expect("expected to add group action");

        let request = GetGroupActionSignersRequestV0 {
            contract_id: contract_id.to_vec(),
            group_contract_position: 0,
            status: 0,
            action_id: action_id.to_vec(),
            prove: true,
        };

        let result = platform
            .query_group_action_signers_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupActionSignersResponseV0 {
                result: Some(get_group_action_signers_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }
}
