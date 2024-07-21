use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::{
    get_contested_resource_vote_state_response_v0, GetContestedResourceVoteStateResponseV0,
};
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::finished_vote_info::FinishedVoteOutcome;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::FinishedVoteInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::{check_validation_result_with_data, platform_value};
use dpp::voting::contender_structs::{ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0};
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use drive::error::query::QuerySyntaxError;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery,
};

impl<C> Platform<C> {
    pub(super) fn query_contested_resource_vote_state_v0(
        &self,
        GetContestedResourceVoteStateRequestV0 {
            contract_id,
            document_type_name,
            index_name,
            index_values,
            result_type,
            allow_include_locked_and_abstaining_vote_tally,
            start_at_identifier_info,
            count,
            prove,
        }: GetContestedResourceVoteStateRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetContestedResourceVoteStateResponseV0>, Error> {
        let config = &self.config.drive;
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

        if index.name != index_name {
            return Ok(QueryValidationResult::new_with_error(QueryError::InvalidArgument(format!(
                "index with name {} is not the contested index on the document type {}, {} is the name of the only contested index",
                index_name, document_type_name, index.name
            ))));
        }

        if index.properties.len() != index_values.len() {
            return Ok(QueryValidationResult::new_with_error(QueryError::InvalidArgument(format!(
                "query uses index {}, this index has {} properties, but the query provided {} index values instead",
                index_name, index.properties.len(), index_values.len()
            ))));
        }

        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        let index_values = match index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                Ok(
                    bincode::decode_from_slice(serialized_value.as_slice(), bincode_config)
                        .map_err(|_| {
                            QueryError::InvalidArgument(format!(
                        "could not convert {:?} to a value in the index values at position {}",
                        serialized_value, pos
                    ))
                        })?
                        .0,
                )
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
        };

        let limit = check_validation_result_with_data!(count.map_or(
            Ok(config.default_query_limit),
            |limit| {
                let limit = u16::try_from(limit)
                    .map_err(|_| QueryError::InvalidArgument("limit out of bounds".to_string()))?;
                if limit == 0 || limit > config.default_query_limit {
                    Err(QueryError::InvalidArgument(format!(
                        "limit {} out of bounds of [1, {}]",
                        limit, config.default_query_limit
                    )))
                } else {
                    Ok(limit)
                }
            }
        ));

        let query = ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type: result_type.try_into()?,
            offset: None,
            limit: Some(limit),
            start_at: start_at_identifier_info
                .map(|start_at_identifier_info| {
                    Ok::<([u8; 32], bool), platform_value::Error>((
                        Identifier::from_vec(start_at_identifier_info.start_identifier)?
                            .to_buffer(),
                        start_at_identifier_info.start_identifier_included,
                    ))
                })
                .transpose()?,
            allow_include_locked_and_abstaining_vote_tally,
        };

        let response = if prove {
            let proof = match query.execute_with_proof(&self.drive, None, None, platform_version) {
                Ok(result) => result.0,
                Err(drive::error::Error::Query(query_error)) => {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        query_error,
                    )));
                }
                Err(e) => return Err(e.into()),
            };

            GetContestedResourceVoteStateResponseV0 {
                result: Some(
                    get_contested_resource_vote_state_response_v0::Result::Proof(
                        self.response_proof_v0(platform_state, proof),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        } else {
            let results =
                match query.execute_no_proof(&self.drive, None, &mut vec![], platform_version) {
                    Ok(result) => result,
                    Err(drive::error::Error::Query(query_error)) => {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            query_error,
                        )));
                    }
                    Err(e) => return Err(e.into()),
                };

            let abstain_vote_tally = results.abstaining_vote_tally;
            let lock_vote_tally = results.locked_vote_tally;
            let finished_vote_info =
                results
                    .winner
                    .map(|(winner_info, finished_at_block_info)| match winner_info {
                        ContestedDocumentVotePollWinnerInfo::NoWinner => FinishedVoteInfo {
                            finished_vote_outcome: FinishedVoteOutcome::NoPreviousWinner as i32,
                            won_by_identity_id: None,
                            finished_at_block_height: finished_at_block_info.height,
                            finished_at_core_block_height: finished_at_block_info.core_height,
                            finished_at_block_time_ms: finished_at_block_info.time_ms,
                            finished_at_epoch: finished_at_block_info.epoch.index as u32,
                        },
                        ContestedDocumentVotePollWinnerInfo::WonByIdentity(identity_id) => {
                            FinishedVoteInfo {
                                finished_vote_outcome: FinishedVoteOutcome::TowardsIdentity as i32,
                                won_by_identity_id: Some(identity_id.to_vec()),
                                finished_at_block_height: finished_at_block_info.height,
                                finished_at_core_block_height: finished_at_block_info.core_height,
                                finished_at_block_time_ms: finished_at_block_info.time_ms,
                                finished_at_epoch: finished_at_block_info.epoch.index as u32,
                            }
                        }
                        ContestedDocumentVotePollWinnerInfo::Locked => FinishedVoteInfo {
                            finished_vote_outcome: FinishedVoteOutcome::Locked as i32,
                            won_by_identity_id: None,
                            finished_at_block_height: finished_at_block_info.height,
                            finished_at_core_block_height: finished_at_block_info.core_height,
                            finished_at_block_time_ms: finished_at_block_info.time_ms,
                            finished_at_epoch: finished_at_block_info.epoch.index as u32,
                        },
                    });

            let contenders = results
                .contenders
                .into_iter()
                .map(|contender| match contender {
                    ContenderWithSerializedDocument::V0(ContenderWithSerializedDocumentV0 {
                        identity_id,
                        serialized_document,
                        vote_tally,
                    }) => get_contested_resource_vote_state_response_v0::Contender {
                        identifier: identity_id.to_vec(),
                        vote_count: vote_tally,
                        document: serialized_document,
                    },
                })
                .collect();

            GetContestedResourceVoteStateResponseV0 {
                result: Some(
                    get_contested_resource_vote_state_response_v0::Result::ContestedResourceContenders(
                        get_contested_resource_vote_state_response_v0::ContestedResourceContenders {
                            contenders,
                            abstain_vote_tally,
                            lock_vote_tally,
                            finished_vote_info,
                        },
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
