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
use drive::util::grove_operations::GroveDBToUse;
use crate::query::response_metadata::CheckpointUsed;

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

        if let Err(error) = super::super::validate_serialized_index_values(
            index_values.iter().map(Vec::as_slice),
            index.properties.len(),
            || "serialized index values exceed the contested index query limits".to_string(),
        ) {
            return Ok(QueryValidationResult::new_with_error(error));
        }

        let index_values = match index_values
            .into_iter()
            .enumerate()
            .map(|(pos, serialized_value)| {
                super::super::decode_serialized_index_value(serialized_value.as_slice(), || {
                    format!(
                        "could not convert a value in the index values at position {}",
                        pos
                    )
                })
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

            let (grovedb_used, proof) =
                self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)?;

            GetContestedResourceVoteStateResponseV0 {
                result: Some(get_contested_resource_vote_state_response_v0::Result::Proof(proof)),
                metadata: Some(self.response_metadata_v0(platform_state, grovedb_used)),
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
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_contested_resource_vote_state_invalid_contract_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceVoteStateRequestV0 {
            contract_id: vec![0; 8],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            index_values: vec![],
            result_type: 0,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at_identifier_info: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_vote_state_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id must be a valid identifier")
        ));
    }

    #[test]
    fn test_query_contested_resource_vote_state_contract_not_found() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceVoteStateRequestV0 {
            contract_id: vec![0; 32],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            index_values: vec![],
            result_type: 0,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at_identifier_info: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_vote_state_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(_))]
        ));
    }

    #[test]
    fn test_query_contested_resource_vote_state_contract_not_found_prove() {
        // Exercises the prove branch dispatch when the contract is missing.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceVoteStateRequestV0 {
            contract_id: vec![0; 32],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            index_values: vec![],
            result_type: 0,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at_identifier_info: None,
            count: None,
            prove: true,
        };

        let result = platform
            .query_contested_resource_vote_state_v0(request, &state, version)
            .expect("expected query to succeed");

        // The contract-not-found check fires before the prove split, so this
        // still returns a DataContractNotFound error.
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::DataContractNotFound(_))]
        ));
    }

    /// NOTE: the `count` guards on lines 132-142 are not reachable here
    /// without a valid 32-byte `contract_id` + an existing contract, so
    /// these two tests deliberately pin validation **ordering** — i.e.
    /// that `contract_id.try_into()` fires before the count checks. They
    /// both assert the same `InvalidArgument("contract_id …")` message.
    #[test]
    fn test_query_contested_resource_vote_state_invalid_contract_id_runs_before_count_zero() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceVoteStateRequestV0 {
            contract_id: vec![0; 8],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            index_values: vec![],
            result_type: 0,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at_identifier_info: None,
            count: Some(0),
            prove: false,
        };

        let result = platform
            .query_contested_resource_vote_state_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id must be a valid identifier")
        ));
    }

    #[test]
    fn test_query_contested_resource_vote_state_invalid_contract_id_runs_before_count_over_u16() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetContestedResourceVoteStateRequestV0 {
            contract_id: vec![0; 8],
            document_type_name: "x".to_string(),
            index_name: "x".to_string(),
            index_values: vec![],
            result_type: 0,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at_identifier_info: None,
            count: Some((u16::MAX as u32) + 1),
            prove: false,
        };

        let result = platform
            .query_contested_resource_vote_state_v0(request, &state, version)
            .expect("expected query to succeed");

        // Same `contract_id` error as above — confirms validation ordering.
        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg.contains("contract_id must be a valid identifier")
        ));
    }

    #[test]
    fn test_query_contested_resource_vote_state_aggregate_index_bytes_rejected() {
        let (platform, state, version) = setup_platform(Some((1, 1)), Network::Testnet, None);

        let request = GetContestedResourceVoteStateRequestV0 {
            contract_id: dpp::system_data_contracts::dpns_contract::ID.to_vec(),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![vec![0xff; 5 * 1024], vec![]],
            result_type: 0,
            allow_include_locked_and_abstaining_vote_tally: false,
            start_at_identifier_info: None,
            count: None,
            prove: false,
        };

        let result = platform
            .query_contested_resource_vote_state_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)]
                if msg == "serialized index values exceed the contested index query limits"
        ));
    }
}
