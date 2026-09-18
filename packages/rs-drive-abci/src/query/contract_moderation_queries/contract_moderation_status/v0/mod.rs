use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::{identifier_from_request, list_from_request};
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_moderation_status_request::GetContractModerationStatusRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_status_response::{
    get_contract_moderation_status_response_v0,
    ContractModerationStatus as ContractModerationStatusProto,
    GetContractModerationStatusResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns an identity's status on the lists of a moderated contract the request names. Each
    /// list must be one the contract keeps. The proved form proves the identity's entry, present
    /// or absent, on each of them.
    pub(super) fn query_contract_moderation_status_v0(
        &self,
        GetContractModerationStatusRequestV0 {
            contract_id,
            identity_id,
            lists,
            prove,
        }: GetContractModerationStatusRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractModerationStatusResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let identity_id =
            check_validation_result_with_data!(identifier_from_request(identity_id, "identity_id"));

        let mut requested: Vec<ContractModerationList> = Vec::with_capacity(lists.len());
        for list in lists {
            let list = check_validation_result_with_data!(list_from_request(list, "lists"));
            if requested.contains(&list) {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!("lists names {} twice", list)),
                ));
            }
            requested.push(list);
        }
        if requested.is_empty() {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument("lists must name at least one list".to_string()),
            ));
        }

        let kept = check_validation_result_with_data!(
            self.kept_moderation_lists(contract_id, platform_version)?
        );
        for list in &requested {
            if !kept.contains(list) {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument(format!(
                        "contract {} does not keep a {}",
                        contract_id, list
                    )),
                ));
            }
        }

        let response = if prove {
            let proof =
                check_validation_result_with_data!(self.drive.prove_contract_moderation_status(
                    contract_id,
                    identity_id,
                    &requested,
                    None,
                    platform_version
                ));

            GetContractModerationStatusResponseV0 {
                result: Some(get_contract_moderation_status_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let status =
                check_validation_result_with_data!(self.drive.fetch_contract_moderation_status(
                    contract_id,
                    identity_id,
                    &requested,
                    None,
                    platform_version
                ));

            GetContractModerationStatusResponseV0 {
                result: Some(get_contract_moderation_status_response_v0::Result::Status(
                    ContractModerationStatusProto {
                        banned: status.banned,
                        suspended_until: status.suspended_until,
                    },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
