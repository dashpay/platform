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

            GetGroupActionSignersResponseV0 {
                result: Some(get_group_action_signers_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
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
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
