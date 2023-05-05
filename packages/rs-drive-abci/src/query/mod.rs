use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform::Platform;
use dapi_grpc::platform::v0::get_documents_request::Start;
use dapi_grpc::platform::v0::{
    GetDataContractRequest, GetDataContractResponse, GetDocumentsRequest, GetDocumentsResponse,
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse,
    GetIdentityBalanceAndRevisionResponse, GetIdentityBalanceResponse, GetIdentityRequest, Proof,
    ResponseMetadata,
};
use dpp::identifier::Identifier;
use dpp::platform_value::Bytes20;

use dpp::serialization_traits::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::{check_validation_result_with_data, ProtocolError};

use drive::error::query::QuerySyntaxError;
use drive::query::DriveQuery;
use prost::Message;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
        _prove: bool,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let state = self.state.read().unwrap();
        let metadata = ResponseMetadata {
            height: state.height(),
            core_chain_locked_height: state.core_height(),
            time_ms: state.last_block_time_ms().unwrap_or_default(),
            protocol_version: state.current_protocol_version_in_consensus,
        };
        match query_path {
            "/identity/balance" => {
                let GetIdentityRequest { id, prove } =
                    check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
                let identity_id: Identifier = check_validation_result_with_data!(id.try_into());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_identity_balance(identity_id.into_buffer(), None));
                    GetIdentityBalanceResponse {
                        balance: None,
                        proof: Some(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        }),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let balance = check_validation_result_with_data!(self
                        .drive
                        .fetch_identity_balance(identity_id.into_buffer(), None,));
                    GetIdentityBalanceResponse {
                        balance,
                        proof: None,
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/identity/balanceAndRevision" => {
                let GetIdentityRequest { id, prove } =
                    check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
                let identity_id: Identifier = check_validation_result_with_data!(id.try_into());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_identity_balance_and_revision(identity_id.into_buffer(), None));
                    GetIdentityBalanceResponse {
                        balance: None,
                        proof: Some(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        }),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let balance = check_validation_result_with_data!(self
                        .drive
                        .fetch_identity_balance(identity_id.into_buffer(), None,));
                    let revision = check_validation_result_with_data!(self
                        .drive
                        .fetch_identity_revision(identity_id.into_buffer(), true, None,));
                    GetIdentityBalanceAndRevisionResponse {
                        balance,
                        revision,
                        proof: None,
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/identities/keys" => {
                // let identity_id = query.remove_identifier("identityIds")?;
                // let request = query.str_val("keyRequest")?;
                todo!()
            }
            "/dataContract" => {
                let GetDataContractRequest { id, prove } =
                    check_validation_result_with_data!(GetDataContractRequest::decode(query_data));
                let contract_id: Identifier = check_validation_result_with_data!(id.try_into());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_contract(contract_id.into_buffer(), None));
                    GetIdentityBalanceResponse {
                        balance: None,
                        proof: Some(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        }),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let contract = check_validation_result_with_data!(self
                        .drive
                        .fetch_contract(contract_id.into_buffer(), None, None,)
                        .unwrap())
                    .map(|contract| contract.contract.serialize())
                    .transpose()?;
                    GetDataContractResponse {
                        data_contract: contract.unwrap_or_default(),
                        proof: None,
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/documents" | "/dataContract/documents" => {
                let GetDocumentsRequest {
                    data_contract_id,
                    document_type: document_type_name,
                    r#where,
                    order_by,
                    limit,
                    prove,
                    start,
                } = check_validation_result_with_data!(GetDocumentsRequest::decode(query_data));
                let contract_id: Identifier =
                    check_validation_result_with_data!(data_contract_id.try_into());
                let (_, contract) = check_validation_result_with_data!(self
                    .drive
                    .get_contract_with_fetch_info(contract_id.to_buffer(), None, true, None));
                let contract = check_validation_result_with_data!(contract.ok_or(
                    QueryError::Query(QuerySyntaxError::ContractNotFound(
                        "contract not found when querying from value with contract info",
                    ))
                ));
                let contract_ref = &contract.contract;
                let document_type = check_validation_result_with_data!(
                    contract_ref.document_type_for_name(document_type_name.as_str())
                );

                let where_clause = check_validation_result_with_data!(ciborium::de::from_reader(
                    r#where.as_slice()
                )
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode query from cbor".to_string(),
                    ))
                }));

                let order_by = check_validation_result_with_data!(ciborium::de::from_reader(
                    order_by.as_slice()
                )
                .map_err(|_| {
                    QueryError::Query(QuerySyntaxError::DeserializationError(
                        "unable to decode query from cbor".to_string(),
                    ))
                }));

                let (start_at_included, start_at) = if let Some(start) = start {
                    match start {
                        Start::StartAfter(after) => (false, Some(after)),
                        Start::StartAt(at) => (true, Some(at)),
                    }
                } else {
                    (true, None)
                };

                if limit > u16::MAX as u32 {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidLimit("limit out of bounds"),
                    )));
                }

                let drive_query =
                    check_validation_result_with_data!(DriveQuery::from_decomposed_values(
                        where_clause,
                        order_by,
                        Some(limit as u16),
                        start_at,
                        start_at_included,
                        None,
                        contract_ref,
                        document_type
                    ));
                let response_data = if prove {
                    let (proof, _) = check_validation_result_with_data!(
                        drive_query.execute_with_proof(&self.drive, None, None)
                    );
                    GetDocumentsResponse {
                        documents: vec![],
                        proof: Some(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        }),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let results = check_validation_result_with_data!(
                        drive_query.execute_raw_results_no_proof(&self.drive, None, None,)
                    )
                    .0;
                    GetDocumentsResponse {
                        documents: results,
                        proof: None,
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            // "/identity/by-public-key-hash" => {
            //     let GetIdentitiesByPublicKeyHashesRequest {
            //         id, prove
            //     }  = check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
            //     // let public_key_hash = query.remove_bytes_20("publicKeyHash")?;
            //     // if prove {
            //     //     self.prove_full_identity_by_unique_public_key_hash(
            //     //         public_key_hash.into_buffer(),
            //     //         None,
            //     //     )
            //     // } else {
            //     //     self.fetch_serialized_full_identity_by_unique_public_key_hash(
            //     //         public_key_hash.into_buffer(),
            //     //         CborEncodedQueryResult,
            //     //         None,
            //     //     )
            //     // }
            // }
            "/identities/by-public-key-hash" => {
                let GetIdentitiesByPublicKeyHashesRequest {
                    public_key_hashes,
                    prove,
                } = check_validation_result_with_data!(
                    GetIdentitiesByPublicKeyHashesRequest::decode(query_data)
                );
                let public_key_hashes = check_validation_result_with_data!(public_key_hashes
                    .into_iter()
                    .map(|pub_key_hash_vec| {
                        Bytes20::from_vec(pub_key_hash_vec).map(|bytes| bytes.0)
                    })
                    .collect::<Result<Vec<[u8; 20]>, dpp::platform_value::Error>>());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_full_identities_by_unique_public_key_hashes(
                            &public_key_hashes,
                            None
                        ));
                    GetIdentitiesByPublicKeyHashesResponse {
                        identities: vec![],
                        proof: Some(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        }),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    //todo: fix this so we return optionals
                    let identities = check_validation_result_with_data!(self
                        .drive
                        .fetch_full_identities_by_unique_public_key_hashes(
                            public_key_hashes.as_slice(),
                            None,
                        ));
                    let identities = check_validation_result_with_data!(identities
                        .into_values()
                        .filter_map(|maybe_identity| Some(maybe_identity?.serialize_consume()))
                        .collect::<Result<Vec<Vec<u8>>, ProtocolError>>());
                    GetIdentitiesByPublicKeyHashesResponse {
                        identities,
                        proof: None,
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))

                // let public_key_hashes_values: Vec<_> =
                //     query.remove_inner_value_array("publicKeyHashes")?;
                // let public_key_hashes = public_key_hashes_values
                //     .into_iter()
                //     .map(|pub_key_hash_value| {
                //         pub_key_hash_value.into_bytes_20().map(|bytes| bytes.0)
                //     })
                //     .collect::<Result<Vec<[u8; 20]>, dpp::platform_value::Error>>()?;
                // if prove {
                //     self.prove_full_identities_by_unique_public_key_hashes(&public_key_hashes, None)
                // } else {
                //     self.fetch_serialized_full_identities_by_unique_public_key_hashes(
                //         public_key_hashes.as_slice(),
                //         CborEncodedQueryResult,
                //         None,
                //     )
                // }
            }
            other => Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::Unsupported(format!("query path '{}' is not supported", other)),
            ))),
        }
    }
}
