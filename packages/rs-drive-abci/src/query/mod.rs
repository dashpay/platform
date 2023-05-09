use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform::Platform;
use dapi_grpc::platform::v0::get_documents_request::Start;
use dapi_grpc::platform::v0::{
    get_data_contracts_response, get_identity_by_public_key_hashes_response,
    get_identity_keys_response, GetDataContractRequest, GetDataContractResponse,
    GetDataContractsRequest, GetDataContractsResponse, GetDocumentsRequest, GetDocumentsResponse,
    GetIdentitiesByPublicKeyHashesRequest, GetIdentitiesByPublicKeyHashesResponse,
    GetIdentityBalanceAndRevisionResponse, GetIdentityBalanceResponse,
    GetIdentityByPublicKeyHashesRequest, GetIdentityByPublicKeyHashesResponse,
    GetIdentityKeysRequest, GetIdentityKeysResponse, GetIdentityRequest, Proof, ResponseMetadata,
};
use dpp::identifier::Identifier;
use dpp::platform_value::{Bytes20, Bytes32, Value, ValueMapHelper};
use std::collections::BTreeMap;

use dpp::serialization_traits::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::{check_validation_result_with_data, ProtocolError};

use dapi_grpc::platform::v0::get_data_contracts_response::DataContractEntry;
use dpp::identity::{KeyID, Purpose, SecurityLevel};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
    SerializedKeyVec,
};
use drive::error::query::QuerySyntaxError;
use drive::query::DriveQuery;
use prost::Message;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

fn from_i32_to_key_kind_request_type(value: i32) -> Option<KeyKindRequestType> {
    match value {
        0 => Some(KeyKindRequestType::CurrentKeyOfKindRequest),
        1 => Some(KeyKindRequestType::AllKeysOfKindRequest),
        _ => None,
    }
}

fn convert_key_request_type(
    request_type: dapi_grpc::platform::v0::key_request_type::Request,
) -> Result<KeyRequestType, QueryError> {
    match request_type {
        dapi_grpc::platform::v0::key_request_type::Request::AllKeys(_) => {
            Ok(KeyRequestType::AllKeys)
        }
        dapi_grpc::platform::v0::key_request_type::Request::SpecificKeys(specific_keys) => {
            let key_ids = specific_keys
                .key_ids
                .into_iter()
                .map(|id| id as KeyID)
                .collect();
            Ok(KeyRequestType::SpecificKeys(key_ids))
        }
        dapi_grpc::platform::v0::key_request_type::Request::SearchKey(search_key) => {
            let purpose_map = search_key.purpose_map.into_iter().map(|(purpose, security_level_map)| {
                let security_level_map = security_level_map.security_level_map.into_iter().map(|(security_level, key_kind_request_type)| {
                    if security_level > u8::MAX as u32 {
                        return Err(QueryError::Query(QuerySyntaxError::InvalidKeyParameter("security level out of bounds".to_string())));
                    }
                    let security_level = SecurityLevel::try_from(security_level as u8).map_err(|_| QueryError::Query(QuerySyntaxError::InvalidKeyParameter(format!("security level {} not recognized", security_level))))?;

                    let key_kind_request_type = from_i32_to_key_kind_request_type(key_kind_request_type).ok_or(QueryError::Query(QuerySyntaxError::InvalidKeyParameter(format!("unknown key kind request type {}", key_kind_request_type))))?;
                    Ok((
                        security_level as u8,
                        key_kind_request_type,
                    ))
                }).collect::<Result<BTreeMap<SecurityLevelU8, KeyKindRequestType>, QueryError>>()?;
                if purpose > u8::MAX as u32 {
                    return Err(QueryError::Query(QuerySyntaxError::InvalidKeyParameter("purpose out of bounds".to_string())));
                }
                let purpose = Purpose::try_from(purpose as u8).map_err(|_| QueryError::Query(QuerySyntaxError::InvalidKeyParameter(format!("purpose {} not recognized", purpose))))?;

                Ok((purpose as u8, security_level_map))
            }).collect::<Result<BTreeMap<PurposeU8, BTreeMap<SecurityLevelU8, KeyKindRequestType>>, QueryError>>()?;

            Ok(KeyRequestType::SearchKey(purpose_map))
        }
    }
}

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
        _prove: &bool,
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
            "/identity/keys" => {
                let GetIdentityKeysRequest {
                    identity_id,
                    request_type,
                    limit,
                    offset,
                    prove,
                } = check_validation_result_with_data!(GetIdentityKeysRequest::decode(query_data));
                let identity_id: Identifier =
                    check_validation_result_with_data!(identity_id.try_into());
                if let Some(limit) = limit {
                    if limit > u16::MAX as u32 {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::InvalidParameter("limit out of bounds".to_string()),
                        )));
                    }
                    if limit as u16 > self.config.drive.max_query_limit {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::InvalidLimit(format!(
                                "limit greater than max limit {}",
                                self.config.drive.max_query_limit
                            )),
                        )));
                    }
                }

                if let Some(offset) = offset {
                    if offset > u16::MAX as u32 {
                        return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                            QuerySyntaxError::InvalidParameter("limit out of bounds".to_string()),
                        )));
                    }
                }
                let Some(request_type) =  request_type  else {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidParameter("key request must be defined".to_string()),
                    )));
                };
                let Some(request) =  request_type.request  else {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidParameter("key request must be defined".to_string()),
                    )));
                };
                let key_request_type =
                    check_validation_result_with_data!(convert_key_request_type(request));
                let key_request = IdentityKeysRequest {
                    identity_id: identity_id.into_buffer(),
                    request_type: key_request_type,
                    limit: limit.map(|l| l as u16),
                    offset: offset.map(|o| o as u16),
                };
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_identity_keys(key_request, None));
                    GetIdentityKeysResponse {
                        result: Some(get_identity_keys_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let keys: SerializedKeyVec = check_validation_result_with_data!(self
                        .drive
                        .fetch_identity_keys(key_request, None));
                    GetIdentityKeysResponse {
                        result: Some(get_identity_keys_response::Result::Keys(
                            get_identity_keys_response::Keys { keys_bytes: keys },
                        )),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
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
            "/dataContracts" => {
                let GetDataContractsRequest { ids, prove } =
                    check_validation_result_with_data!(GetDataContractsRequest::decode(query_data));
                let contract_ids = check_validation_result_with_data!(ids
                    .into_iter()
                    .map(|contract_id_vec| {
                        Bytes32::from_vec(contract_id_vec).map(|bytes| bytes.0)
                    })
                    .collect::<Result<Vec<[u8; 32]>, dpp::platform_value::Error>>());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_contracts(contract_ids.as_slice(), None));
                    GetDataContractsResponse {
                        metadata: Some(metadata),
                        result: Some(get_data_contracts_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                    }
                    .encode_to_vec()
                } else {
                    let contracts = check_validation_result_with_data!(self
                        .drive
                        .get_contracts_with_fetch_info(contract_ids.as_slice(), false, None));

                    let contracts = check_validation_result_with_data!(contracts
                        .into_iter()
                        .map(
                            |(key, maybe_contract)| Ok::<DataContractEntry, ProtocolError>(
                                get_data_contracts_response::DataContractEntry {
                                    key: key.to_vec(),
                                    value: maybe_contract
                                        .map(|contract| Ok::<
                                            get_data_contracts_response::DataContractValue,
                                            ProtocolError,
                                        >(
                                            get_data_contracts_response::DataContractValue {
                                                value: contract.contract.serialize()?
                                            }
                                        ))
                                        .transpose()?,
                                }
                            )
                        )
                        .collect());
                    GetDataContractsResponse {
                        result: Some(get_data_contracts_response::Result::DataContracts(
                            get_data_contracts_response::DataContracts {
                                data_contract_entries: contracts,
                            },
                        )),
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
                    .get_contract_with_fetch_info_and_fee(
                        contract_id.to_buffer(),
                        None,
                        true,
                        None
                    ));
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
                        QuerySyntaxError::InvalidLimit(format!("limit {} out of bounds", limit)),
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
                        document_type,
                        &self.config.drive,
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
            "/identity/by-public-key-hash" => {
                let GetIdentityByPublicKeyHashesRequest {
                    public_key_hash,
                    prove,
                } = check_validation_result_with_data!(
                    GetIdentityByPublicKeyHashesRequest::decode(query_data)
                );
                let public_key_hash = check_validation_result_with_data!(Bytes20::from_vec(
                    public_key_hash
                )
                .map(|bytes| bytes.0));
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_full_identity_by_unique_public_key_hash(public_key_hash, None));
                    GetIdentityByPublicKeyHashesResponse {
                        metadata: Some(metadata),
                        result: Some(get_identity_by_public_key_hashes_response::Result::Proof(
                            Proof {
                                grovedb_proof: proof,
                                quorum_hash: state.last_quorum_hash().to_vec(),
                                signature: state.last_block_signature().to_vec(),
                                round: state.last_block_round(),
                            },
                        )),
                    }
                    .encode_to_vec()
                } else {
                    let maybe_identity = check_validation_result_with_data!(self
                        .drive
                        .fetch_full_identity_by_unique_public_key_hash(public_key_hash, None,));
                    let serialized_identity = check_validation_result_with_data!(maybe_identity
                        .map(|identity| identity.serialize_consume())
                        .transpose());
                    GetIdentityByPublicKeyHashesResponse {
                        metadata: Some(metadata),
                        result: Some(
                            get_identity_by_public_key_hashes_response::Result::Identity(
                                serialized_identity.unwrap_or_default(),
                            ),
                        ),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/identities/by-public-key-hash" => {
                let decoded_data: Value =
                    check_validation_result_with_data!(ciborium::de::from_reader(query_data)
                        .map_err(|_| {
                            QueryError::Query(QuerySyntaxError::DeserializationError(
                                "unable to decode query data from cbor".to_string(),
                            ))
                        }));

                let decoded_data_map = check_validation_result_with_data!(decoded_data
                    .as_map()
                    .ok_or(QueryError::Query(QuerySyntaxError::DeserializationError(
                        "query data is not a map".to_string()
                    ))));

                let public_key_hashes_value = check_validation_result_with_data!(decoded_data_map
                    .get_key("publicKeyHashes")
                    .map_err(|_| {
                        QueryError::Query(QuerySyntaxError::DeserializationError(
                            "publicKeyHashes is missing in query data ".to_string(),
                        ))
                    }));

                let public_key_hashes = check_validation_result_with_data!(public_key_hashes_value
                    .as_array()
                    .ok_or(QueryError::Query(QuerySyntaxError::DeserializationError(
                        "publicKeyHashes is not array".to_string(),
                    ))));

                let public_key_hashes = check_validation_result_with_data!(public_key_hashes
                    .into_iter()
                    .map(|pub_key_hash_value| {
                        Bytes20::try_from(pub_key_hash_value).map(|bytes| bytes.0)
                    })
                    .collect::<Result<Vec<[u8; 20]>, dpp::platform_value::Error>>());
                let response_data = if *_prove {
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
            }
            other => Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::Unsupported(format!("query path '{}' is not supported", other)),
            ))),
        }
    }
}
