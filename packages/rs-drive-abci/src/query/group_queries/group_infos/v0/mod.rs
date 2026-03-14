use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_group_infos_request::GetGroupInfosRequestV0;
use dapi_grpc::platform::v0::get_group_infos_response::get_group_infos_response_v0::{
    GroupInfos, GroupMemberEntry, GroupPositionInfoEntry,
};
use dapi_grpc::platform::v0::get_group_infos_response::{
    get_group_infos_response_v0, GetGroupInfosResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_group_infos_v0(
        &self,
        GetGroupInfosRequestV0 {
            contract_id,
            start_at_group_contract_position,
            count,
            prove,
        }: GetGroupInfosRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetGroupInfosResponseV0>, Error> {
        let config = &self.config.drive;
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let limit = count
            .map_or(Some(config.default_query_limit), |limit_value| {
                if limit_value == 0
                    || limit_value > u16::MAX as u32
                    || limit_value as u16 > config.default_query_limit
                {
                    None
                } else {
                    Some(limit_value as u16)
                }
            })
            .ok_or(drive::error::Error::Query(QuerySyntaxError::InvalidLimit(
                format!("limit greater than max limit {}", config.max_query_limit),
            )))?;

        let start_at_group_contract_position = match start_at_group_contract_position {
            None => None,
            Some(start_at_group_contract_position) => {
                if start_at_group_contract_position.start_group_contract_position > u16::MAX as u32
                {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidParameter(format!(
                            "start group contract position {} can not be over u16::MAX",
                            start_at_group_contract_position.start_group_contract_position
                        )),
                    )));
                }
                Some((
                    start_at_group_contract_position.start_group_contract_position as u16,
                    start_at_group_contract_position.start_group_contract_position_included,
                ))
            }
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_group_infos(
                contract_id,
                start_at_group_contract_position,
                Some(limit),
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let group_infos = self
                .drive
                .fetch_group_infos(
                    contract_id,
                    start_at_group_contract_position,
                    Some(limit),
                    None,
                    platform_version,
                )?
                .into_iter()
                .map(|(group_contract_position, group)| {
                    let members = group
                        .members()
                        .iter()
                        .map(|(member_id, power)| GroupMemberEntry {
                            member_id: member_id.to_vec(),
                            power: *power,
                        })
                        .collect();
                    GroupPositionInfoEntry {
                        group_contract_position: group_contract_position as u32,
                        members,
                        group_required_power: group.required_power(),
                    }
                })
                .collect();

            GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::GroupInfos(
                    GroupInfos { group_infos },
                )),
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
    use dapi_grpc::platform::v0::get_group_infos_request::StartAtGroupContractPosition;
    use dpp::block::block_info::BlockInfo;
    use dpp::dashcore::Network;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    #[allow(unused_imports)]
    use dpp::identifier::Identifier;
    use std::collections::BTreeMap;

    #[test]
    fn test_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 8],
            start_at_group_contract_position: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_when_prove_is_true() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 8],
            start_at_group_contract_position: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_limit_zero() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: None,
            count: Some(0),
            prove: false,
        };

        let result = platform.query_group_infos_v0(request, &state, version);

        assert!(result.is_err(), "expected an error for zero limit");
    }

    #[test]
    fn test_invalid_limit_exceeds_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: None,
            count: Some(u16::MAX as u32 + 1),
            prove: false,
        };

        let result = platform.query_group_infos_v0(request, &state, version);

        assert!(result.is_err(), "expected an error for limit exceeding max");
    }

    #[test]
    fn test_start_group_contract_position_exceeds_u16_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: Some(StartAtGroupContractPosition {
                start_group_contract_position: u16::MAX as u32 + 1,
                start_group_contract_position_included: true,
            }),
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }

    #[test]
    fn test_query_group_infos_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::GroupInfos(GroupInfos {
                    group_infos,
                })),
                metadata: Some(_),
            }) if group_infos.is_empty()
        ));
    }

    #[test]
    fn test_query_group_infos_with_prove_empty_state() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_infos_with_valid_start_position() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: Some(StartAtGroupContractPosition {
                start_group_contract_position: 0,
                start_group_contract_position_included: true,
            }),
            count: Some(10),
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::GroupInfos(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_infos_with_valid_count() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: None,
            count: Some(5),
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_empty_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![],
            start_at_group_contract_position: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_query_group_infos_with_populated_groups_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let contract_id = Identifier::from([1u8; 32]);
        let member_id = Identifier::from([2u8; 32]);

        let mut members = BTreeMap::new();
        members.insert(member_id, 5u32);

        let group = Group::V0(GroupV0 {
            members,
            required_power: 5,
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

        let request = GetGroupInfosRequestV0 {
            contract_id: contract_id.to_vec(),
            start_at_group_contract_position: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_infos_with_start_position_not_included() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfosRequestV0 {
            contract_id: vec![0; 32],
            start_at_group_contract_position: Some(StartAtGroupContractPosition {
                start_group_contract_position: 0,
                start_group_contract_position_included: false,
            }),
            count: Some(10),
            prove: false,
        };

        let result = platform
            .query_group_infos_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
    }
}
