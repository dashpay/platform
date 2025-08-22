use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
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

            GetGroupInfosResponseV0 {
                result: Some(get_group_infos_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
