use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
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

            GetGroupInfoResponseV0 {
                result: Some(get_group_info_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
