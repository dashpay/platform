use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_group_info_request::GetGroupInfoRequestV0;
use dapi_grpc::platform::v0::get_group_info_response::get_group_info_response_v0::{
    GroupInfo, GroupInfoEntry, GroupMemberEntry,
};
use dapi_grpc::platform::v0::get_group_info_response::{
    get_group_info_response_v0, GetGroupInfoResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_group_info_v0(
        &self,
        GetGroupInfoRequestV0 {
            contract_id,
            group_contract_position,
            prove,
        }: GetGroupInfoRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetGroupInfoResponseV0>, Error> {
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract id must be a valid identifier (32 bytes long)".to_string(),
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

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_group_info(
                contract_id,
                group_contract_position as u16,
                None,
                platform_version,
            ));

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetGroupInfoResponseV0 {
                result: Some(get_group_info_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
            }
        } else {
            let group_info = self
                .drive
                .fetch_group_info(
                    contract_id,
                    group_contract_position as u16,
                    None,
                    platform_version,
                )?
                .map(|group| {
                    let members = group
                        .members()
                        .iter()
                        .map(|(member_id, power)| GroupMemberEntry {
                            member_id: member_id.to_vec(),
                            power: *power,
                        })
                        .collect();
                    GroupInfoEntry {
                        members,
                        group_required_power: group.required_power(),
                    }
                });

            GetGroupInfoResponseV0 {
                result: Some(get_group_info_response_v0::Result::GroupInfo(GroupInfo {
                    group_info,
                })),
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
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![0; 8],
            group_contract_position: 0,
            prove: false,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_group_contract_position_over_u16_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32 + 1,
            prove: false,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }

    #[test]
    fn test_query_group_info_no_prove_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            prove: false,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupInfoResponseV0 {
                result: Some(get_group_info_response_v0::Result::GroupInfo(GroupInfo {
                    group_info: None,
                })),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_group_info_prove_returns_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: 0,
            prove: true,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(result.errors.is_empty());
        assert!(matches!(
            result.data,
            Some(GetGroupInfoResponseV0 {
                result: Some(get_group_info_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_group_contract_position_at_u16_max_is_valid() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32,
            prove: false,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        // Should not have a validation error for position being over u16::MAX
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_contract_id_empty() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![],
            group_contract_position: 0,
            prove: false,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_group_contract_position_over_u16_max_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetGroupInfoRequestV0 {
            contract_id: vec![0; 32],
            group_contract_position: u16::MAX as u32 + 1,
            prove: true,
        };

        let result = platform
            .query_group_info_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))]
                if msg.contains("can not be over u16::MAX")
        ));
    }
}
