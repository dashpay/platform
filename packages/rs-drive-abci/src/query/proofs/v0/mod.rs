use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::vote_status_request::RequestType;
use dapi_grpc::platform::v0::get_proofs_request::GetProofsRequestV0;
use dapi_grpc::platform::v0::get_proofs_response::{get_proofs_response_v0, GetProofsResponseV0};
use dpp::check_validation_result_with_data;
use dpp::platform_value::Bytes32;
use dpp::prelude::Identifier;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::drive::identity::{IdentityDriveQuery, IdentityProveRequestType};
use drive::query::{IdentityBasedVoteDriveQuery, SingleDocumentDriveQuery};

impl<C> Platform<C> {
    pub(super) fn query_proofs_v0(
        &self,
        GetProofsRequestV0 {
            identities,
            contracts,
            documents,
            votes,
        }: GetProofsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponseV0>, Error> {
        let contract_ids = check_validation_result_with_data!(contracts
            .into_iter()
            .map(|contract_request| {
                Bytes32::from_vec(contract_request.contract_id)
                    .map(|bytes| (bytes.0, None))
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<([u8; 32], Option<bool>)>, QueryError>>());

        let identity_requests = check_validation_result_with_data!(identities
            .into_iter()
            .map(|identity_request| {
                Ok(IdentityDriveQuery {
                    identity_id: Bytes32::from_vec(identity_request.identity_id)
                        .map(|bytes| bytes.0)
                        .map_err(|_| {
                            QueryError::InvalidArgument(
                                "id must be a valid identifier (32 bytes long)".to_string(),
                            )
                        })?,
                    prove_request_type: IdentityProveRequestType::try_from(
                        identity_request.request_type as u8,
                    )
                    .map_err(|_| {
                        QueryError::InvalidArgument(
                            format!(
                                "invalid prove request type '{}'",
                                identity_request.request_type
                            )
                            .to_string(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<IdentityDriveQuery>, QueryError>>());

        let document_queries = check_validation_result_with_data!(documents
            .into_iter()
            .map(|document_proof_request| {
                let contract_id: Identifier =
                    document_proof_request.contract_id.try_into().map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;
                let document_id: Identifier =
                    document_proof_request.document_id.try_into().map_err(|_| {
                        QueryError::InvalidArgument(
                            "id must be a valid identifier (32 bytes long)".to_string(),
                        )
                    })?;

                let contested_status = document_proof_request
                    .document_contested_status
                    .try_into()?;

                Ok(SingleDocumentDriveQuery {
                    contract_id: contract_id.into_buffer(),
                    document_type_name: document_proof_request.document_type,
                    document_type_keeps_history: document_proof_request.document_type_keeps_history,
                    document_id: document_id.into_buffer(),
                    block_time_ms: None, //None because we want latest
                    contested_status,
                })
            })
            .collect::<Result<Vec<SingleDocumentDriveQuery>, QueryError>>());

        let vote_queries = check_validation_result_with_data!(votes
            .into_iter()
            .filter_map(|vote_proof_request| {
                if let Some(request_type) = vote_proof_request.request_type {
                    match request_type {
                        RequestType::ContestedResourceVoteStatusRequest(contested_resource_vote_status_request) => {
                            let identity_id = match contested_resource_vote_status_request.voter_identifier.try_into() {
                                Ok(identity_id) => identity_id,
                                Err(_) => return Some(Err(QueryError::InvalidArgument(
                                    "voter_identifier must be a valid identifier (32 bytes long)".to_string(),
                                ))),
                            };
                            let contract_id = match contested_resource_vote_status_request.contract_id.try_into() {
                                Ok(contract_id) => contract_id,
                                Err(_) => return Some(Err(QueryError::InvalidArgument(
                                    "contract_id must be a valid identifier (32 bytes long)".to_string(),
                                ))),
                            };
                            let document_type_name = contested_resource_vote_status_request.document_type_name;
                            let index_name = contested_resource_vote_status_request.index_name;
                            let index_values = match contested_resource_vote_status_request.index_values.into_iter().enumerate().map(|(pos, serialized_value)|
                                Ok(bincode::decode_from_slice(serialized_value.as_slice(), bincode::config::standard().with_big_endian()
                                    .with_no_limit()).map_err(|_| QueryError::InvalidArgument(
                                    format!("could not convert {:?} to a value in the index values at position {}", serialized_value, pos),
                                ))?.0)
                            ).collect::<Result<Vec<_>, QueryError>>() {
                                Ok(index_values) => index_values,
                                Err(e) => return Some(Err(e)),
                            };
                            let vote_poll = ContestedDocumentResourceVotePoll {
                                contract_id,
                                document_type_name,
                                index_name,
                                index_values,
                            }.into();
                            Some(Ok(IdentityBasedVoteDriveQuery {
                                identity_id,
                                vote_poll,
                            }))
                        }
                    }
                } else {
                    None
                }
            })
            .collect::<Result<Vec<IdentityBasedVoteDriveQuery>, QueryError>>());

        let proof = self.drive.prove_multiple_state_transition_results(
            &identity_requests,
            &contract_ids,
            &document_queries,
            &vote_queries,
            None,
            platform_version,
        )?;

        let response = GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(
                self.response_proof_v0(platform_state, proof),
            )),
            metadata: Some(self.response_metadata_v0(platform_state)),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::vote_status_request::ContestedResourceVoteStatusRequest;
    use dapi_grpc::platform::v0::get_proofs_request::get_proofs_request_v0::{
        ContractRequest, DocumentRequest, IdentityRequest, VoteStatusRequest,
    };
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::platform_value::Value;
    use dpp::util::strings::convert_to_homograph_safe_chars;

    #[test]
    fn test_invalid_identity_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 8],
                request_type: 0,
            }],
            contracts: vec![],
            documents: vec![],
            votes: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_identity_prove_request_type() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request_type = 10;

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 32],
                request_type,
            }],
            contracts: vec![],
            documents: vec![],
            votes: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::InvalidArgument(msg)] if msg == &format!(
                "invalid prove request type '{}'",
                request_type
            )
        ))
    }

    #[test]
    fn test_invalid_contract_ids() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![ContractRequest {
                contract_id: vec![0; 8],
            }],
            documents: vec![],
            votes: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_contract_id_for_documents_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 8],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 32],
                document_contested_status: 0,
            }],
            votes: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_document_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 8],
                document_contested_status: 0,
            }],
            votes: vec![],
        };

        let result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_proof_of_absence() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![0; 32],
                document_contested_status: 0,
            }],
            votes: vec![],
        };

        let validation_result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }

    #[test]
    fn test_proof_of_absence_of_vote() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let dpns_contract = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns()
            .as_ref()
            .clone();

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let serialized_index_values = [
            Value::Text("dash".to_string()),
            Value::Text(convert_to_homograph_safe_chars("quantum")),
        ]
        .iter()
        .map(|value| {
            bincode::encode_to_vec(value, config).expect("expected to encode value in path")
        })
        .collect();

        let request = GetProofsRequestV0 {
            identities: vec![],
            contracts: vec![],
            documents: vec![],
            votes: vec![VoteStatusRequest {
                request_type: Some(RequestType::ContestedResourceVoteStatusRequest(
                    ContestedResourceVoteStatusRequest {
                        contract_id: dpns_contract.id().to_vec(),
                        document_type_name: "domain".to_string(),
                        index_name: "parentNameAndLabel".to_string(),
                        index_values: serialized_index_values,
                        voter_identifier: [0u8; 32].to_vec(),
                    },
                )),
            }],
        };

        let validation_result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }

    #[test]
    fn test_prove_all() {
        let (platform, state, version) = setup_platform(None, Network::Testnet);

        let request = GetProofsRequestV0 {
            identities: vec![IdentityRequest {
                identity_id: vec![0; 32],
                request_type: 0,
            }],
            contracts: vec![ContractRequest {
                contract_id: vec![0; 32],
            }],
            documents: vec![DocumentRequest {
                contract_id: vec![0; 32],
                document_type: "niceDocument".to_string(),
                document_type_keeps_history: false,
                document_id: vec![1; 32],
                document_contested_status: 0,
            }],
            votes: vec![],
        };

        let validation_result = platform
            .query_proofs_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(validation_result.data, Some(GetProofsResponseV0 {
            result: Some(get_proofs_response_v0::Result::Proof(proof)),
            metadata: Some(_),
        }) if !proof.grovedb_proof.is_empty()));
    }
}
