use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_path_elements_request::GetPathElementsRequestV0;
use dapi_grpc::platform::v0::get_path_elements_response::get_path_elements_response_v0::Elements;
use dapi_grpc::platform::v0::get_path_elements_response::{
    get_path_elements_response_v0, GetPathElementsResponseV0,
};
use dpp::check_validation_result_with_data;

use crate::error::query::QueryError;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::util::grove_operations::GroveDBToUse;

const MAX_PATH_COMPONENTS: usize = 256;
const MAX_GROVEDB_KEY_BYTES: usize = 255;
const MAX_PATH_QUERY_BYTES: usize = 64 * 1024;

fn validate_path_elements_input(
    path: &[Vec<u8>],
    keys: &[Vec<u8>],
    max_keys: usize,
) -> Result<(), QueryError> {
    if keys.len() > max_keys {
        return Err(QueryError::Query(QuerySyntaxError::InvalidLimit(format!(
            "trying to get {} values, maximum is {}",
            keys.len(),
            max_keys
        ))));
    }

    if path.len() > MAX_PATH_COMPONENTS {
        return Err(QueryError::InvalidArgument(format!(
            "path has {} components, maximum is {}",
            path.len(),
            MAX_PATH_COMPONENTS
        )));
    }

    let total_bytes = path
        .iter()
        .chain(keys)
        .try_fold(0usize, |total, component| {
            if component.len() > MAX_GROVEDB_KEY_BYTES {
                return Err(QueryError::InvalidArgument(format!(
                    "path or key component has {} bytes, maximum is {}",
                    component.len(),
                    MAX_GROVEDB_KEY_BYTES
                )));
            }

            total.checked_add(component.len()).ok_or_else(|| {
                QueryError::InvalidArgument("path query byte count overflow".to_string())
            })
        })?;

    if total_bytes > MAX_PATH_QUERY_BYTES {
        return Err(QueryError::InvalidArgument(format!(
            "path query has {} component bytes, maximum is {}",
            total_bytes, MAX_PATH_QUERY_BYTES
        )));
    }

    Ok(())
}

impl<C> Platform<C> {
    // Security note: this endpoint intentionally allows querying ANY path in GroveDB.
    // All platform state is public and authenticated -- every element has a Merkle
    // proof back to the state root signed by the validator quorum. There is no
    // concept of "private" vs "public" paths. Restricting paths here would not
    // improve security since the same data is accessible through other query
    // endpoints or by running a full node.
    pub(super) fn query_path_elements_v0(
        &self,
        GetPathElementsRequestV0 { path, keys, prove }: GetPathElementsRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetPathElementsResponseV0>, Error> {
        if let Err(error) = validate_path_elements_input(
            &path,
            &keys,
            platform_version.drive_abci.query.max_returned_elements as usize,
        ) {
            return Ok(QueryValidationResult::new_with_error(error));
        }
        let response = if prove {
            let proof = check_validation_result_with_data!(self.drive.prove_elements(
                path,
                keys,
                None,
                platform_version
            ));

            GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let result = check_validation_result_with_data!(self.drive.fetch_elements(
                path,
                keys,
                None,
                platform_version
            ));

            let serialized = result
                .into_iter()
                .map(|element| {
                    element
                        .serialize(&platform_version.drive.grove_version)
                        .map_err(|e| Error::Drive(drive::error::Error::from(e)))
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?;

            GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Elements(Elements {
                    elements: serialized,
                })),
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
    use drive::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
    use drive::drive::RootTree;
    use drive::grovedb::Element;
    use integer_encoding::VarInt;

    #[test]
    fn test_query_total_system_credits_from_path_elements_query() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to insert identity");

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys: vec![TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec()],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_path_elements_response_v0::Result::Elements(mut elements) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected elements")
        };

        assert_eq!(elements.elements.len(), 1);

        let element = Element::deserialize(
            elements.elements.remove(0).as_slice(),
            &platform_version.drive.grove_version,
        )
        .expect("expected to deserialize element");

        let Element::Item(value, _) = element else {
            panic!("expected item")
        };

        let (amount, _) = u64::decode_var(value.as_slice()).expect("expected amount");

        assert_eq!(amount, 100);
    }

    #[test]
    fn test_query_path_elements_exceeds_max_returned_elements() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();
        let max = platform_version.drive_abci.query.max_returned_elements as usize;

        // Build a keys list of length max+1 (trivial bytes, content irrelevant)
        let keys: Vec<Vec<u8>> = (0..(max + 1)).map(|i| vec![i as u8]).collect();

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys,
            prove: false,
        };

        let result = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.errors.as_slice(),
            [QueryError::Query(
                drive::error::query::QuerySyntaxError::InvalidLimit(_)
            )]
        ));
    }

    #[test]
    fn test_query_path_elements_enforces_path_and_component_byte_budgets() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let invalid_requests = [
            GetPathElementsRequestV0 {
                path: vec![vec![]; MAX_PATH_COMPONENTS + 1],
                keys: vec![],
                prove: false,
            },
            GetPathElementsRequestV0 {
                path: vec![vec![0; MAX_GROVEDB_KEY_BYTES + 1]],
                keys: vec![],
                prove: true,
            },
            GetPathElementsRequestV0 {
                path: vec![vec![0; MAX_GROVEDB_KEY_BYTES]; MAX_PATH_COMPONENTS],
                keys: vec![vec![0; 255], vec![1; 255]],
                prove: false,
            },
        ];

        for request in invalid_requests {
            let result = platform
                .query_path_elements_v0(request, &state, version)
                .expect("expected validation to succeed");
            assert!(matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(_)]
            ));
            assert!(result.data.is_none());
        }
    }

    #[test]
    fn test_query_path_elements_proof_branch() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_to_system_credits(100, None, platform_version)
            .expect("expected to add credits");

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys: vec![TOTAL_SYSTEM_CREDITS_STORAGE_KEY.to_vec()],
            prove: true,
        };

        let result = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        assert!(matches!(
            result.data,
            Some(GetPathElementsResponseV0 {
                result: Some(get_path_elements_response_v0::Result::Proof(_)),
                metadata: Some(_),
            })
        ));
    }

    #[test]
    fn test_query_path_elements_missing_key_returns_no_entries() {
        // An existing path with a key that is not present should produce a
        // valid Elements response. The fetch skips absent keys, so the
        // returned list is empty.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys: vec![b"nonexistent-key".to_vec()],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_path_elements_response_v0::Result::Elements(elements) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected elements")
        };

        // Missing keys produce no entries.
        assert!(elements.elements.is_empty());
    }

    #[test]
    fn test_query_path_elements_empty_keys() {
        // Passing an empty `keys` vec must succeed with an empty Elements list.
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let request = GetPathElementsRequestV0 {
            path: vec![vec![RootTree::Misc as u8]],
            keys: vec![],
            prove: false,
        };

        let response = platform
            .query_path_elements_v0(request, &state, version)
            .expect("expected query to succeed");

        let response_data = response.into_data().expect("expected data");

        let get_path_elements_response_v0::Result::Elements(elements) =
            response_data.result.expect("expected a result")
        else {
            panic!("expected elements")
        };

        assert!(elements.elements.is_empty());
    }
}
