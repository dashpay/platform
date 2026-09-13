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
use std::collections::{BTreeMap, BTreeSet};

use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use dpp::identity::{Purpose, SecurityLevel};
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
    SerializedKeyVec,
};
use drive::util::grove_operations::GroveDBToUse;

fn from_i32_to_key_kind_request_type(value: i32) -> Option<KeyKindRequestType> {
    match value {
        0 => Some(KeyKindRequestType::CurrentKeyOfKindRequest),
        1 => Some(KeyKindRequestType::AllKeysOfKindRequest),
        _ => None,
    }
}

/// Converts the wire key request into a Drive key request.
///
/// `max_specific_key_ids` bounds a `SpecificKeys` request: every distinct id
/// becomes one GroveDB key item, and an id the identity does not have still
/// costs an absence proof, so the response `limit` alone does not bound the
/// work. Duplicates are collapsed before the bound is applied so that the
/// bound measures the work the node would actually do, and the collection
/// stops as soon as the bound is exceeded so a rejected request never
/// materializes its whole distinct set.
fn convert_key_request_type(
    request_type: dapi_grpc::platform::v0::key_request_type::Request,
    max_specific_key_ids: u16,
) -> Result<KeyRequestType, QueryError> {
    match request_type {
        dapi_grpc::platform::v0::key_request_type::Request::AllKeys(_) => {
            Ok(KeyRequestType::AllKeys)
        }
        dapi_grpc::platform::v0::key_request_type::Request::SpecificKeys(specific_keys) => {
            let mut key_ids = BTreeSet::new();
            for key_id in specific_keys.key_ids {
                key_ids.insert(key_id);
                if key_ids.len() > max_specific_key_ids as usize {
                    return Err(QueryError::TooManyElements(format!(
                        "trying to get more than {} specific keys",
                        max_specific_key_ids
                    )));
                }
            }
            Ok(KeyRequestType::SpecificKeys(key_ids.into_iter().collect()))
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
        platform_state: &PlatformState,
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

        let key_request_type = check_validation_result_with_data!(convert_key_request_type(
            request,
            platform_version.drive_abci.query.max_returned_elements
        ));

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

            GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            // The non-proof specific-keys fetch needs a limit. Default it to the
            // number of distinct ids, which the bound above already caps, so a
            // request that omits it behaves like the proof path instead of
            // failing inside Drive. The proof path is left as requested so the
            // proof matches what the client verifies against.
            let mut key_request = key_request;
            if key_request.limit.is_none() {
                if let KeyRequestType::SpecificKeys(key_ids) = &key_request.request_type {
                    key_request.limit = key_ids.len().try_into().ok();
                }
            }

            let keys: SerializedKeyVec =
                self.drive
                    .fetch_identity_keys(key_request, None, platform_version)?;

            GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Keys(
                    get_identity_keys_response_v0::Keys { keys_bytes: keys },
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
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
    use dpp::dashcore::Network;

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 8],
            request_type: None,
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_invalid_limit_u16_overflow() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: Some(u32::MAX),
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "limit out of bounds"
        ));
    }

    #[test]
    fn test_invalid_limit_max() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: Some((platform.config.drive.max_query_limit + 1) as u32),
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
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
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: None,
            offset: Some(u32::MAX),
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "offset out of bounds"
        ));
    }

    #[test]
    fn test_missing_request_type() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: None,
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "key request must be defined"
        ));
    }

    #[test]
    fn test_missing_request() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType { request: None }),
            limit: None,
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidParameter(msg))] if msg == "key request must be defined"
        ));
    }

    #[test]
    fn test_invalid_key_request_type() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

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
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg))] if msg == "security level out of bounds"
        ));
    }

    #[test]
    fn test_absent_keys() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

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
            .query_keys_v0(request, &state, version)
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
    fn test_invalid_security_level() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Security level 200 is invalid (not a valid u8 SecurityLevel enum value)
        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SearchKey(SearchKey {
                    purpose_map: [(
                        0,
                        SecurityLevelMap {
                            security_level_map: [(200, 0)].into_iter().collect(),
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
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg))] if msg.contains("security level")
        ));
    }

    #[test]
    fn test_invalid_purpose() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Purpose 200 is invalid
        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SearchKey(SearchKey {
                    purpose_map: [(
                        200,
                        SecurityLevelMap {
                            security_level_map: [(0, 0)].into_iter().collect(),
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
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg))] if msg.contains("purpose")
        ));
    }

    #[test]
    fn test_invalid_key_kind_request_type() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Key kind request type 99 is invalid (only 0 and 1 are valid)
        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SearchKey(SearchKey {
                    purpose_map: [(
                        0,
                        SecurityLevelMap {
                            security_level_map: [(0, 99)].into_iter().collect(),
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
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg))] if msg.contains("unknown key kind request type")
        ));
    }

    #[test]
    fn test_purpose_out_of_bounds() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Purpose > u8::MAX
        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SearchKey(SearchKey {
                    purpose_map: [(
                        256, // u8::MAX + 1
                        SecurityLevelMap {
                            security_level_map: [(0, 0)].into_iter().collect(),
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
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(QuerySyntaxError::InvalidKeyParameter(msg))] if msg.contains("purpose out of bounds")
        ));
    }

    #[test]
    fn test_specific_keys_request_returns_empty_for_absent_identity() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SpecificKeys(
                    dapi_grpc::platform::v0::SpecificKeys {
                        key_ids: vec![0, 1, 2],
                    },
                )),
            }),
            limit: Some(100),
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
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
    fn test_search_key_request_returns_empty_for_absent_identity() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::SearchKey(SearchKey {
                    purpose_map: [(
                        0, // AUTHENTICATION
                        SecurityLevelMap {
                            security_level_map: [(0, 0)].into_iter().collect(), // MASTER, CurrentKeyOfKindRequest
                        },
                    )]
                    .into_iter()
                    .collect(),
                })),
            }),
            limit: Some(100),
            offset: None,
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
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
    fn test_query_with_valid_limit_and_offset() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetIdentityKeysRequestV0 {
            identity_id: vec![0; 32],
            request_type: Some(KeyRequestType {
                request: Some(Request::AllKeys(AllKeys {})),
            }),
            limit: Some(10),
            offset: Some(0),
            prove: false,
        };

        let result = platform
            .query_keys_v0(request, &state, version)
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
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

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
            .query_keys_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ));
    }

    mod specific_keys_bounds {
        use super::*;
        use crate::rpc::core::MockCoreRPCLike;
        use dapi_grpc::platform::v0::SpecificKeys;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::{Identity, IdentityPublicKey, KeyID};
        use dpp::serialization::PlatformDeserializable;
        use drive::drive::identity::key::fetch::KeyRequestType as DriveKeyRequestType;
        use drive::drive::Drive;
        use drive::grovedb::GroveDb;

        /// Stores an identity with `key_count` keys (ids `0..key_count`) and returns it.
        fn seed_identity(
            platform: &Platform<MockCoreRPCLike>,
            key_count: u32,
            seed: u64,
            platform_version: &PlatformVersion,
        ) -> Identity {
            let identity = Identity::random_identity(key_count, Some(seed), platform_version)
                .expect("expected a random identity");

            platform
                .drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            identity
        }

        fn specific_keys_request(
            identity_id: Vec<u8>,
            key_ids: Vec<u32>,
            prove: bool,
        ) -> GetIdentityKeysRequestV0 {
            GetIdentityKeysRequestV0 {
                identity_id,
                request_type: Some(KeyRequestType {
                    request: Some(Request::SpecificKeys(SpecificKeys { key_ids })),
                }),
                limit: None,
                offset: None,
                prove,
            }
        }

        /// Keys of a non-proof response, deserialized and keyed by id.
        fn fetched_keys(
            result: QueryValidationResult<GetIdentityKeysResponseV0>,
        ) -> BTreeMap<KeyID, IdentityPublicKey> {
            let Some(GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Keys(keys)),
                ..
            }) = result.data
            else {
                panic!("expected keys, got errors {:?}", result.errors);
            };

            keys.keys_bytes
                .into_iter()
                .map(|bytes| {
                    let key = IdentityPublicKey::deserialize_from_bytes(&bytes)
                        .expect("expected a serialized identity public key");
                    (key.id(), key)
                })
                .collect()
        }

        fn proof_bytes(result: QueryValidationResult<GetIdentityKeysResponseV0>) -> Vec<u8> {
            let Some(GetIdentityKeysResponseV0 {
                result: Some(get_identity_keys_response_v0::Result::Proof(proof)),
                ..
            }) = result.data
            else {
                panic!("expected a proof, got errors {:?}", result.errors);
            };

            proof.grovedb_proof
        }

        /// Verifies a specific-keys proof at the GroveDB level against the raw
        /// (possibly duplicated) id list and returns the proved keys by id.
        /// GroveDB proves the requested ids that do not exist implicitly, so
        /// they do not show up in the result. The Drive key verifier is not
        /// used here because it rejects proofs that carry absent keys.
        fn verify_specific_keys_proof(
            proof: &[u8],
            identity_id: [u8; 32],
            key_ids: Vec<KeyID>,
            platform_version: &PlatformVersion,
        ) -> BTreeMap<KeyID, IdentityPublicKey> {
            let path_query = IdentityKeysRequest {
                identity_id,
                request_type: DriveKeyRequestType::SpecificKeys(key_ids),
                limit: None,
                offset: None,
            }
            .into_path_query();

            let (_, proved_values) =
                GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                    .expect("proof should verify");

            proved_values
                .into_iter()
                .filter_map(|(_, _, maybe_element)| maybe_element)
                .map(|element| {
                    let bytes = element
                        .into_item_bytes()
                        .expect("key element should be an item");
                    let key = IdentityPublicKey::deserialize_from_bytes(&bytes)
                        .expect("expected a serialized identity public key");
                    (key.id(), key)
                })
                .collect()
        }

        #[test]
        fn test_oversized_distinct_list_is_rejected_with_and_without_proof() {
            let (platform, state, version) = setup_platform(None, Network::Testnet, None);
            let max = version.drive_abci.query.max_returned_elements;

            // One more distinct id than the bound allows.
            let key_ids: Vec<u32> = (0..=max as u32).collect();
            let expected_message = format!("trying to get more than {} specific keys", max);

            for prove in [false, true] {
                let result = platform
                    .query_keys_v0(
                        specific_keys_request(vec![0; 32], key_ids.clone(), prove),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed");

                assert!(result.data.is_none(), "prove={prove}: no data expected");
                assert!(
                    matches!(
                        result.errors.as_slice(),
                        [QueryError::TooManyElements(msg)] if msg == &expected_message
                    ),
                    "prove={prove}: unexpected errors {:?}",
                    result.errors
                );
            }
        }

        #[test]
        fn test_boundary_sized_list_returns_existing_keys_with_and_without_proof() {
            let (platform, state, version) = setup_platform(None, Network::Testnet, None);
            let identity = seed_identity(&platform, 5, 44444, version);
            let identity_id = identity.id().to_buffer();
            let max = version.drive_abci.query.max_returned_elements;

            // Exactly the bound, of which only ids 0..5 exist.
            let key_ids: Vec<u32> = (0..max as u32).collect();

            let fetched = fetched_keys(
                platform
                    .query_keys_v0(
                        specific_keys_request(identity_id.to_vec(), key_ids.clone(), false),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed"),
            );
            assert_eq!(&fetched, identity.public_keys());

            let proof = proof_bytes(
                platform
                    .query_keys_v0(
                        specific_keys_request(identity_id.to_vec(), key_ids.clone(), true),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed"),
            );
            let proved = verify_specific_keys_proof(&proof, identity_id, key_ids, version);
            assert_eq!(proved, fetched);
        }

        #[test]
        fn test_duplicate_heavy_list_is_deduplicated_before_the_bound() {
            let (platform, state, version) = setup_platform(None, Network::Testnet, None);
            let identity = seed_identity(&platform, 5, 55555, version);
            let identity_id = identity.id().to_buffer();
            let max = version.drive_abci.query.max_returned_elements as usize;

            // Far more entries than the bound, but only two distinct ids.
            let key_ids: Vec<u32> = [1, 0].into_iter().cycle().take(max * 50).collect();
            assert!(key_ids.len() > max);

            let fetched = fetched_keys(
                platform
                    .query_keys_v0(
                        specific_keys_request(identity_id.to_vec(), key_ids.clone(), false),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed"),
            );
            assert_eq!(fetched.keys().copied().collect::<Vec<_>>(), vec![0, 1]);

            let proof = proof_bytes(
                platform
                    .query_keys_v0(
                        specific_keys_request(identity_id.to_vec(), key_ids.clone(), true),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed"),
            );

            // The Drive verifier collapses duplicates the same way, so the raw
            // list verifies the proof the node built from the deduplicated one.
            let key_request = IdentityKeysRequest {
                identity_id,
                request_type: DriveKeyRequestType::SpecificKeys(key_ids),
                limit: None,
                offset: None,
            };
            let (_, partial_identity) = Drive::verify_identity_keys_by_identity_id(
                &proof,
                key_request,
                false,
                false,
                false,
                version,
            )
            .expect("proof should verify");
            let proved = partial_identity
                .expect("expected a partial identity")
                .loaded_public_keys;
            assert_eq!(proved, fetched);
        }

        #[test]
        fn test_nonexistent_ids_at_the_bound_return_nothing_with_and_without_proof() {
            let (platform, state, version) = setup_platform(None, Network::Testnet, None);
            let identity = seed_identity(&platform, 5, 66666, version);
            let identity_id = identity.id().to_buffer();
            let max = version.drive_abci.query.max_returned_elements as u32;

            // A full-size list of ids the identity does not have.
            let key_ids: Vec<u32> = (1_000..1_000 + max).collect();

            let fetched = fetched_keys(
                platform
                    .query_keys_v0(
                        specific_keys_request(identity_id.to_vec(), key_ids.clone(), false),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed"),
            );
            assert!(fetched.is_empty());

            let proof = proof_bytes(
                platform
                    .query_keys_v0(
                        specific_keys_request(identity_id.to_vec(), key_ids.clone(), true),
                        &state,
                        version,
                    )
                    .expect("expected query to succeed"),
            );
            let proved = verify_specific_keys_proof(&proof, identity_id, key_ids, version);
            assert_eq!(proved, fetched);
        }
    }
}
