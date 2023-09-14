use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dapi_grpc::platform::v0::get_documents_request::Start;
use dapi_grpc::platform::v0::{
    get_data_contract_history_response, get_data_contract_response, get_data_contracts_response,
    get_documents_response, get_identities_by_public_key_hashes_response, get_identities_response,
    get_identity_balance_and_revision_response, get_identity_balance_response,
    get_identity_by_public_key_hashes_response, get_identity_keys_response, get_identity_response,
    GetDataContractHistoryRequest, GetDataContractHistoryResponse, GetDataContractRequest,
    GetDataContractResponse, GetDataContractsRequest, GetDataContractsResponse,
    GetDocumentsRequest, GetDocumentsResponse, GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse, GetIdentitiesRequest, GetIdentitiesResponse,
    GetIdentityBalanceAndRevisionResponse, GetIdentityBalanceResponse,
    GetIdentityByPublicKeyHashesRequest, GetIdentityByPublicKeyHashesResponse,
    GetIdentityKeysRequest, GetIdentityKeysResponse, GetIdentityRequest, GetIdentityResponse,
    GetProofsRequest, GetProofsResponse, Proof, ResponseMetadata,
};
use dpp::identifier::Identifier;
use dpp::platform_value::{Bytes20, Bytes32};
use std::collections::BTreeMap;

use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use dpp::validation::ValidationResult;
use dpp::{check_validation_result_with_data, ProtocolError};
use drive::drive::identity::IdentityDriveQuery;
use drive::drive::identity::IdentityProveRequestType;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_data_contracts_response::DataContractEntry;
use dapi_grpc::platform::v0::get_identities_response::IdentityEntry;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::BalanceAndRevision;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identity::{KeyID, Purpose, SecurityLevel};
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
    SerializedKeyVec,
};
use drive::error::contract::DataContractError;
use drive::error::query::QuerySyntaxError;
use drive::query::{DriveQuery, SingleDocumentDriveQuery};
use prost::Message;

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
    pub(super) fn query_v0(
        &self,
        query_path: &str,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let state = self.state.read().unwrap();
        let metadata = ResponseMetadata {
            height: state.height(),
            core_chain_locked_height: state.core_height(),
            time_ms: state.last_block_time_ms().unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: state.current_protocol_version_in_consensus(),
        };
        let quorum_type: u32 = self.config.quorum_type() as u32;
        match query_path {
            "/identity" => {
                let GetIdentityRequest { id, prove } =
                    check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
                let identity_id: Identifier = check_validation_result_with_data!(id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self.drive.prove_full_identity(
                        identity_id.into_buffer(),
                        None,
                        &platform_version.drive
                    ));

                    if proof.is_empty() {
                        return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                            format!("identity {} not found", identity_id),
                        )));
                    }

                    GetIdentityResponse {
                        result: Some(get_identity_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let maybe_identity = check_validation_result_with_data!(self
                        .drive
                        .fetch_full_identity(identity_id.into_buffer(), None, platform_version)
                        .map_err(QueryError::Drive));

                    let identity = check_validation_result_with_data!(maybe_identity
                        .ok_or_else(|| {
                            QueryError::NotFound(format!("identity {} not found", identity_id))
                        })
                        .and_then(|identity| identity
                            .serialize_consume_to_bytes()
                            .map_err(QueryError::Protocol)));

                    GetIdentityResponse {
                        result: Some(get_identity_response::Result::Identity(identity)),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/identities" => {
                let GetIdentitiesRequest { ids, prove } =
                    check_validation_result_with_data!(GetIdentitiesRequest::decode(query_data));
                let identity_ids = check_validation_result_with_data!(ids
                    .into_iter()
                    .map(|identity_id_vec| {
                        Bytes32::from_vec(identity_id_vec)
                            .map(|bytes| bytes.0)
                            .map_err(|_| {
                                QueryError::InvalidArgument(
                                    "id must be a valid identifier (32 bytes long)".to_string(),
                                )
                            })
                    })
                    .collect::<Result<Vec<[u8; 32]>, QueryError>>());
                let response_data = if prove {
                    let proof =
                        check_validation_result_with_data!(self.drive.prove_full_identities(
                            identity_ids.as_slice(),
                            None,
                            &platform_version.drive
                        ));
                    GetIdentitiesResponse {
                        metadata: Some(metadata),
                        result: Some(get_identities_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                    }
                    .encode_to_vec()
                } else {
                    let identities = check_validation_result_with_data!(self
                        .drive
                        .fetch_full_identities(identity_ids.as_slice(), None, platform_version));

                    let identities = check_validation_result_with_data!(identities
                        .into_iter()
                        .map(|(key, maybe_identity)| Ok::<IdentityEntry, ProtocolError>(
                            get_identities_response::IdentityEntry {
                                key: key.to_vec(),
                                value: maybe_identity
                                    .map(|identity| Ok::<
                                        get_identities_response::IdentityValue,
                                        ProtocolError,
                                    >(
                                        get_identities_response::IdentityValue {
                                            value: identity.serialize_consume_to_bytes()?
                                        }
                                    ))
                                    .transpose()?,
                            }
                        ))
                        .collect());
                    GetIdentitiesResponse {
                        result: Some(get_identities_response::Result::Identities(
                            get_identities_response::Identities {
                                identity_entries: identities,
                            },
                        )),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/identity/balance" => {
                let GetIdentityRequest { id, prove } =
                    check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
                let identity_id: Identifier = check_validation_result_with_data!(id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));
                let response_data = if prove {
                    let proof =
                        check_validation_result_with_data!(self.drive.prove_identity_balance(
                            identity_id.into_buffer(),
                            None,
                            &platform_version.drive
                        ));
                    GetIdentityBalanceResponse {
                        result: Some(get_identity_balance_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let balance = check_validation_result_with_data!(self
                        .drive
                        .fetch_identity_balance(identity_id.into_buffer(), None, platform_version));
                    GetIdentityBalanceResponse {
                        result: Some(get_identity_balance_response::Result::Balance(
                            balance.unwrap(),
                        )),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/identity/balanceAndRevision" => {
                let GetIdentityRequest { id, prove } =
                    check_validation_result_with_data!(GetIdentityRequest::decode(query_data));
                let identity_id: Identifier = check_validation_result_with_data!(id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_identity_balance_and_revision(
                            identity_id.into_buffer(),
                            None,
                            &platform_version.drive
                        ));
                    GetIdentityBalanceResponse {
                        result: Some(get_identity_balance_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let balance = check_validation_result_with_data!(self
                        .drive
                        .fetch_identity_balance(identity_id.into_buffer(), None, platform_version));
                    let revision =
                        check_validation_result_with_data!(self.drive.fetch_identity_revision(
                            identity_id.into_buffer(),
                            true,
                            None,
                            platform_version
                        ));
                    GetIdentityBalanceAndRevisionResponse {
                        result: Some(
                            get_identity_balance_and_revision_response::Result::BalanceAndRevision(
                                BalanceAndRevision { balance, revision },
                            ),
                        ),
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
                let identity_id: Identifier = check_validation_result_with_data!(identity_id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));
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
                let Some(request_type) = request_type else {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidParameter(
                            "key request must be defined".to_string(),
                        ),
                    )));
                };
                let Some(request) = request_type.request else {
                    return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                        QuerySyntaxError::InvalidParameter(
                            "key request must be defined".to_string(),
                        ),
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
                let response_data =
                    if prove {
                        let proof = check_validation_result_with_data!(self
                            .drive
                            .prove_identity_keys(key_request, None, platform_version));
                        GetIdentityKeysResponse {
                            result: Some(get_identity_keys_response::Result::Proof(Proof {
                                grovedb_proof: proof,
                                quorum_hash: state.last_quorum_hash().to_vec(),
                                signature: state.last_block_signature().to_vec(),
                                round: state.last_block_round(),
                                block_id_hash: state.last_block_id_hash().to_vec(),
                                quorum_type,
                            })),
                            metadata: Some(metadata),
                        }
                        .encode_to_vec()
                    } else {
                        let keys: SerializedKeyVec = check_validation_result_with_data!(self
                            .drive
                            .fetch_identity_keys(key_request, None, platform_version));
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
                let contract_id: Identifier = check_validation_result_with_data!(id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self.drive.prove_contract(
                        contract_id.into_buffer(),
                        None,
                        platform_version
                    ));

                    if proof.is_empty() {
                        return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                            format!("data contract {} not found", contract_id),
                        )));
                    }

                    GetDataContractResponse {
                        result: Some(get_data_contract_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let maybe_data_contract = check_validation_result_with_data!(self
                        .drive
                        .fetch_contract(
                            contract_id.into_buffer(),
                            None,
                            None,
                            None,
                            platform_version
                        )
                        .unwrap());

                    let data_contract = check_validation_result_with_data!(maybe_data_contract
                        .ok_or_else(|| {
                            QueryError::NotFound(format!("data contract {} not found", contract_id))
                        })
                        .and_then(|data_contract| data_contract
                            .contract
                            .serialize_to_bytes_with_platform_version(platform_version)
                            .map_err(QueryError::Protocol)));

                    GetDataContractResponse {
                        result: Some(get_data_contract_response::Result::DataContract(
                            data_contract,
                        )),
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
                        Bytes32::from_vec(contract_id_vec)
                            .map(|bytes| bytes.0)
                            .map_err(|_| {
                                QueryError::InvalidArgument(
                                    "id must be a valid identifier (32 bytes long)".to_string(),
                                )
                            })
                    })
                    .collect::<Result<Vec<[u8; 32]>, QueryError>>());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self.drive.prove_contracts(
                        contract_ids.as_slice(),
                        None,
                        platform_version
                    ));
                    GetDataContractsResponse {
                        metadata: Some(metadata),
                        result: Some(get_data_contracts_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                    }
                    .encode_to_vec()
                } else {
                    let contracts = check_validation_result_with_data!(self
                        .drive
                        .get_contracts_with_fetch_info(
                            contract_ids.as_slice(),
                            false,
                            None,
                            platform_version
                        ));

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
                                                value: contract
                                                    .contract
                                                    .serialize_to_bytes_with_platform_version(
                                                        platform_version
                                                    )?
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
            "/dataContractHistory" => {
                let GetDataContractHistoryRequest {
                    id,
                    limit,
                    offset,
                    start_at_ms,
                    prove,
                } = check_validation_result_with_data!(GetDataContractHistoryRequest::decode(
                    query_data
                ));
                let contract_id: Identifier = check_validation_result_with_data!(id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));

                // TODO: make a cast safe
                let limit = limit
                    .map(|limit| {
                        u16::try_from(limit).map_err(|_| {
                            Error::Drive(drive::error::Error::DataContract(
                                DataContractError::Overflow(
                                    "can't fit u16 limit from the supplied value",
                                ),
                            ))
                        })
                    })
                    .transpose()?;
                let offset = offset
                    .map(|offset| {
                        u16::try_from(offset).map_err(|_| {
                            Error::Drive(drive::error::Error::DataContract(
                                DataContractError::Overflow(
                                    "can't fit u16 offset from the supplied value",
                                ),
                            ))
                        })
                    })
                    .transpose()?;

                let response_data = if prove {
                    let proof =
                        check_validation_result_with_data!(self.drive.prove_contract_history(
                            contract_id.to_buffer(),
                            None,
                            start_at_ms,
                            limit,
                            offset,
                            platform_version,
                        ));
                    GetDataContractHistoryResponse {
                        metadata: Some(metadata),
                        result: Some(get_data_contract_history_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                    }
                    .encode_to_vec()
                } else {
                    let contracts =
                        check_validation_result_with_data!(self.drive.fetch_contract_with_history(
                            contract_id.to_buffer(),
                            None,
                            start_at_ms,
                            limit,
                            offset,
                            platform_version,
                        ));

                    let contract_historical_entries = check_validation_result_with_data!(contracts
                        .into_iter()
                        .map(|(date_in_seconds, data_contract)| Ok::<
                            get_data_contract_history_response::DataContractHistoryEntry,
                            ProtocolError,
                        >(
                            get_data_contract_history_response::DataContractHistoryEntry {
                                date: date_in_seconds,
                                value: data_contract
                                    .serialize_to_bytes_with_platform_version(platform_version)?
                            }
                        ))
                        .collect());
                    GetDataContractHistoryResponse {
                        result: Some(
                            get_data_contract_history_response::Result::DataContractHistory(
                                get_data_contract_history_response::DataContractHistory {
                                    data_contract_entries: contract_historical_entries,
                                },
                            ),
                        ),
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
                let contract_id: Identifier = check_validation_result_with_data!(data_contract_id
                    .try_into()
                    .map_err(|_| QueryError::InvalidArgument(
                        "id must be a valid identifier (32 bytes long)".to_string()
                    )));
                let (_, contract) = check_validation_result_with_data!(self
                    .drive
                    .get_contract_with_fetch_info_and_fee(
                        contract_id.to_buffer(),
                        None,
                        true,
                        None,
                        platform_version,
                    ));
                let contract = check_validation_result_with_data!(contract.ok_or(
                    QueryError::Query(QuerySyntaxError::DataContractNotFound(
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
                        "unable to decode 'where' query from cbor".to_string(),
                    ))
                }));

                // TODO: fix?
                //   Fails with "query syntax error: deserialization error: unable to decode 'order_by' query from cbor"
                //   cbor deserialization fails if order_by is empty
                let order_by = if !order_by.is_empty() {
                    check_validation_result_with_data!(ciborium::de::from_reader(
                        order_by.as_slice()
                    )
                    .map_err(|_| {
                        QueryError::Query(QuerySyntaxError::DeserializationError(
                            "unable to decode 'order_by' query from cbor".to_string(),
                        ))
                    }))
                } else {
                    None
                };

                let (start_at_included, start_at) = if let Some(start) = start {
                    match start {
                        Start::StartAfter(after) => (
                            false,
                            Some(check_validation_result_with_data!(after
                                .try_into()
                                .map_err(|_| QueryError::Query(
                                    QuerySyntaxError::InvalidStartsWithClause(
                                        "start after should be a 32 byte identifier",
                                    )
                                )))),
                        ),
                        Start::StartAt(at) => (
                            true,
                            Some(check_validation_result_with_data!(at.try_into().map_err(
                                |_| QueryError::Query(QuerySyntaxError::InvalidStartsWithClause(
                                    "start at should be a 32 byte identifier",
                                ))
                            ))),
                        ),
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
                        Some(if limit == 0 {
                            self.config.drive.default_query_limit
                        } else {
                            limit as u16
                        }),
                        start_at,
                        start_at_included,
                        None,
                        contract_ref,
                        document_type,
                        &self.config.drive,
                    ));
                let response_data = if prove {
                    let (proof, _) = check_validation_result_with_data!(
                        drive_query.execute_with_proof(&self.drive, None, None, platform_version)
                    );
                    GetDocumentsResponse {
                        result: Some(get_documents_response::Result::Proof(Proof {
                            grovedb_proof: proof,
                            quorum_hash: state.last_quorum_hash().to_vec(),
                            quorum_type,
                            block_id_hash: state.last_block_id_hash().to_vec(),
                            signature: state.last_block_signature().to_vec(),
                            round: state.last_block_round(),
                        })),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                } else {
                    let results = check_validation_result_with_data!(drive_query
                        .execute_raw_results_no_proof(&self.drive, None, None, platform_version))
                    .0;
                    GetDocumentsResponse {
                        result: Some(get_documents_response::Result::Documents(
                            get_documents_response::Documents { documents: results },
                        )),
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
                let public_key_hash =
                    check_validation_result_with_data!(Bytes20::from_vec(public_key_hash)
                        .map(|bytes| bytes.0)
                        .map_err(|_| QueryError::InvalidArgument(
                            "public key hash must be 20 bytes long".to_string()
                        )));
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_full_identity_by_unique_public_key_hash(
                            public_key_hash,
                            None,
                            platform_version
                        ));

                    if proof.is_empty() {
                        return Ok(QueryValidationResult::new_with_error(QueryError::NotFound(
                            format!("identity {} not found", hex::encode(public_key_hash)),
                        )));
                    }

                    GetIdentityByPublicKeyHashesResponse {
                        metadata: Some(metadata),
                        result: Some(get_identity_by_public_key_hashes_response::Result::Proof(
                            Proof {
                                grovedb_proof: proof,
                                quorum_hash: state.last_quorum_hash().to_vec(),
                                quorum_type,
                                block_id_hash: state.last_block_id_hash().to_vec(),
                                signature: state.last_block_signature().to_vec(),
                                round: state.last_block_round(),
                            },
                        )),
                    }
                    .encode_to_vec()
                } else {
                    let maybe_identity = check_validation_result_with_data!(self
                        .drive
                        .fetch_full_identity_by_unique_public_key_hash(
                            public_key_hash,
                            None,
                            platform_version
                        ));
                    let serialized_identity = check_validation_result_with_data!(maybe_identity
                        .ok_or_else(|| {
                            QueryError::NotFound(format!(
                                "identity {} not found",
                                hex::encode(public_key_hash)
                            ))
                        })
                        .and_then(|identity| identity
                            .serialize_consume_to_bytes()
                            .map_err(QueryError::Protocol)));

                    GetIdentityByPublicKeyHashesResponse {
                        metadata: Some(metadata),
                        result: Some(
                            get_identity_by_public_key_hashes_response::Result::Identity(
                                serialized_identity,
                            ),
                        ),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
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
                        Bytes20::from_vec(pub_key_hash_vec)
                            .map(|bytes| bytes.0)
                            .map_err(|_| {
                                QueryError::InvalidArgument(
                                    "public key hash must be 20 bytes long".to_string(),
                                )
                            })
                    })
                    .collect::<Result<Vec<[u8; 20]>, QueryError>>());
                let response_data = if prove {
                    let proof = check_validation_result_with_data!(self
                        .drive
                        .prove_full_identities_by_unique_public_key_hashes(
                            &public_key_hashes,
                            None,
                            platform_version,
                        ));
                    GetIdentitiesByPublicKeyHashesResponse {
                        result: Some(get_identities_by_public_key_hashes_response::Result::Proof(
                            Proof {
                                grovedb_proof: proof,
                                quorum_hash: state.last_quorum_hash().to_vec(),
                                quorum_type,
                                block_id_hash: state.last_block_id_hash().to_vec(),
                                signature: state.last_block_signature().to_vec(),
                                round: state.last_block_round(),
                            },
                        )),
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
                            platform_version,
                        ));
                    let identities = check_validation_result_with_data!(identities
                        .into_values()
                        .filter_map(|maybe_identity| Some(
                            maybe_identity?.serialize_consume_to_bytes()
                        ))
                        .collect::<Result<Vec<Vec<u8>>, ProtocolError>>());
                    GetIdentitiesByPublicKeyHashesResponse {
                        result: Some(
                            get_identities_by_public_key_hashes_response::Result::Identities(
                                get_identities_by_public_key_hashes_response::Identities {
                                    identities,
                                },
                            ),
                        ),
                        metadata: Some(metadata),
                    }
                    .encode_to_vec()
                };
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            "/proofs" => {
                let GetProofsRequest {
                    identities,
                    contracts,
                    documents,
                } = check_validation_result_with_data!(GetProofsRequest::decode(query_data));
                let contract_ids = check_validation_result_with_data!(contracts
                    .into_iter()
                    .map(|contract_request| {
                        Bytes32::from_vec(contract_request.contract_id)
                            .map(|bytes| bytes.0)
                            .map_err(|_| {
                                QueryError::InvalidArgument(
                                    "id must be a valid identifier (32 bytes long)".to_string(),
                                )
                            })
                    })
                    .collect::<Result<Vec<[u8; 32]>, QueryError>>());
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
                            )?,
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

                        Ok(SingleDocumentDriveQuery {
                            contract_id: contract_id.into_buffer(),
                            document_type_name: document_proof_request.document_type,
                            document_type_keeps_history: document_proof_request
                                .document_type_keeps_history,
                            document_id: document_id.into_buffer(),
                            block_time_ms: None, //None because we want latest
                        })
                    })
                    .collect::<Result<Vec<_>, QueryError>>());
                let proof = check_validation_result_with_data!(self.drive.prove_multiple(
                    &identity_requests,
                    &contract_ids,
                    &document_queries,
                    None,
                    platform_version,
                ));
                let response_data = GetProofsResponse {
                    proof: Some(Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_block_id_hash().to_vec(),
                        signature: state.last_block_signature().to_vec(),
                        round: state.last_block_round(),
                    }),
                    metadata: Some(metadata),
                }
                .encode_to_vec();
                Ok(QueryValidationResult::new_with_data(response_data))
            }
            other => Ok(QueryValidationResult::new_with_error(
                QueryError::InvalidArgument(format!("query path '{}' is not supported", other)),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    pub mod query_data_contract_history {
        use crate::error::Error;
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
        use dapi_grpc::platform::v0::{
            get_data_contract_history_response, GetDataContractHistoryRequest,
            GetDataContractHistoryResponse,
        };
        use dpp::block::block_info::BlockInfo;

        use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::config::v0::DataContractConfigSettersV0;

        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::data_contract::schema::DataContractSchemaMethodsV0;
        use dpp::data_contract::DataContract;
        use dpp::platform_value::platform_value;
        use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use dpp::validation::ValidationResult;
        use dpp::version::PlatformVersion;
        use drive::drive::Drive;
        use drive::error::contract::DataContractError;
        use prost::Message;

        fn default_request() -> GetDataContractHistoryRequest {
            GetDataContractHistoryRequest {
                id: vec![1; 32],
                limit: Some(10),
                offset: Some(0),
                start_at_ms: 0,
                prove: false,
            }
        }

        /// Set up simple contract history with one update
        fn set_up_history(platform: &TempPlatform<MockCoreRPCLike>) -> DataContract {
            let state = platform.platform.state.read().unwrap();
            let current_protocol_version = state.current_protocol_version_in_consensus();
            let platform_version = PlatformVersion::get(current_protocol_version)
                .expect("expected to get platform version");
            drop(state);
            let mut data_contract =
                get_data_contract_fixture(None, current_protocol_version).data_contract_owned();
            data_contract.config_mut().set_keeps_history(true);
            data_contract.config_mut().set_readonly(false);

            platform
                .drive
                .apply_contract(
                    &data_contract,
                    BlockInfo {
                        time_ms: 1000,
                        height: 10,
                        core_height: 20,
                        epoch: Default::default(),
                    },
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("To apply contract");

            let mut updated_data_contract = data_contract.clone();

            let updated_document_schema = platform_value!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string"
                    },
                    "newProp": {
                        "type": "integer",
                        "minimum": 0
                    }
                },
                "required": [
                "$createdAt"
                ],
                "additionalProperties": false
            });

            updated_data_contract
                .set_document_schema(
                    "niceDocument",
                    updated_document_schema,
                    true,
                    platform_version,
                )
                .expect("to be able to set document schema");

            platform
                .drive
                .apply_contract(
                    &updated_data_contract,
                    BlockInfo {
                        time_ms: 2000,
                        height: 11,
                        core_height: 21,
                        epoch: Default::default(),
                    },
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("To apply contract");

            data_contract
        }

        struct TestData {
            platform: TempPlatform<MockCoreRPCLike>,
            original_data_contract: DataContract,
        }

        fn set_up_test() -> TestData {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let original_data_contract = set_up_history(&platform);

            TestData {
                platform,
                original_data_contract,
            }
        }

        #[test]
        pub fn should_return_contract_history_with_no_errors_if_parameters_are_valid() {
            let platform_version = PlatformVersion::latest();

            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                id: original_data_contract.id().to_vec(),
                ..default_request()
            };

            let request_data = request.encode_to_vec();

            let result = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .expect("To return result");

            let ValidationResult { errors, data } = result;

            assert!(
                errors.is_empty(),
                "expect no errors to be returned from the query"
            );

            let data = data.expect("expect data to be returned from the query");
            let _data_ref = data.as_slice();

            let response = GetDataContractHistoryResponse::decode(data.as_slice())
                .expect("To decode response");

            let GetDataContractHistoryResponse {
                metadata: _,
                result,
            } = response;
            let res = result.expect("expect result to be returned from the query");

            let contract_history = match res {
                get_data_contract_history_response::Result::DataContractHistory(
                    data_contract_history,
                ) => data_contract_history,
                get_data_contract_history_response::Result::Proof(_) => {
                    panic!("expect result to be DataContractHistory");
                }
            };

            let mut history_entries = contract_history.data_contract_entries;
            assert_eq!(history_entries.len(), 2);

            let second_entry = history_entries.pop().unwrap();
            let first_entry = history_entries.pop().unwrap();

            assert_eq!(first_entry.date, 1000);
            let first_entry_data_contract = first_entry.value;
            let first_data_contract_update = DataContract::versioned_deserialize(
                &first_entry_data_contract,
                true,
                platform_version,
            )
            .expect("To decode data contract");
            assert_eq!(first_data_contract_update, original_data_contract);

            assert_eq!(second_entry.date, 2000);

            let second_entry_data_contract = second_entry.value;

            let second_data_contract_update = DataContract::versioned_deserialize(
                &second_entry_data_contract,
                true,
                platform_version,
            )
            .expect("To decode data contract");

            let updated_doc = second_data_contract_update
                .document_type_for_name("niceDocument")
                .expect("should return document type");

            assert!(
                updated_doc.properties().contains_key("newProp"),
                "expect data contract to have newProp field",
            );
        }

        #[test]
        pub fn should_return_contract_history_proofs_with_no_errors_if_parameters_are_valid() {
            let platform_version = PlatformVersion::latest();
            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                id: original_data_contract.id().to_vec(),
                prove: true,
                ..default_request()
            };
            let request_data = request.encode_to_vec();

            let result = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .expect("To return result");

            let ValidationResult { errors, data } = result;

            assert!(
                errors.is_empty(),
                "expect no errors to be returned from the query"
            );

            let data = data.expect("expect data to be returned from the query");
            let _data_ref = data.as_slice();

            let response = GetDataContractHistoryResponse::decode(data.as_slice())
                .expect("To decode response");

            let GetDataContractHistoryResponse {
                metadata: _,
                result,
            } = response;
            let res = result.expect("expect result to be returned from the query");

            let contract_proof = match res {
                get_data_contract_history_response::Result::DataContractHistory(_) => {
                    panic!("expect result to be Proof");
                }
                get_data_contract_history_response::Result::Proof(proof) => proof,
            };

            // Check that the proof has correct values inside
            let (_root_hash, contract_history) = Drive::verify_contract_history(
                &contract_proof.grovedb_proof,
                original_data_contract.id().to_buffer(),
                request.start_at_ms,
                Some(10),
                Some(0),
                platform_version,
            )
            .expect("To verify contract history");

            let mut history_entries = contract_history.expect("history to exist");

            assert_eq!(history_entries.len(), 2);

            // Taking entries by date
            let first_data_contract_update =
                history_entries.remove(&1000).expect("first entry to exist");
            let second_data_contract_update = history_entries
                .remove(&2000)
                .expect("second entry to exist");

            assert_eq!(first_data_contract_update, original_data_contract);

            let updated_doc = second_data_contract_update
                .document_type_for_name("niceDocument")
                .expect("To have niceDocument document");

            assert!(
                updated_doc.properties().contains_key("newProp"),
                "expect data contract to have newProp field",
            );
        }

        #[test]
        pub fn should_return_error_when_limit_is_larger_than_u16() {
            let platform_version = PlatformVersion::latest();
            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                id: original_data_contract.id().to_vec(),
                limit: Some(100000),
                ..default_request()
            };
            let request_data = request.encode_to_vec();

            let error = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .unwrap_err();

            match error {
                Error::Drive(drive_error) => match drive_error {
                    drive::error::Error::DataContract(contract_error) => match contract_error {
                        DataContractError::Overflow(error_message) => {
                            assert_eq!(
                                error_message,
                                "can't fit u16 limit from the supplied value"
                            );
                        }
                        _ => panic!("expect contract overflow error"),
                    },
                    _ => panic!("expect contract error"),
                },
                _ => panic!("expect drive error"),
            }
        }

        #[test]
        pub fn should_return_error_when_offset_is_larger_than_u16() {
            let platform_version = PlatformVersion::latest();

            let TestData {
                platform,
                original_data_contract,
            } = set_up_test();

            let request = GetDataContractHistoryRequest {
                id: original_data_contract.id().to_vec(),
                offset: Some(100000),
                ..default_request()
            };
            let request_data = request.encode_to_vec();

            let error = platform
                .query_v0("/dataContractHistory", &request_data, platform_version)
                .unwrap_err();

            match error {
                Error::Drive(drive_error) => match drive_error {
                    drive::error::Error::DataContract(contract_error) => match contract_error {
                        DataContractError::Overflow(error_message) => {
                            assert_eq!(
                                error_message,
                                "can't fit u16 offset from the supplied value"
                            );
                        }
                        _ => panic!("expect contract overflow error"),
                    },
                    _ => panic!("expect contract error"),
                },
                _ => panic!("expect drive error"),
            }
        }
    }
}
