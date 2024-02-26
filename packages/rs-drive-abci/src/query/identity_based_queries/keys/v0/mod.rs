use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_response::{
    get_identity_keys_response_v0, GetIdentityKeysResponseV0,
};
use dpp::check_validation_result_with_data;
use dpp::identifier::Identifier;
use drive::error::query::QuerySyntaxError;
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
        GetIdentityKeysRequestV0 {
            identity_id,
            request_type,
            limit,
            offset,
            prove,
        }: GetIdentityKeysRequestV0,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentityKeysResponseV0>, Error> {
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

        let response = if prove {
            let proof = self
                .drive
                .prove_identity_keys(key_request, None, platform_version)?;

            let (metadata, proof) = self.response_metadata_and_proof_v0(proof);

            GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Proof(proof)),
                metadata: Some(metadata),
            }
        } else {
            let keys: SerializedKeyVec =
                self.drive
                    .fetch_identity_keys(key_request, None, platform_version)?;

            GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Keys(
                    get_identity_keys_response_v0::Keys { keys_bytes: keys },
                )),
                metadata: Some(self.response_metadata_v0()),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};
    use dapi_grpc::platform::v0::key_request_type::Request;
    use dapi_grpc::platform::v0::{AllKeys, KeyRequestType, SearchKey, SecurityLevelMap};

    #[test]
    fn test_invalid_identity_id() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 8],
            request_type: None,
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_limit_u16_overflow() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: Some(u32::MAX),
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "limit out of bounds"
        ));
    }

    #[test]
    fn test_invalid_limit_max() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: Some((platform.config.drive.max_query_limit + 1) as u32),
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        let error_message = format!(
            "limit greater than max limit {}",
            platform.config.drive.max_query_limit
        );

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidLimit(msg))] if msg == &error_message
        ));
    }

    #[test]
    fn test_invalid_offset_u16_overflow() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: None,
            offset: Some(u32::MAX),
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "offset out of bounds"
        ));
    }

    #[test]
    fn test_missing_request_type() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "key request must be defined"
        ));
    }

    #[test]
    fn test_missing_request() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType { request: None }),
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "key request must be defined"
        ));
    }

    #[test]
    fn test_invalid_key_request_type() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SearchKey(SearchKey {
                    purpose_map: [(
                        0,
                        SecurityLevelMap {
                            security_level_map: [(u32::MAX, 0)].into_iter().collect(),
                        },
                    )]
                    .into_iter()
                    .collect(),
                })),
            }),
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg))] if msg == "security level out of bounds"
        ));
    }

    #[test]
    fn test_absent_keys() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::AllKeys(AllKeys {})),
            }),
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Keys(keys)),
                ..
            }) if keys.keys_bytes.is_empty()
        ));
    }

    #[test]
    fn test_absent_keys_proof() {
        let (platform, version) = setup_platform();

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::AllKeys(AllKeys {})),
            }),
            limit: None,
            offset: None,
            prove: true,
        };

        let result = platform
            .query_keys_v0(request, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ));
    }
}
