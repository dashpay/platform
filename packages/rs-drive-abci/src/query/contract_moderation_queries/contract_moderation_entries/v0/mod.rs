use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::contract_moderation_queries::{identifier_from_request, list_from_request};
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contract_moderation_entries_request::GetContractModerationEntriesRequestV0;
use dapi_grpc::platform::v0::get_contract_moderation_entries_response::{
    get_contract_moderation_entries_response_v0, ContractModerationEntries,
    ContractModerationEntry as ContractModerationEntryProto,
    GetContractModerationEntriesResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::contract::moderation::types::ContractModerationEntriesQuery;
use drive::util::grove_operations::GroveDBToUse;

/// The page size when the request names none.
const DEFAULT_LIMIT: u16 = 100;

impl<C> Platform<C> {
    /// Returns one page of a moderated contract's banlist or suspension list, in identity id
    /// order. The list must be one the contract keeps.
    pub(super) fn query_contract_moderation_entries_v0(
        &self,
        GetContractModerationEntriesRequestV0 {
            contract_id,
            list,
            start_after,
            limit,
            prove,
        }: GetContractModerationEntriesRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContractModerationEntriesResponseV0>, Error> {
        let contract_id =
            check_validation_result_with_data!(identifier_from_request(contract_id, "contract_id"));
        let list = check_validation_result_with_data!(list_from_request(list, "list"));
        let start_after = check_validation_result_with_data!(start_after
            .map(|bytes| identifier_from_request(bytes, "start_after"))
            .transpose());
        let limit = match limit {
            None => DEFAULT_LIMIT,
            Some(limit) => check_validation_result_with_data!(u16::try_from(limit).map_err(|_| {
                QueryError::InvalidArgument(format!("limit {limit} is out of bounds"))
            })),
        };

        let kept = check_validation_result_with_data!(
            self.kept_moderation_lists(contract_id, platform_version)?
        );
        if !kept.contains(&list) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!(
                    "contract {} does not keep a {}",
                    contract_id, list
                )),
            ));
        }

        let query = ContractModerationEntriesQuery {
            list,
            start_after,
            limit,
        };

        let response = if prove {
            let proof = check_validation_result_with_data!(self
                .drive
                .prove_contract_moderation_entries(contract_id, &query, None, platform_version));

            GetContractModerationEntriesResponseV0 {
                result: Some(get_contract_moderation_entries_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let entries = check_validation_result_with_data!(self
                .drive
                .fetch_contract_moderation_entries(contract_id, &query, None, platform_version));

            GetContractModerationEntriesResponseV0 {
                result: Some(
                    get_contract_moderation_entries_response_v0::Result::Entries(
                        ContractModerationEntries {
                            entries: entries
                                .into_iter()
                                .map(|entry| ContractModerationEntryProto {
                                    identity_id: entry.identity_id.to_vec(),
                                    until: entry.until,
                                })
                                .collect(),
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
