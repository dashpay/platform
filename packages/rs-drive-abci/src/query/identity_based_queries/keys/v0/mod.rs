use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_response::GetIdentityKeysResponseV0;
use dapi_grpc::platform::v0::{get_identity_keys_response, GetIdentityKeysResponse, Proof};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use drive::error::query::QuerySyntaxError;
use prost::Message;
use std::collections::BTreeMap;

use dpp::identity::{KeyID, Purpose, SecurityLevel};
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
    SerializedKeyVec,
};

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
    pub(super) fn query_keys_v0(
        &self,
        state: &PlatformState,
        request: GetIdentityKeysRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let metadata = self.response_metadata_v0(state);
        let quorum_type = self.config.validator_set_quorum_type() as u32;
        let GetIdentityKeysRequestV0 {
            identity_id,
            request_type,
            limit,
            offset,
            prove,
        } = request;
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
                    QuerySyntaxError::InvalidParameter("offset out of bounds".to_string()),
                )));
            }
        }
        let Some(request_type) = request_type else {
            return Ok(QueryValidationResult::new_with_error(QueryError::Query(
                QuerySyntaxError::InvalidParameter("key request must be defined".to_string()),
            )));
        };
        let Some(request) = request_type.request else {
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
            let proof = self
                .drive
                .prove_identity_keys(key_request, None, platform_version)?;

            GetIdentityKeysResponse {
                version: Some(get_identity_keys_response::Version::V0(GetIdentityKeysResponseV0 {
                    result: Some(get_identity_keys_response::get_identity_keys_response_v0::Result::Proof(Proof {
                        grovedb_proof: proof,
                        quorum_hash: state.last_committed_quorum_hash().to_vec(),
                        quorum_type,
                        block_id_hash: state.last_committed_block_id_hash().to_vec(),
                        signature: state.last_committed_block_signature().to_vec(),
                        round: state.last_committed_block_round(),
                    })),
                    metadata: Some(metadata),
                })),
            }
                .encode_to_vec()
        } else {
            let keys: SerializedKeyVec =
                self.drive
                    .fetch_identity_keys(key_request, None, platform_version)?;

            GetIdentityKeysResponse {
                version: Some(get_identity_keys_response::Version::V0(
                    GetIdentityKeysResponseV0 {
                        result: Some(
                            get_identity_keys_response::get_identity_keys_response_v0::Result::Keys(
                                get_identity_keys_response::get_identity_keys_response_v0::Keys {
                                    keys_bytes: keys,
                                },
                            ),
                        ),
                        metadata: Some(metadata),
                    },
                )),
            }
            .encode_to_vec()
        };
        Ok(QueryValidationResult::new_with_data(response_data))
    }
}
