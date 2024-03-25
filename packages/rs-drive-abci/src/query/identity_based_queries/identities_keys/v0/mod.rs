
use std::collections::HashMap;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_identities_keys_request::GetIdentitiesKeysRequestV0;
use dapi_grpc::platform::v0::get_identities_keys_response::{get_identities_keys_response_v0, GetIdentitiesKeysResponseV0};
use dpp::platform_value::Bytes32;
use dpp::serialization::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use dpp::{check_validation_result_with_data, ProtocolError};

impl<C> Platform<C> {
    pub(super) fn query_identities_v0(
        &self,
        GetIdentitiesKeysRequestV0 { ids, prove }: GetIdentitiesKeysRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetIdentitiesKeysResponseV0>, Error> {
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

        let response = if prove {
            todo!()
            // let proof = self.drive.prove_full_identities(
            //     identity_ids.as_slice(),
            //     None,
            //     &platform_version.drive,
            // )?;
            //
            // GetPartialIdentitiesResponseV0 {
            //     result: Some(get_partial_identities_response_v0::Result::Proof(
            //         self.response_proof_v0(platform_state, proof),
            //     )),
            //     metadata: Some(self.response_metadata_v0(platform_state)),
            // }
        } else {
            let identities_keys = self.drive.fetch_identities_keys(
                identity_ids.as_slice(),
                None,
                platform_version,
            )?;

            use get_identities_keys_response_v0::Keys;

            let identities_keys = identities_keys
                .into_iter()
                .map(|(key, identity_keys)| {
                    let keys_bytes = identity_keys
                        .into_iter()
                        .map(|(_, key)| key.serialize_to_bytes())
                        .collect::<Result<Vec<Vec<u8>>, ProtocolError>>()?;
                    
                    Ok((hex::encode(&key), Keys { keys_bytes }))
                })
                .collect::<Result<HashMap<String, Keys>, ProtocolError>>()?;

            GetIdentitiesKeysResponseV0 {
                result: Some(
                    get_identities_keys_response_v0::Result::IdentitiesKeys(
                        get_identities_keys_response_v0::IdentitiesKeys {
                            entries: identities_keys
                        }
                    )
                ),
                metadata: Some(self.response_metadata_v0(platform_state)),
            }
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::{assert_invalid_identifier, setup_platform};

    #[test]
    fn test_invalid_identity_id() {
        let (platform, state, version) = setup_platform();

        let request = GetIdentitiesRequestV0 {
            ids: vec![vec![0; 8]],
            prove: false,
        };

        let result = platform
            .query_identities_v0(request, &state, version)
            .expect("should query identities");

        assert_invalid_identifier(result);
    }

    #[test]
    fn test_identities_not_found() {
        let (platform, state, version) = setup_platform();

        let id = vec![0; 32];

        let request = GetIdentitiesRequestV0 {
            ids: vec![id.clone()],
            prove: false,
        };

        let result = platform
            .query_identities_v0(request, &state, version)
            .expect("should query identities");

        assert!(matches!(result.data, Some(GetIdentitiesResponseV0 {
            result: Some(get_identities_response_v0::Result::Identities(identites)),
            metadata: Some(_)
        }) if identites.identity_entries.len() == 1 && identites.identity_entries[0].value.is_none()));
    }

    #[test]
    fn test_identities_absence_proof() {
        let (platform, state, version) = setup_platform();

        let id = vec![0; 32];
        let request = GetIdentitiesRequestV0 {
            ids: vec![id.clone()],
            prove: true,
        };

        let result = platform
            .query_identities_v0(request, &state, version)
            .expect("should query identities");

        assert!(matches!(
            result.data,
            Some(GetIdentitiesResponseV0 {
                result: Some(get_identities_response_v0::Result::Proof(_)),
                metadata: Some(_)
            })
        ));
    }
}
