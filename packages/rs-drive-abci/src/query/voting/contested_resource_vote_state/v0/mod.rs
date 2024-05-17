use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::GetContestedResourceVoteStateResponseV0;
use dpp::check_validation_result_with_data;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::error::query::QuerySyntaxError;

impl<C> Platform<C> {
    pub(super) fn query_contested_resource_vote_status_v0(
        &self,
        GetContestedResourceVoteStateRequestV0 {
            contract_id,
            document_type_name,
            index_name,
            index_values,
            include_documents,
            start_at_identifier_info,
            prove,
        }: GetContestedResourceVoteStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourceVoteStateResponseV0>, Error> {
        let contract_id: Identifier =
            check_validation_result_with_data!(contract_id.try_into().map_err(|_| {
                QueryError::InvalidArgument(
                    "contract_id must be a valid identifier (32 bytes long)".to_string(),
                )
            }));

        let (_, contract) = self.drive.get_contract_with_fetch_info_and_fee(
            contract_id.to_buffer(),
            None,
            true,
            None,
            platform_version,
        )?;

        let contract = check_validation_result_with_data!(contract.ok_or(QueryError::Query(
            QuerySyntaxError::DataContractNotFound(
                "contract not found when querying from value with contract info",
            )
        )));

        let contract_ref = &contract.contract;

        let document_type = check_validation_result_with_data!(contract_ref
            .document_type_for_name(document_type_name.as_str())
            .map_err(|_| QueryError::InvalidArgument(format!(
                "document type {} not found for contract {}",
                document_type_name, contract_id
            ))));

        let index = check_validation_result_with_data!(document_type.find_contested_index().ok_or(
            QueryError::InvalidArgument(format!(
                "document type {} does not have a contested index",
                document_type_name
            ))
        ));

        if index.name != &index_name {
            return Ok(QueryValidationResult::new_with_error(QueryError::InvalidArgument(format!(
                "index with name {} is not the contested index on the document type {}, {} is the name of the only contested index",
                index_name, document_type_name, index.name
            ))));
        }

        let index_values = match index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                Ok(bincode::decode_from_slice(
                    serialized_value.as_slice(),
                    bincode::config::standard()
                        .with_big_endian()
                        .with_no_limit(),
                )
                .map_err(|_| {
                    QueryError::InvalidArgument(format!(
                        "could not convert {:?} to a value in the index values at position {}",
                        serialized_value, pos
                    ))
                })?
                .0)
            })
            .collect::<Result<Vec<_>, QueryError>>()
        {
            Ok(index_values) => index_values,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let vote_poll = ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        }
        .into();

        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_identity_balance(
                identity_id.into_buffer(),
                None,
                &platform_version.drive
            ));

            GetContestedResourceVoteStateResponseV0 {
                result: Some(get_identity_balance_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof),
                )),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let maybe_balance = self.drive.fetch_identity_balance(
                identity_id.into_buffer(),
                None,
                platform_version,
            )?;

            let Some(balance) = maybe_balance else {
                return Ok(ValidationResult::new_with_error(QueryError::NotFound(
                    "No Identity found".to_string(),
                )));
            };

            GetContestedResourceVoteStateResponseV0 {
                result: Some(
                    get_contested_resource_vote_status_response_v0::Result::ContestedResourceVoters(
                        balance,
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
